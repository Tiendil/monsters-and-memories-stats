//! Local browser/build integration tests, enabled by bin/test-browser.sh.
//! ChromeDriver speaks WebDriver over loopback; no JavaScript test application
//! or public metrics service is involved.

use reqwest::{Method, blocking::Client};
use serde_json::{Value, json};
use std::{
    env, fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_until(mut ready: impl FnMut() -> bool, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ready() {
        assert!(Instant::now() < deadline, "timed out: {description}");
        thread::sleep(Duration::from_millis(50));
    }
}

struct Browser {
    client: Client,
    endpoint: String,
    session: String,
    _driver: Process,
}

impl Browser {
    fn new(scratch: &Path) -> Self {
        let driver_port = port();
        let log = fs::File::create(scratch.join("chromedriver.log")).unwrap();
        let driver = Process(
            Command::new(env::var_os("CHROMEDRIVER").unwrap_or_else(|| "chromedriver".into()))
                .arg(format!("--port={driver_port}"))
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .expect("start local ChromeDriver"),
        );
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap();
        let endpoint = format!("http://127.0.0.1:{driver_port}");
        wait_until(
            || client.get(format!("{endpoint}/status")).send().is_ok(),
            "ChromeDriver startup",
        );
        let mut options = json!({
            "args": ["--headless=new", "--disable-background-networking", "--disable-component-update", "--no-first-run", "--no-default-browser-check", "--window-size=1200,900", format!("--user-data-dir={}", scratch.join("chrome-profile").display())],
            "prefs": {"download.default_directory": scratch.join("downloads"), "download.prompt_for_download": false}
        });
        if let Some(binary) = env::var_os("CHROME") {
            options["binary"] = json!(binary);
        }
        let response: Value = client
            .post(format!("{endpoint}/session"))
            .json(&json!({"capabilities":{"alwaysMatch":{
                "browserName":"chrome", "goog:chromeOptions": options,
                "goog:loggingPrefs":{"performance":"ALL", "browser":"ALL"}
            }}}))
            .send()
            .unwrap()
            .json()
            .unwrap();
        let session = response["value"]["sessionId"]
            .as_str()
            .unwrap_or_else(|| panic!("create browser session: {response}"))
            .to_owned();
        Self {
            client,
            endpoint,
            session,
            _driver: driver,
        }
    }

    fn request(&self, method: Method, path: &str, body: Value) -> Value {
        let url = format!("{}/session/{}{}", self.endpoint, self.session, path);
        let mut request = self.client.request(method.clone(), url);
        if method == Method::POST {
            request = request.json(&body);
        }
        let response = request.send().unwrap();
        let status = response.status();
        let value: Value = response.json().unwrap();
        assert!(status.is_success(), "WebDriver {path}: {value}");
        value["value"].clone()
    }

    fn element(&self, selector: &str) -> String {
        let mut found = None;
        wait_until(
            || {
                found = self
                    .request(
                        Method::POST,
                        "/elements",
                        json!({"using":"css selector","value":selector}),
                    )
                    .as_array()
                    .unwrap()
                    .first()
                    .and_then(|v| v["element-6066-11e4-a52e-4f735466cecf"].as_str())
                    .map(str::to_owned);
                found.is_some()
            },
            selector,
        );
        found.unwrap()
    }

    fn text(&self, selector: &str) -> String {
        self.request(
            Method::GET,
            &format!("/element/{}/text", self.element(selector)),
            Value::Null,
        )
        .as_str()
        .unwrap()
        .to_owned()
    }

    fn verify(&self, url: &str, expected: &Value, scratch: &Path) {
        // Scope network evidence to this dashboard navigation; a fresh Chrome
        // profile can otherwise include its internal new-tab startup assets.
        self.request(Method::POST, "/url", json!({"url":"about:blank"}));
        self.request(Method::POST, "/log", json!({"type":"performance"}));
        self.request(Method::POST, "/log", json!({"type":"browser"}));
        self.request(Method::POST, "/url", json!({"url":url}));
        let count = expected["snapshots"].as_array().unwrap().len();
        assert_eq!(self.text("#history-count"), format!("{count} observations"));
        let status = self.text("#history-status");
        if count == 0 {
            assert!(status.contains("No observations"));
        } else {
            assert!(status.contains("UTC"));
        }
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":375,"height":800}),
        );
        let rect = self.request(
            Method::GET,
            &format!("/element/{}/rect", self.element("main")),
            Value::Null,
        );
        assert!(rect["width"].as_f64().unwrap() <= 375.0);
        let download = scratch.join("downloads/history.json");
        self.request(
            Method::POST,
            &format!("/element/{}/click", self.element("#download-history")),
            json!({}),
        );
        wait_until(|| download.exists(), "history.json download");
        let actual: Value = serde_json::from_str(&fs::read_to_string(&download).unwrap()).unwrap();
        assert_eq!(actual, *expected);
        fs::remove_file(download).unwrap();

        let log = self.request(Method::POST, "/log", json!({"type":"performance"}));
        let mut wasm_requested = false;
        for entry in log.as_array().unwrap() {
            let message: Value = serde_json::from_str(entry["message"].as_str().unwrap()).unwrap();
            if message["message"]["method"] != "Network.requestWillBeSent" {
                continue;
            }
            let request = message["message"]["params"]["request"]["url"]
                .as_str()
                .unwrap();
            if request.starts_with("data:") || request.starts_with("blob:") {
                continue;
            }
            assert!(
                request.starts_with(url),
                "unexpected runtime request: {request}"
            );
            assert!(
                request == url
                    || [".js", ".css", ".wasm"]
                        .iter()
                        .any(|ext| request.ends_with(ext)),
                "separate data or unexpected asset request: {request}"
            );
            wasm_requested |= request.ends_with(".wasm");
        }
        assert!(wasm_requested, "browser must run the actual compiled WASM");
        let log = self.request(Method::POST, "/log", json!({"type":"browser"}));
        assert!(
            !log.as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["level"] == "SEVERE"),
            "browser errors: {log}"
        );
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self
            .client
            .delete(format!("{}/session/{}", self.endpoint, self.session))
            .send();
    }
}

fn build(root: &Path, history: &Path, dist: &Path, public_url: &str, success: bool) {
    let output = Command::new(root.join("bin/build-dashboard.sh"))
        .env("MNM_STATS_HISTORY", history)
        .arg("--dist")
        .arg(dist)
        .arg("--public-url")
        .arg(public_url)
        .output()
        .unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "build result:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !success {
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(diagnostic.contains("invalid history"), "{diagnostic}");
    }
}

#[test]
fn embedded_history_download_subpath_and_cached_rebuilds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let scratch = root.join(format!(
        ".session/tests/browser-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(scratch.join("downloads")).unwrap();
    fs::create_dir_all(scratch.join("site")).unwrap();
    println!("Browser test artifacts: {}", scratch.display());
    let history = scratch.join("history.jsonl");
    fs::write(&history, "").unwrap();
    let server_port = port();
    let log = fs::File::create(scratch.join("server.log")).unwrap();
    let _server = Process(
        Command::new("python3")
            .args([
                "-m",
                "http.server",
                &server_port.to_string(),
                "--bind",
                "127.0.0.1",
                "--directory",
            ])
            .arg(scratch.join("site"))
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap(),
    );
    let origin = format!("http://127.0.0.1:{server_port}/");
    let client = Client::builder().no_proxy().build().unwrap();
    wait_until(|| client.get(&origin).send().is_ok(), "local static server");
    let browser = Browser::new(&scratch);

    build(&root, &history, &scratch.join("site"), "/", true);
    browser.verify(
        &origin,
        &json!({"schema_version":1,"snapshots":[]}),
        &scratch,
    );
    println!("Empty history renders and downloads at the site root.");

    let first = json!({"schema_version":1,"observed_at":"2026-02-28T23:10:00.123Z","active_subscriptions":40,
        "servers":[{"id":"new-server","name":"Server Ω \"West\"\nTwo","daily_active":0,"monthly_active":20,"online":5,
        "starting_zones":[{"id":"zone","name":"Starting zone","online":2}]}]});
    let mut second = first.clone();
    second["observed_at"] = json!("2026-03-01T01:10:00Z");
    second["active_subscriptions"] = json!(42);
    second["servers"][0]["id"] = json!("another-server");
    let mut snapshots = Vec::new();
    for records in [vec![first.clone()], vec![first, second]] {
        fs::write(
            &history,
            records.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        snapshots = records
            .into_iter()
            .map(|mut record| {
                record.as_object_mut().unwrap().remove("schema_version");
                record
            })
            .collect::<Vec<_>>();
        build(&root, &history, &scratch.join("site/mnm"), "/mnm/", true);
        browser.verify(
            &format!("{origin}mnm/"),
            &json!({"schema_version":1,"snapshots":snapshots}),
            &scratch,
        );
    }
    println!(
        "History-only changes rebuild cached output; all fields survive download under /mnm/."
    );

    let previous = fs::read(scratch.join("site/mnm/index.html")).unwrap();
    fs::write(&history, "{\"schema_version\":1").unwrap();
    build(&root, &history, &scratch.join("site/mnm"), "/mnm/", false);
    assert_eq!(
        fs::read(scratch.join("site/mnm/index.html")).unwrap(),
        previous
    );
    browser.verify(
        &format!("{origin}mnm/"),
        &json!({"schema_version":1,"snapshots":snapshots}),
        &scratch,
    );
    println!("Invalid history fails the build and leaves the last valid site usable.");
}
