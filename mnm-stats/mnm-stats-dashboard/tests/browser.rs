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

fn completed_download(path: &Path) -> Option<Value> {
    // Chrome may create the destination before it finishes writing the JSON.
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[test]
fn download_readiness_requires_complete_json() {
    let scratch = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../.session/tests/download-readiness-{}",
        std::process::id()
    ));
    fs::create_dir_all(&scratch).unwrap();
    let path = scratch.join("history.json");
    assert!(completed_download(&path).is_none());
    for incomplete in ["", "{\"schema_version\":1,\"snapshots\":["] {
        fs::write(&path, incomplete).unwrap();
        assert!(completed_download(&path).is_none());
    }
    fs::write(&path, r#"{"schema_version":1,"snapshots":[]}"#).unwrap();
    assert_eq!(
        completed_download(&path),
        Some(json!({"schema_version":1,"snapshots":[]}))
    );
    fs::remove_dir_all(scratch).unwrap();
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
                .arg("--verbose")
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
            "args": ["--headless=new", "--disable-background-networking", "--disable-component-update", "--disable-sync", "--disable-notifications", "--disable-features=GCM,OptimizationHints,MediaRouter", "--no-first-run", "--no-default-browser-check", "--window-size=1200,900", format!("--user-data-dir={}", scratch.join("chrome-profile").display())],
            "prefs": {"download.default_directory": scratch.join("downloads"), "download.prompt_for_download": false}
        });
        if env::var_os("MNM_STATS_CONTAINER").is_some() {
            options["args"]
                .as_array_mut()
                .unwrap()
                .push(json!("--no-sandbox"));
        }
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
        let browser = Self {
            client,
            endpoint,
            session,
            _driver: driver,
        };
        // Fix the browser clock; fixtures and assertions stay in Rust.
        // Pausing all Chrome timers stalls navigation.
        let now: chrono::DateTime<chrono::Utc> = "2026-06-01T12:00:00Z".parse().unwrap();
        browser.request(
            Method::POST,
            "/goog/cdp/execute",
            json!({
            "cmd":"Page.addScriptToEvaluateOnNewDocument",
            "params":{"source":format!("Date.now = () => {};", now.timestamp_millis())}
            }),
        );
        browser
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

    fn count(&self, selector: &str) -> usize {
        self.request(
            Method::POST,
            "/elements",
            json!({"using":"css selector","value":selector}),
        )
        .as_array()
        .unwrap()
        .len()
    }

    fn click(&self, selector: &str) {
        self.request(
            Method::POST,
            &format!("/element/{}/click", self.element(selector)),
            json!({}),
        );
    }

    fn move_pointer(&self, selector: &str, x: i32, y: i32) {
        let element = self.element(selector);
        self.request(
            Method::POST,
            "/actions",
            json!({"actions":[{
                "type":"pointer", "id":"mouse", "parameters":{"pointerType":"mouse"},
                "actions":[{"type":"pointerMove", "duration":0,
                    "origin":{"element-6066-11e4-a52e-4f735466cecf":element}, "x":x,"y":y}]
            }]}),
        );
    }

    fn hover(&self, selector: &str) {
        self.element(selector);
        // Scroll without clicking: overlapping series can cover a marker, but
        // moving the mouse to that coordinate must still show all observations.
        let document = self.request(
            Method::POST,
            "/goog/cdp/execute",
            json!({
                "cmd":"DOM.getDocument", "params":{}
            }),
        );
        let node = self.request(Method::POST, "/goog/cdp/execute", json!({
            "cmd":"DOM.querySelector", "params":{"nodeId":document["root"]["nodeId"],"selector":selector}
        }));
        self.request(
            Method::POST,
            "/goog/cdp/execute",
            json!({
                "cmd":"DOM.scrollIntoViewIfNeeded", "params":{"nodeId":node["nodeId"]}
            }),
        );
        // Scrolling queues an event that clears hover details. Let the browser
        // finish that frame before moving the pointer onto the scrolled marker.
        self.request(
            Method::POST,
            "/execute/async",
            json!({
                "script":"const done = arguments[0]; requestAnimationFrame(() => requestAnimationFrame(() => done(null)));",
                "args":[]
            }),
        );
        // Adjacent observations can be less than a pixel apart. Aim at the
        // outer edge of the first/last marker to select that boundary sample.
        let position = self.request(Method::POST, "/execute/sync", json!({
            "script":"const e=document.querySelector(arguments[0]); const r=e.getBoundingClientRect(); const offset=e.matches('.point') ? (e===e.parentElement.firstElementChild ? -1 : e===e.parentElement.lastElementChild ? 1 : 0) : 0; return [r.x+r.width/2+offset,r.y+r.height/2];",
            "args":[selector]
        }));
        self.pointer_at(&position);
    }

    fn pointer_at(&self, position: &Value) {
        self.request(
            Method::POST,
            "/goog/cdp/execute",
            json!({
                "cmd":"Input.dispatchMouseEvent",
                "params":{"type":"mouseMoved","x":position[0],"y":position[1],"buttons":0}
            }),
        );
    }

    fn expect_count(&self, selector: &str, expected: usize) {
        wait_until(
            || self.count(selector) == expected,
            &format!("{selector} should contain {expected} elements"),
        );
    }

    fn select(&self, selector: &str, value: &str) {
        self.click(&format!("{selector} option[value='{value}']"));
    }

    fn input(&self, selector: &str, value: &str) {
        let element = self.element(selector);
        self.request(
            Method::POST,
            &format!("/element/{element}/clear"),
            json!({}),
        );
        self.request(
            Method::POST,
            &format!("/element/{element}/value"),
            json!({"text":value}),
        );
    }

    fn expect_text(&self, selector: &str, expected: &str) {
        wait_until(
            || self.text(selector) == expected,
            &format!("{selector} should contain {expected:?}"),
        );
    }

    fn verify_download(&self, expected: &Value, scratch: &Path) {
        let download = scratch.join("downloads/history.json");
        self.click("#download-history");
        let mut actual = None;
        wait_until(
            || {
                actual = completed_download(&download);
                actual.is_some()
            },
            "complete history.json download",
        );
        assert_eq!(actual.unwrap(), *expected);
        fs::remove_file(download).unwrap();
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
        assert_eq!(
            self.text("footer a[href='https://plotly.com/javascript/']"),
            "Charts by Plotly"
        );
        self.expect_count(
            ".plot-surface[data-ready='true']",
            self.count(".plot-surface"),
        );
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
        self.verify_download(expected, scratch);
        self.verify_requests(url, true);
    }

    fn verify_features(&self, url: &str, expected: &Value, scratch: &Path) {
        let metrics = [
            "daily",
            "monthly",
            "subscriptions",
            "online",
            "starting-zones",
            "zone-z",
            "zone-w",
            "daily-monthly",
            "daily-subscriptions",
            "monthly-subscriptions",
        ];
        let records = expected["snapshots"].as_array().unwrap();
        let now: chrono::DateTime<chrono::Utc> = "2026-06-01T12:00:00Z".parse().unwrap();
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":1280,"height":1000}),
        );
        assert!(
            self.text("#selected-interval")
                .contains("2026-06-01T12:00:00Z")
        );
        assert!(self.text("#freshness").contains("Stale data"));
        assert!(self.text("#weekly-unavailable").contains("not available"));
        assert_eq!(self.count(".chart-card"), metrics.len());
        assert!(self.text("#server-scope").contains("Retired server"));
        for (key, days) in [
            ("7", Some(7)),
            ("30", Some(30)),
            ("90", Some(90)),
            ("180", Some(180)),
            ("365", Some(365)),
            ("all", None),
        ] {
            self.select("#time-range", key);
            let selected: Vec<_> = records
                .iter()
                .filter(|record| {
                    let at: chrono::DateTime<chrono::Utc> =
                        record["observed_at"].as_str().unwrap().parse().unwrap();
                    at <= now && days.is_none_or(|d| at >= now - chrono::Duration::days(d))
                })
                .collect();
            for metric in metrics {
                let count = selected
                    .iter()
                    .filter(|r| {
                        !(metric.ends_with("-subscriptions") && r["active_subscriptions"] == 0)
                    })
                    .count();
                self.expect_text(
                    &format!("[data-metric='{metric}'] .sample-count"),
                    &format!("{count} plotted observations"),
                );
                self.ready(metric);
            }
        }
        // Exact values and pagination use original observations, not source chart history.
        self.click("[data-metric='daily'] summary");
        self.expect_count("[data-metric='daily'] tbody tr", 50);
        self.click("[data-metric='daily'] .next");
        self.expect_count("[data-metric='daily'] tbody tr", records.len() - 50);
        self.click("[data-metric='daily'] .previous");
        self.click("[data-metric='daily'] summary");
        self.select("#time-range", "7");
        self.select("#server-scope", "a");
        let first = records
            .iter()
            .find(|r| r["observed_at"] == "2026-05-25T12:00:00Z")
            .unwrap();
        let a = &first["servers"][0];
        let n = |field: &str| a[field].as_u64().unwrap();
        let subscriptions = first["active_subscriptions"].as_u64().unwrap();
        for (metric, value) in [
            ("daily", n("daily_active").to_string()),
            ("monthly", n("monthly_active").to_string()),
            ("subscriptions", subscriptions.to_string()),
            ("online", n("online").to_string()),
            (
                "starting-zones",
                (a["starting_zones"][0]["online"].as_u64().unwrap()
                    + a["starting_zones"][1]["online"].as_u64().unwrap())
                .to_string(),
            ),
            (
                "zone-z",
                a["starting_zones"][0]["online"]
                    .as_u64()
                    .unwrap()
                    .to_string(),
            ),
            (
                "zone-w",
                a["starting_zones"][1]["online"]
                    .as_u64()
                    .unwrap()
                    .to_string(),
            ),
            (
                "daily-monthly",
                format!(
                    "{:.2}% ({} / {})",
                    n("daily_active") as f64 / n("monthly_active") as f64 * 100.0,
                    n("daily_active"),
                    n("monthly_active")
                ),
            ),
            (
                "daily-subscriptions",
                format!(
                    "{:.2}% ({} / {})",
                    n("daily_active") as f64 / subscriptions as f64 * 100.0,
                    n("daily_active"),
                    subscriptions
                ),
            ),
            (
                "monthly-subscriptions",
                format!(
                    "{:.2}% ({} / {})",
                    n("monthly_active") as f64 / subscriptions as f64 * 100.0,
                    n("monthly_active"),
                    subscriptions
                ),
            ),
        ] {
            self.hover(&format!("[data-metric='{metric}'] .scatterlayer .point"));
            self.expect_hover(
                metric,
                if metric == "subscriptions" {
                    "Global subscriptions"
                } else {
                    "Alpha <island> & West"
                },
                "2026-05-25T12:00:00Z",
                &value,
            );
            self.hover(&format!("[data-metric='{metric}'] .unit"));
            self.expect_count(".hovertext", 0);
            self.click(&format!("[data-metric='{metric}'] summary"));
            self.expect_text(
                &format!(
                    "[data-metric='{metric}'] tr[data-at='2026-05-25T12:00:00Z'] .exact-value"
                ),
                &value,
            );
            if metric.ends_with("-subscriptions") {
                self.expect_text(
                    &format!(
                        "[data-metric='{metric}'] tr[data-at='2026-05-30T13:00:00Z'] .exact-value"
                    ),
                    "not available",
                );
            }
            self.click(&format!("[data-metric='{metric}'] summary"));
        }
        self.hover("[data-metric='daily'] .scatterlayer .point");
        self.select("#server-scope", "retired");
        self.expect_count(".hovertext", 0);
        for metric in metrics.into_iter().filter(|m| *m != "subscriptions") {
            self.expect_text(
                &format!("[data-metric='{metric}'] .empty-chart"),
                "No available observations for this selection.",
            );
        }
        assert_eq!(
            self.count("[data-metric='subscriptions'] .plot-surface[data-ready='true']"),
            1
        );
        self.select("#server-scope", "a");
        assert!(self.text("#correlation-scope").contains("Alpha"));
        assert_eq!(self.count(".correlation-value"), 3);
        assert!(self.text(".correlations").contains("r ="));
        self.select("#comparison-mode", "entities");
        for metric in metrics {
            assert_eq!(
                self.count(&format!("[data-metric='{metric}'] .legend li")),
                if metric == "subscriptions" { 1 } else { 3 }
            );
        }
        self.hover("[data-metric='online'] .scatterlayer .trace:last-child .point:last-child");
        self.expect_hover(
            "online",
            "Beta",
            records.last().unwrap()["observed_at"].as_str().unwrap(),
            "5",
        );
        // Three individual servers, then all servers alongside the three individuals.
        self.click("#entity-choices input[value='all']");
        self.click("#entity-choices input[value='server:c']");
        assert_eq!(self.count("[data-metric='daily'] .legend li"), 3);
        self.click("#entity-choices input[value='all']");
        assert_eq!(self.count("[data-metric='daily'] .legend li"), 4);
        self.verify_download(expected, scratch);
        self.click("#entity-choices input[value='server:b']");
        assert_eq!(self.count("[data-metric='daily'] .legend li"), 3);
        for (mode, periods) in [
            ("months", ["2024-02", "2024-03", "2024-04"]),
            ("years", ["2023", "2024", "2025"]),
        ] {
            self.select("#comparison-mode", mode);
            while self.count(".remove-period") > 0 {
                self.click(".remove-period");
            }
            for period in periods {
                self.input("#period-input", period);
                self.click("#add-period");
            }
            for metric in metrics {
                assert_eq!(
                    self.count(&format!("[data-metric='{metric}'] .legend li")),
                    3,
                    "{mode} / {metric}"
                );
                self.ready(metric);
            }
            // Constant zone counts coincide across all three periods. Every
            // series must retain its actual observation date, not its aligned x.
            self.hover("[data-metric='zone-w'] .scatterlayer .point");
            let dates = if mode == "months" {
                [
                    "2024-02-01T00:00:00Z",
                    "2024-03-01T00:00:00Z",
                    "2024-04-01T00:00:00Z",
                ]
            } else {
                [
                    "2023-02-01T00:00:00Z",
                    "2024-02-01T00:00:00Z",
                    "2025-02-01T00:00:00Z",
                ]
            };
            for at in dates {
                self.expect_hover("zone-w", "Alpha <island> & West", at, "3");
            }
            self.expect_count("[data-metric='zone-w'] .hovertext", 3);
            self.click("[data-metric='daily'] summary");
            assert!(
                self.text("[data-metric='daily'] table")
                    .contains("2024-02-29T23:00:00Z")
            );
            self.click("[data-metric='daily'] summary");
            self.verify_download(expected, scratch);
            self.click(".remove-period");
            assert_eq!(self.count("[data-metric='daily'] .legend li"), 2);
            self.input("#period-input", "invalid");
            self.click("#add-period");
            assert!(!self.text(".period-picker [role='alert']").is_empty());
        }
        self.select("#comparison-mode", "intervals");
        self.input("#interval-hours", "24");
        for start in ["2024-02-01T00:00", "2024-03-01T00:00", "2024-04-01T00:00"] {
            self.input("#interval-start", start);
            self.click("#add-interval");
        }
        for metric in metrics {
            assert_eq!(
                self.count(&format!("[data-metric='{metric}'] .legend li")),
                3
            );
            self.ready(metric);
        }
        self.hover("[data-metric='zone-w'] .scatterlayer .point");
        assert!(
            self.text("[data-metric='zone-w'] .hoverlayer")
                .contains("2024-02-01T00:00:00Z UTC")
        );
        self.move_pointer("[data-metric='zone-w'] .plot-surface", 0, 0);
        self.expect_count(".hovertext", 0);
        self.input("#interval-hours", "0");
        self.expect_text(
            "[data-metric='daily'] .error",
            "Choose a positive duration in hours.",
        );
        self.input("#interval-hours", "24");
        self.verify_download(expected, scratch);
        self.verify_requests(url, false);
        self.select("#comparison-mode", "overview");
        // Capture the real browser for visual review; decoding is a separate local activity.
        self.select("#server-scope", "");
        self.request(Method::POST, "/url", json!({"url":url}));
        self.element("#history-count");
        fs::write(
            scratch.join("dashboard-desktop.png.b64"),
            self.request(Method::GET, "/screenshot", Value::Null)
                .as_str()
                .unwrap(),
        )
        .unwrap();
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":375,"height":900}),
        );
        let rect = self.request(
            Method::GET,
            &format!("/element/{}/rect", self.element("main")),
            Value::Null,
        );
        assert!(rect["width"].as_f64().unwrap() <= 375.0);
        self.select("#time-range", "7");
        self.select("#server-scope", "a");
        // The rightmost marker requires scrolling the narrow plot horizontally.
        self.hover("[data-metric='daily'] .scatterlayer .trace:last-child .point:last-child");
        let last = records.last().unwrap();
        self.expect_hover(
            "daily",
            "Alpha <island> & West",
            last["observed_at"].as_str().unwrap(),
            &last["servers"][0]["daily_active"]
                .as_u64()
                .unwrap()
                .to_string(),
        );
        fs::write(
            scratch.join("dashboard-mobile.png.b64"),
            self.request(Method::GET, "/screenshot", Value::Null)
                .as_str()
                .unwrap(),
        )
        .unwrap();
        println!(
            "All metric families, six ranges, exact values, historical entities, three-period/entity comparisons, missing data, and filter-independent downloads verified."
        );
    }

    fn ready(&self, metric: &str) {
        self.expect_count(
            &format!("[data-metric='{metric}'] .plot-surface[data-ready='true']"),
            1,
        );
    }

    fn expect_hover(&self, metric: &str, series: &str, at: &str, value: &str) {
        let selector = format!("[data-metric='{metric}'] .hovertext");
        wait_until(
            || {
                let rows = self.request(Method::POST, "/execute/sync", json!({
                "script":"return Array.from(document.querySelectorAll(arguments[0]), row => row.textContent);",
                "args":[selector]
            }));
                rows.as_array().unwrap().iter().any(|row| {
                    let text = row.as_str().unwrap();
                    text.contains(series)
                        && text.contains(&format!("{at} UTC"))
                        && text.ends_with(value)
                })
            },
            &format!("{metric}: hover must show {series}, {at}, {value}"),
        );
    }

    fn computed(&self, selector: &str, property: &str) -> String {
        let value = self
            .request(
                Method::GET,
                &format!("/element/{}/css/{property}", self.element(selector)),
                Value::Null,
            )
            .as_str()
            .unwrap()
            .to_owned();
        // WebDriver normalizes some opaque CSS colors to rgba; SVG fill stays rgb.
        if let Some(rgb) = value
            .strip_prefix("rgba(")
            .and_then(|v| v.strip_suffix(", 1)"))
        {
            format!("rgb({rgb})")
        } else {
            value
        }
    }

    fn verify_token_styles(&self, changed: bool) {
        self.select("#comparison-mode", "overview");
        // Seven days keeps demo observations sparse enough to draw point markers.
        self.select("#time-range", "7");
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":1280,"height":1000}),
        );
        assert_eq!(
            self.computed("html", "background-color"),
            if changed {
                "rgb(25, 30, 35)"
            } else {
                "rgb(17, 26, 32)"
            }
        );
        assert_eq!(self.computed(".chart-card", "border-radius"), "10.4px");
        assert_eq!(
            self.computed(".metric-description", "font-size"),
            if changed { "16px" } else { "14px" }
        );
        assert_eq!(
            self.computed("#time-range", "padding-top"),
            if changed { "8px" } else { "12px" }
        );
        assert_eq!(
            self.computed(".chart-grid", "grid-template-columns")
                .split_whitespace()
                .count(),
            2
        );
        let size = self.computed("[data-metric='daily'] .xtick text", "font-size");
        let size: f64 = size.strip_suffix("px").unwrap().parse().unwrap();
        assert!((size - if changed { 18.0 } else { 12.0 }).abs() < 0.001);
        let color = if changed {
            "rgba(204, 102, 51, 0.5)"
        } else {
            "rgb(139, 217, 198)"
        };
        assert_eq!(
            self.computed("[data-metric='daily'] .swatch", "background-color"),
            color
        );
        assert_eq!(
            self.computed("[data-metric='daily'] .scatterlayer .point", "fill"),
            if changed { "rgb(204, 102, 51)" } else { color }
        );
        assert_eq!(
            self.computed("[data-metric='daily'] .scatterlayer .point", "fill-opacity"),
            if changed { "0.5" } else { "1" }
        );
        self.hover("[data-metric='daily'] .scatterlayer .point");
        assert_eq!(
            self.computed("[data-metric='daily'] .hovertext path", "stroke"),
            color
        );
        assert!(
            self.text("[data-metric='daily'] .hoverlayer")
                .contains("UTC")
        );
        // The changed breakpoint is 900px, so the same 1000px window changes layout.
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":1000,"height":900}),
        );
        assert_eq!(
            self.computed(".chart-grid", "grid-template-columns")
                .split_whitespace()
                .count(),
            if changed { 2 } else { 1 }
        );
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":375,"height":800}),
        );
        assert_eq!(
            self.computed(".chart-grid", "grid-template-columns")
                .split_whitespace()
                .count(),
            1
        );
        assert_eq!(self.computed(".page-header", "flex-direction"), "column");
        assert_eq!(self.computed(".plot-surface", "min-width"), "480px");
    }

    fn verify_extended_palette(&self) {
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":1280,"height":1000}),
        );
        self.select("#comparison-mode", "months");
        while self.count(".remove-period") > 0 {
            self.click(".remove-period");
        }
        for month in [
            "2023-02", "2023-03", "2023-04", "2024-02", "2024-03", "2024-04", "2025-02",
        ] {
            self.input("#period-input", month);
            self.click("#add-period");
        }
        self.expect_count("[data-metric='daily'] .legend li", 7);
        for i in 0..7 {
            let swatch = format!(
                "[data-metric='daily'] .legend li:nth-child({}) .swatch",
                i + 1
            );
            // Each selected fixture month has exactly three observations.
            let circle = format!(
                "[data-metric='daily'] .scatterlayer .trace:nth-child({}) .point",
                i + 1
            );
            let color = self.computed(&swatch, "background-color");
            assert_eq!(self.computed(&circle, "fill"), color);
            self.hover(&circle);
            wait_until(
                || {
                    self.request(Method::POST, "/execute/sync", json!({
                    "script":"const row = Array.from(document.querySelectorAll(arguments[0])).find(e => e.textContent.startsWith(arguments[1])); return row ? getComputedStyle(row.querySelector('path')).stroke : null;",
                    "args":["[data-metric='daily'] .hovertext", format!("{}. ", i + 1)]
                })) == color
                },
                &format!("series {i} hover border matches its legend"),
            );
        }
        let height = self.computed("[data-metric='daily'] .plot-surface", "height");
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":375,"height":800}),
        );
        wait_until(
            || {
                self.request(Method::POST, "/execute/sync", json!({
            "script":"const e=document.querySelector(arguments[0]), svg=e.querySelector('svg.main-svg'); return Number(svg.getAttribute('width'))===e.clientWidth && Number(svg.getAttribute('height'))===e.clientHeight && getComputedStyle(e).height===arguments[1];",
            "args":["[data-metric='daily'] .plot-surface", height]
        })) == true
            },
            "resizing retains enough chart height for every comparison hover label",
        );
        self.hover("[data-metric='daily'] .scatterlayer .point");
        self.expect_count("[data-metric='daily'] .hovertext", 7);
        self.select("#comparison-mode", "overview");
    }

    fn verify_requests(&self, url: &str, require_wasm: bool) {
        let plotly_url = include_str!("../index.html")
            .split("src=\"")
            .filter_map(|part| part.split_once('"').map(|(url, _)| url))
            .find(|url| url.starts_with("https://cdn.plot.ly/"))
            .expect("Plotly CDN URL in frontend HTML");
        let log = self.request(Method::POST, "/log", json!({"type":"performance"}));
        let mut wasm_requested = false;
        let mut plotly_requested = false;
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
            if request == plotly_url {
                plotly_requested = true;
                continue;
            }
            assert!(
                request.starts_with(url),
                "unexpected runtime request: {request}"
            );
            assert!(
                request == url || [".js", ".wasm"].iter().any(|ext| request.ends_with(ext)),
                "separate data or unexpected asset request: {request}"
            );
            wasm_requested |= request.ends_with(".wasm");
        }
        assert!(
            !require_wasm || wasm_requested,
            "browser must run the actual compiled WASM"
        );
        assert!(
            !require_wasm || plotly_requested,
            "browser must load the chart engine from the configured CDN URL"
        );
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
    build_with_tokens(root, history, dist, public_url, success, None);
}

fn build_with_tokens(
    root: &Path,
    history: &Path,
    dist: &Path,
    public_url: &str,
    success: bool,
    tokens: Option<&Path>,
) {
    let mut command = Command::new(root.join("bin/build-dashboard.sh"));
    if let Some(tokens) = tokens {
        command.env("MNM_STATS_TOKENS", tokens);
    }
    let output = command
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
        assert!(
            diagnostic.contains(if tokens.is_some() {
                "invalid tokens"
            } else {
                "invalid history"
            }),
            "{diagnostic}"
        );
    }
}

fn comparison_history() -> Vec<Value> {
    use chrono::{Datelike, Duration, NaiveDate};
    let mut times = std::collections::BTreeSet::new();
    for year in [2023, 2024, 2025] {
        for month in [2, 3, 4] {
            let first = NaiveDate::from_ymd_opt(year, month, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc();
            times.insert(first);
            times.insert(first + Duration::hours(1));
            let next = first.checked_add_months(chrono::Months::new(1)).unwrap();
            times.insert(next - Duration::hours(1));
        }
    }
    let now: chrono::DateTime<chrono::Utc> = "2026-06-01T12:00:00Z".parse().unwrap();
    for days in [7, 30, 90, 180, 365] {
        let boundary = now - Duration::days(days);
        for hours in [-1, 0, 1] {
            times.insert(boundary + Duration::hours(hours));
        }
    }
    for days in 1..=4 {
        for hours in [0, 1, 2] {
            times.insert(now - Duration::days(days) + Duration::hours(hours));
        }
    }
    times.into_iter().enumerate().map(|(i, at)| {
        let server = |id, name, daily, monthly, online, z, w| json!({"id":id,"name":name,"daily_active":daily,"monthly_active":monthly,"online":online,"starting_zones":[{"id":"z","name":"Harbor & Hills","online":z},{"id":"w","name":"Lower Docks","online":w}]});
        let mut servers = vec![
            server("a", "Alpha <island> & West", 20 + i * 3, 10 + i, 100 + i, 7 + i, 3),
            server("b", "Beta", 5, 10, 5, 2, 1), server("c", "Gamma", 8, 16, 8, 3, 2),
        ];
        if at.year() == 2023 { servers.push(server("retired", "Retired server", 17, 25, 3, 1, 1)); }
        let subscriptions = if at.to_rfc3339() == "2026-05-30T13:00:00+00:00" { 0 } else { 10 + i };
        json!({"observed_at":at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),"active_subscriptions":subscriptions,"servers":servers})
    }).collect()
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
    assert!(fs::read_dir(scratch.join("site")).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("plotly-")
    }));
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
    for records in [vec![first.clone()], vec![first, second]] {
        fs::write(
            &history,
            records.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        let snapshots = records
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

    let snapshots = comparison_history();
    let input = snapshots
        .iter()
        .map(|s| {
            let mut record = s.clone();
            record["schema_version"] = json!(1);
            format!("{record}\n")
        })
        .collect::<String>();
    fs::write(scratch.join("comparison-history.jsonl"), &input).unwrap();
    fs::write(&history, input).unwrap();
    build(&root, &history, &scratch.join("site/mnm"), "/mnm/", true);
    let expected = json!({"schema_version":1,"snapshots":snapshots});
    let url = format!("{origin}mnm/");
    browser.verify(&url, &expected, &scratch);
    browser.verify_features(&url, &expected, &scratch);
    browser.verify_token_styles(false);
    browser.verify_extended_palette();

    let tokens = scratch.join("tokens.json");
    let changed = changed_tokens(&root);
    fs::write(&tokens, changed.to_string()).unwrap();
    build_with_tokens(
        &root,
        &history,
        &scratch.join("site/mnm"),
        "/mnm/",
        true,
        Some(&tokens),
    );
    browser.verify(&url, &expected, &scratch);
    browser.verify_token_styles(true);
    let previous_tokens_site = fs::read(scratch.join("site/mnm/index.html")).unwrap();
    let mut invalid = changed;
    invalid["chart"]["series"]["palette"]["01"]["$value"]["alpha"] = json!(2);
    fs::write(&tokens, invalid.to_string()).unwrap();
    build_with_tokens(
        &root,
        &history,
        &scratch.join("site/mnm"),
        "/mnm/",
        false,
        Some(&tokens),
    );
    assert_eq!(
        fs::read(scratch.join("site/mnm/index.html")).unwrap(),
        previous_tokens_site
    );
    browser.verify(&url, &expected, &scratch);
    browser.verify_token_styles(true);
    println!(
        "Token-only cached rebuild updates embedded CSS and chart/hover colors; invalid tokens preserve the last site."
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
    verify_preview(&root, &scratch, &browser);
}

fn changed_tokens(root: &Path) -> Value {
    let mut tokens: Value = serde_json::from_str(
        &fs::read_to_string(root.join("specs/design-tokens.tokens.json")).unwrap(),
    )
    .unwrap();
    tokens["color"]["surface"]["page"]["$value"]["components"] =
        json!([25.0 / 255.0, 30.0 / 255.0, 35.0 / 255.0]);
    tokens["chart"]["series"]["palette"]["01"]["$value"]["components"] = json!([0.8, 0.4, 0.2]);
    tokens["chart"]["series"]["palette"]["01"]["$value"]["alpha"] = json!(0.5);
    tokens["chart"]["axis"]["label"]["font-size"]["$value"]["value"] = json!(18);
    tokens["breakpoint"]["medium"]["$value"]["value"] = json!(900);
    // Shared primitives update their aliases; an input-only override stays local.
    tokens["scale"]["spacing"]["3"]["$value"]["value"] = json!(1);
    tokens["scale"]["font-size"]["2"]["$value"]["value"] = json!(1);
    tokens["spacing"]["input"]["padding"]["$value"] = json!({"value":0.5,"unit":"rem"});
    tokens
}

fn verify_preview(root: &Path, scratch: &Path, browser: &Browser) {
    let tokens = scratch.join("preview-tokens.json");
    fs::copy(root.join("specs/design-tokens.tokens.json"), &tokens).unwrap();
    let history = scratch.join("preview history.jsonl");
    let at = "2026-06-01T12:00:00Z";
    let generate = || {
        let output = Command::new("cargo")
            .args([
                "run",
                "--locked",
                "--quiet",
                "-p",
                "mnm-stats-dashboard",
                "--example",
                "demo-history",
                "--",
                at,
            ])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let demo = generate();
    assert_eq!(generate(), demo, "a fixed time reproduces demo history");
    let parsed = mnm_stats_model::History::from_jsonl(std::str::from_utf8(&demo).unwrap()).unwrap();
    assert_eq!(parsed.snapshots().len(), 480);
    assert_eq!(
        parsed.snapshots().last().unwrap().observed_at.to_rfc3339(),
        "2026-06-01T12:00:00+00:00"
    );
    fs::write(&history, &demo).unwrap();
    let preview_port = port();
    let origin = format!("http://127.0.0.1:{preview_port}/");
    // Exercise --demo first, then explicit input with the same Cargo cache.
    let demo_preview = start_preview(root, scratch, None, "demo", preview_port);
    let expected: Value = serde_json::from_str(&parsed.to_json().unwrap()).unwrap();
    browser.verify(&origin, &expected, scratch);
    assert!(browser.text("#demo-notice").contains("synthetic"));
    assert_eq!(browser.count(".plot-surface[data-ready='true']"), 10);
    drop(demo_preview);
    let preview = start_preview(root, scratch, Some(&history), "explicit", preview_port);
    let expected: Value = serde_json::from_str(&parsed.to_json().unwrap()).unwrap();
    browser.verify(&origin, &expected, scratch);
    assert_eq!(
        browser.count(".plot-surface[data-ready='true']"),
        10,
        "recent demo values appear in the default range"
    );
    assert_eq!(
        browser.count("#demo-notice"),
        0,
        "explicit history is not labeled demo"
    );

    browser.request(
        Method::POST,
        "/window/rect",
        json!({"width":1280,"height":1000}),
    );
    browser.select("#time-range", "all");
    browser.ready("daily");
    assert_eq!(
        browser.count("[data-metric='daily'] .scatterlayer .point"),
        0
    );
    let point = browser.request(Method::POST, "/execute/async", json!({
        "script":r#"const done = arguments[arguments.length-1]; const line = Array.from(document.querySelectorAll("[data-metric='daily'] .scatterlayer .js-line")).at(-1); line.scrollIntoView({block:'center',inline:'end'}); requestAnimationFrame(() => requestAnimationFrame(() => { const p = line.getPointAtLength(line.getTotalLength()).matrixTransform(line.getScreenCTM()); done([p.x+1,p.y]); }));"#,
        "args":[]
    }));
    browser.pointer_at(&point);
    let last = expected["snapshots"].as_array().unwrap().last().unwrap();
    let total: u64 = last["servers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["daily_active"].as_u64().unwrap())
        .sum();
    browser.expect_hover("daily", "All servers", at, &total.to_string());
    browser.verify_token_styles(false);
    let index = scratch.join("preview-explicit-dist/index.html");
    let original = fs::read(&index).unwrap();
    let replacement = scratch.join("next-tokens.json");
    fs::write(&replacement, changed_tokens(root).to_string()).unwrap();
    fs::rename(&replacement, &tokens).unwrap();
    wait_for_preview_rebuild(&index, &original);
    browser.verify(&origin, &expected, scratch);
    browser.verify_token_styles(true);
    // Watchers must survive more than one atomic replacement.
    let original = fs::read(&index).unwrap();
    fs::copy(root.join("specs/design-tokens.tokens.json"), &replacement).unwrap();
    fs::rename(&replacement, &tokens).unwrap();
    wait_for_preview_rebuild(&index, &original);
    browser.verify(&origin, &expected, scratch);
    browser.verify_token_styles(false);
    println!("Token-only atomic replacements rebuild both CSS and charts in the running preview.");

    // Replace the file atomically, as a collector/editor can do. Its parent is outside the crate.
    let retained = parsed.snapshots().last().unwrap();
    let index = scratch.join("preview-explicit-dist/index.html");
    let original = fs::read(&index).unwrap();
    let updated = scratch.join("preview-next.jsonl");
    fs::write(&updated, retained.to_jsonl_record().unwrap()).unwrap();
    fs::rename(&updated, &history).unwrap();
    wait_for_preview_rebuild(&index, &original);
    let expected = json!({"schema_version":1,"snapshots":[retained]});
    browser.verify(&origin, &expected, scratch);
    browser.select("#time-range", "all");
    assert_eq!(browser.count(".plot-surface[data-ready='true']"), 10);
    browser.verify_download(&expected, scratch);
    // A second replacement verifies that watching survives atomic file replacement.
    let original = fs::read(&index).unwrap();
    fs::write(&updated, "").unwrap();
    fs::rename(&updated, &history).unwrap();
    wait_for_preview_rebuild(&index, &original);
    browser.verify(
        &origin,
        &json!({"schema_version":1,"snapshots":[]}),
        scratch,
    );
    drop(preview);
    fs::write(&history, "broken JSON").unwrap();
    let failed = Command::new(root.join("bin/serve-dashboard.sh"))
        .arg("--history")
        .arg(&history)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("line 1"));
    println!(
        "Development preview loads explicit history outside the crate and rebuilds after an atomic history-only change."
    );
}

fn start_preview(
    root: &Path,
    scratch: &Path,
    history: Option<&Path>,
    label: &str,
    preview_port: u16,
) -> Process {
    let origin = format!("http://127.0.0.1:{preview_port}/");
    let log_path = scratch.join(format!("preview-{label}.log"));
    let log = fs::File::create(&log_path).unwrap();
    let mut command = Command::new(root.join("bin/serve-dashboard.sh"));
    command.env("MNM_STATS_TOKENS", scratch.join("preview-tokens.json"));
    match history {
        Some(path) => {
            command.arg("--history").arg(path);
        }
        None => {
            command
                .arg("--demo")
                .env("MNM_STATS_DEMO_AT", "2026-06-01T12:00:00Z");
        }
    }
    let mut preview = Process(
        command
            .args(["--port", &preview_port.to_string(), "--dist"])
            .arg(scratch.join(format!("preview-{label}-dist")))
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap(),
    );
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(300);
    while !client
        .get(&origin)
        .send()
        .is_ok_and(|r| r.status().is_success())
    {
        assert!(
            preview.0.try_wait().unwrap().is_none(),
            "preview exited: {}",
            fs::read_to_string(&log_path).unwrap()
        );
        assert!(
            Instant::now() < deadline,
            "preview build timed out; see preview.log"
        );
        thread::sleep(Duration::from_millis(200));
    }
    preview
}

fn wait_for_preview_rebuild(index: &Path, previous: &[u8]) {
    let deadline = Instant::now() + Duration::from_secs(120);
    while !fs::read(index).is_ok_and(|current| current != previous) {
        assert!(
            Instant::now() < deadline,
            "input-only preview rebuild timed out"
        );
        thread::sleep(Duration::from_millis(200));
    }
}
