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

    // Cross-metric assertions follow the same section and zone navigation as a visitor.
    // Presentation assertions use the navigation controls directly.
    fn show_metric(&self, metric: &str) {
        let section = match metric {
            "daily" | "monthly" | "subscriptions" => "activity",
            "online" | "starting-zones" => "population",
            m if m.starts_with("zone-") => "population",
            _ => "relationships",
        };
        let present = self.request(
            Method::POST,
            "/execute/sync",
            json!({
                "script":"return !!document.querySelector(arguments[0]);",
                "args":[format!("[data-metric='{metric}']")]
            }),
        );
        if present == true {
            return;
        }
        self.click(&format!("#nav-{section}"));
        if let Some(zone) = metric.strip_prefix("zone-") {
            self.select("#zone-scope", zone);
        }
    }

    fn visit_chart_selector(&self, selector: &str) {
        if let Some(metric) = selector
            .strip_prefix("[data-metric='")
            .and_then(|s| s.split_once("']").map(|p| p.0))
        {
            self.show_metric(metric);
        }
    }

    fn element(&self, selector: &str) -> String {
        self.visit_chart_selector(selector);
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
        self.visit_chart_selector(selector);
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

    fn activate(&self, selector: &str) {
        self.request(
            Method::POST,
            &format!("/element/{}/value", self.element(selector)),
            json!({"text":"\u{e007}","value":["\u{e007}"]}),
        );
    }

    fn servers(&self, ids: &[&str]) {
        self.click("#servers-toggle");
        let selected: Vec<_> = ids
            .iter()
            .map(|id| {
                if id.is_empty() {
                    "all".to_string()
                } else {
                    format!("server:{id}")
                }
            })
            .collect();
        let choices = self.request(Method::POST, "/execute/sync", json!({
            "script":"return Array.from(document.querySelectorAll('#server-options input'), e => ({value:e.value,checked:e.checked}));", "args":[]
        }));
        for choice in choices.as_array().unwrap() {
            let value = choice["value"].as_str().unwrap();
            if choice["checked"].as_bool().unwrap() != selected.iter().any(|id| id == value) {
                self.click(&format!("#server-options input[value='{value}']"));
            }
        }
        self.click("#servers-toggle");
    }

    fn toggle_server(&self, value: &str) {
        self.click("#servers-toggle");
        self.click(&format!("#server-options input[value='{value}']"));
        self.click("#servers-toggle");
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
        match selector {
            "#time-range" => {
                self.click(selector);
                self.click(&format!("[data-range='{value}']"));
            }
            "#comparison-mode" => {
                self.click(selector);
                self.click(&format!("[data-comparison='{value}']"));
            }
            _ => self.click(&format!("{selector} option[value='{value}']")),
        }
    }

    fn date(&self, selector: &str, value: &str) {
        self.element(selector);
        // Native date controls have locale-specific keyboard segments. Set the
        // standard ISO value and deliver the same input event as date selection.
        self.request(Method::POST, "/execute/sync", json!({
            "script":"const e=document.querySelector(arguments[0]); e.value=arguments[1]; e.dispatchEvent(new Event('input',{bubbles:true}));", "args":[selector,value]
        }));
    }

    fn primary_range(&self, start: &str, end: &str) {
        self.click("#time-range");
        self.click("#custom-range");
        self.date("#range-start", start);
        self.date("#range-end", end);
        self.activate("#apply-range");
        self.expect_count(".date-menu[open]", 0);
    }

    fn custom_periods(&self, periods: &[(&str, &str)]) {
        self.primary_range(periods[0].0, periods[0].1);
        self.select("#comparison-mode", "custom");
        while self.count(".remove-period") > 0 {
            self.click(".remove-period");
            self.expect_count(".date-menu[open]", 1);
        }
        for (start, end) in &periods[1..] {
            self.date("#compare-start", start);
            self.date("#compare-end", end);
            self.activate("#add-period");
        }
        self.click("#comparison-mode");
    }

    fn expect_text(&self, selector: &str, expected: &str) {
        wait_until(
            || self.text(selector) == expected,
            &format!("{selector} should contain {expected:?}"),
        );
    }

    fn verify_download(&self, expected: &Value, scratch: &Path) {
        let download = scratch.join("downloads/history.json");
        self.activate("#download-history");
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
        let records = expected["snapshots"].as_array().unwrap();
        let count = records.len();
        self.expect_text(
            "#history-count",
            &format!(
                "{} {}",
                mnm_stats_dashboard::analysis::grouped_count(count as u128),
                if count == 1 { "record" } else { "records" }
            ),
        );
        self.expect_text("#time-range-selection", "Last 7 days");
        assert_eq!(
            self.text("footer a[href='https://plotly.com/javascript/']"),
            "Charts by Plotly"
        );
        self.expect_count(
            ".plot-surface[data-ready='true']",
            self.count(".plot-surface"),
        );
        let status = self.text("#history-status");
        assert!(status.contains("collected roughly hourly from M&M’s public statistics"));
        self.expect_text(
            "#history-status a[href='https://account.monstersandmemories.com/metrics'][target='_blank'][rel~='noopener']",
            "M&M’s public statistics",
        );
        if count == 0 {
            assert!(status.contains("No statistics collected yet"));
            self.expect_count("#history-status time", 0);
        } else {
            let first: chrono::DateTime<chrono::Utc> =
                records[0]["observed_at"].as_str().unwrap().parse().unwrap();
            let latest: chrono::DateTime<chrono::Utc> = records[count - 1]["observed_at"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            self.expect_text("#first-collection", &first.format("%d %b %Y").to_string());
            self.expect_text(
                "#latest-collection",
                &latest.format("%d %b %Y, %H:%M UTC").to_string(),
            );
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
        if count > 0 && self.count(".empty-selection") > 0 {
            self.expect_text("[data-summary='online'] .headline-value", "Not available");
            self.click(".empty-selection button");
            self.ready("online");
            self.expect_count(".empty-selection", 0);
        }
        self.verify_download(expected, scratch);
        self.verify_requests(url, true);
    }

    fn verify_presentation(&self, expected: &Value, scratch: &Path) {
        let coverage = self.text("#history-status");
        self.expect_count(".headline", 4);
        self.expect_count(".chart-card", 1);
        self.expect_count(".legend li", 1);
        self.expect_text(".legend li", "All Servers");
        self.expect_count(".legend li > span + .swatch", 1);
        let last = expected["snapshots"].as_array().unwrap().last().unwrap();
        let daily: u128 = last["servers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| u128::from(s["daily_active"].as_u64().unwrap()))
            .sum();
        self.expect_text(
            "[data-summary='daily'] .headline-value",
            &mnm_stats_dashboard::analysis::grouped_count(daily),
        );
        self.servers(&["retired"]);
        self.expect_text("[data-summary='daily'] .headline-value", "Not available");
        self.expect_text(
            "[data-summary='subscriptions'] .headline-value",
            &mnm_stats_dashboard::analysis::grouped_count(
                last["active_subscriptions"].as_u64().unwrap().into(),
            ),
        );
        self.servers(&["a"]);
        self.select("#time-range", "7");
        assert_eq!(self.text("#history-status"), coverage);
        self.click("#nav-population");
        self.expect_count(".chart-card", 3);
        self.select("#zone-scope", "zz-archived");
        self.expect_text(
            "[data-metric='zone-zz-archived'] .empty-chart",
            "No available observations for this selection.",
        );
        self.select("#time-range", "all");
        self.ready("zone-zz-archived");
        self.select("#time-range", "7");
        self.select("#zone-scope", "w");
        self.activate("#nav-activity");
        self.expect_count(".chart-card", 3);
        self.click("#nav-population");
        assert_eq!(self.request(Method::POST, "/execute/sync", json!({"script":"return [document.querySelector('#server-options input:checked').value,document.querySelector('#time-range').dataset.value,document.querySelector('#zone-scope').value];","args":[]})), json!(["server:a","7","w"]));
        self.servers(&["", "a", "b"]);
        self.expect_count("[data-metric='online'] .legend li", 3);
        // The checkbox dropdown supports keyboard toggling and Escape restores focus.
        self.activate("#servers-toggle");
        self.request(
            Method::POST,
            &format!(
                "/element/{}/value",
                self.element("#server-options input[value='server:b']")
            ),
            json!({"text":" ","value":[" "]}),
        );
        self.request(Method::POST, "/actions", json!({"actions":[{"type":"key","id":"keyboard","actions":[{"type":"keyDown","value":"\u{e00c}"},{"type":"keyUp","value":"\u{e00c}"}]}]}));
        self.expect_count("[data-metric='online'] .legend li", 2);
        assert_eq!(
            self.request(
                Method::POST,
                "/execute/sync",
                json!({"script":"return document.activeElement.id;","args":[]})
            ),
            "servers-toggle"
        );
        self.expect_count(".server-picker[open]", 0);
        self.toggle_server("server:b");
        self.click("#nav-overview");
        self.expect_count(".headline", 0);
        self.expect_count(".comparison-summary", 0);
        assert_eq!(self.text("#history-status"), coverage);
        self.servers(&["a"]);
        self.expect_count(".headline", 4);
        // Test chart-engine failure without a network request: data and download survive.
        self.request(Method::POST, "/execute/sync", json!({"script":"window.savedPlot = Plotly.newPlot; Plotly.newPlot = () => Promise.reject(new Error('test failure'));","args":[]}));
        self.click("#nav-activity");
        self.expect_text(
            "[data-metric='daily'] .interactive-plot .error",
            "Chart unavailable. Download the history or reload to retry the chart engine.",
        );
        self.click("#nav-overview");
        self.expect_count(".headline", 4);
        self.verify_download(expected, scratch);
        self.request(Method::POST, "/execute/sync", json!({"script":"Plotly.newPlot = window.savedPlot; delete window.savedPlot;","args":[]}));
        self.click("#nav-activity");
        self.click("#nav-overview");
        self.ready("online");
        // Keyboard skip link and section controls preserve a useful focus target.
        self.request(Method::POST, "/execute/sync", json!({"script":"window.scrollTo(0,0); document.querySelector('.skip-link').focus();","args":[]}));
        self.request(Method::POST, "/actions", json!({"actions":[{"type":"key","id":"keyboard","actions":[{"type":"keyDown","value":"\u{e007}"},{"type":"keyUp","value":"\u{e007}"}]}]}));
        assert_eq!(
            self.request(
                Method::POST,
                "/execute/sync",
                json!({"script":"return document.activeElement.id;","args":[]})
            ),
            "content"
        );
        for width in [320, 375] {
            self.request(
                Method::POST,
                "/window/rect",
                json!({"width":width,"height":812}),
            );
            assert_eq!(self.request(Method::POST, "/execute/sync", json!({"script":"return document.documentElement.scrollWidth <= innerWidth;","args":[]})), true);
        }
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":1280,"height":1000}),
        );
        self.request(
            Method::POST,
            "/execute/sync",
            json!({"script":"document.documentElement.style.fontSize='200%';","args":[]}),
        );
        assert_eq!(self.request(Method::POST, "/execute/sync", json!({"script":"return document.documentElement.scrollWidth <= innerWidth;","args":[]})), true);
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":320,"height":812}),
        );
        assert_eq!(self.request(Method::POST, "/execute/sync", json!({"script":"return document.documentElement.scrollWidth <= innerWidth;","args":[]})), true);
        assert_eq!(
            self.computed(".headline-grid", "grid-template-columns")
                .split_whitespace()
                .count(),
            1
        );
        self.request(
            Method::POST,
            "/execute/sync",
            json!({"script":"document.documentElement.style.fontSize='';","args":[]}),
        );
        self.servers(&[""]);
        self.select("#time-range", "30");
    }

    fn verify_features(&self, url: &str, expected: &Value, scratch: &Path) {
        self.verify_presentation(expected, scratch);
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
        self.expect_count(".chart-card", 1);
        self.click("#servers-toggle");
        assert!(self.text("#server-options").contains("Retired server"));
        self.click("#servers-toggle");
        let midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        for (key, days) in [
            ("today", None),
            ("yesterday", None),
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
                    match key {
                        "today" => at >= midnight && at <= now,
                        "yesterday" => at >= midnight - chrono::Duration::days(1) && at < midnight,
                        _ => {
                            at <= now && days.is_none_or(|d| at >= now - chrono::Duration::days(d))
                        }
                    }
                })
                .collect();
            for metric in metrics {
                let count = selected
                    .iter()
                    .filter(|r| {
                        !(metric.ends_with("-subscriptions") && r["active_subscriptions"] == 0)
                    })
                    .count();
                self.ready(metric);
                let plotted_count = self.request(Method::POST, "/execute/sync", json!({
                    "script":"return document.querySelector(arguments[0]).data.filter(trace => trace.hoverinfo !== 'skip').reduce((n, trace) => n + trace.y.filter(value => value !== null).length, 0);",
                    "args":[format!("[data-metric='{metric}'] .plot-surface")]
                }));
                assert_eq!(plotted_count, count, "{metric} in range {key}");
                self.expect_text(
                    &format!("[data-metric='{metric}'] .ytitle"),
                    if matches!(
                        metric,
                        "daily-monthly" | "daily-subscriptions" | "monthly-subscriptions"
                    ) {
                        "Percent (%)"
                    } else {
                        "Count"
                    },
                );
            }
        }
        // Hover details retain exact values and original observation timestamps.
        self.select("#time-range", "7");
        self.servers(&["a"]);
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
            self.hover(&format!("[data-metric='{metric}'] .chart-heading"));
            self.expect_count(".hovertext", 0);
            if metric.ends_with("-subscriptions") {
                assert_eq!(
                    self.request(Method::POST, "/execute/sync", json!({
                        "script":"const trace=document.querySelector(arguments[0]).data[0]; const i=trace.x.indexOf(Date.parse(arguments[1])/1000); return {present:i>=0,value:trace.y[i],hover:trace.text[i]};",
                        "args":[format!("[data-metric='{metric}'] .plot-surface"), "2026-05-30T13:00:00Z"]
                    })),
                    json!({"present":true,"value":null,"hover":""}),
                    "zero subscription denominators remain gaps without hover values"
                );
            }
        }
        self.hover("[data-metric='daily'] .scatterlayer .point");
        self.servers(&["retired"]);
        self.expect_count(".hovertext", 0);
        for metric in metrics.into_iter().filter(|m| *m != "subscriptions") {
            self.expect_text(
                &format!("[data-metric='{metric}'] .empty-chart"),
                "No available observations for this selection.",
            );
        }
        self.ready("subscriptions");
        self.servers(&["a"]);
        self.click("#nav-relationships");
        assert!(self.text(".correlation-scope").contains("Alpha"));
        self.expect_count(".correlation-value", 3);
        assert!(self.text(".correlations").contains("r ="));
        self.servers(&["", "a", "b"]);
        self.expect_count(".correlation-entity", 3);
        for metric in metrics {
            self.expect_count(
                &format!("[data-metric='{metric}'] .legend li"),
                if metric == "subscriptions" { 1 } else { 3 },
            );
        }
        self.hover("[data-metric='online'] .scatterlayer .trace:nth-child(3) .point:last-child");
        self.expect_hover(
            "online",
            "Beta",
            records.last().unwrap()["observed_at"].as_str().unwrap(),
            "5",
        );
        // Three individual servers, then all servers alongside the three individuals.
        self.toggle_server("all");
        self.toggle_server("server:c");
        self.expect_count("[data-metric='daily'] .legend li", 3);
        self.toggle_server("all");
        self.expect_count("[data-metric='daily'] .legend li", 4);
        self.verify_download(expected, scratch);
        self.toggle_server("server:b");
        self.expect_count("[data-metric='daily'] .legend li", 3);
        self.servers(&[]);
        self.expect_count(".empty-servers", 1);
        for metric in metrics {
            self.expect_count(&format!("[data-metric='{metric}'] .plot-surface"), 0);
        }
        self.verify_download(expected, scratch);
        self.click(".empty-servers button");
        self.expect_count(".empty-servers", 0);
        self.expect_text("#servers-selection", "All Servers");
        self.servers(&["a"]);
        self.select("#comparison-mode", "previous");
        self.expect_count("[data-metric='daily'] .legend li", 2);
        for key in ["today", "yesterday"] {
            self.select("#time-range", key);
            self.expect_count("[data-metric='daily'] .legend li", 2);
            self.ready("daily");
            assert!(self.text("#comparison-mode").contains("Previous period"));
        }
        self.select("#time-range", "30");
        assert!(self.text("#comparison-mode").contains("Previous period"));
        self.select("#comparison-mode", "year-over-year");
        self.expect_count("[data-metric='daily'] .legend li", 2);
        let exact_dates = self.text("[data-metric='daily'] .legend");
        self.click("#comparison-mode");
        self.activate("#match-weekday");
        self.expect_count(".date-menu[open]", 0);
        assert_ne!(self.text("[data-metric='daily'] .legend"), exact_dates);
        self.click("#comparison-mode");
        self.activate("#match-date");
        assert_eq!(self.text("[data-metric='daily'] .legend"), exact_dates);
        self.select("#comparison-mode", "disabled");
        self.expect_count("[data-metric='daily'] .legend li", 1);
        self.click("#time-range");
        self.activate("#toggle-comparison");
        self.expect_count("[data-metric='daily'] .legend li", 2);
        self.click("#time-range");
        self.activate("#toggle-comparison");
        self.expect_count("[data-metric='daily'] .legend li", 1);
        for (mode, periods) in [
            (
                "months",
                [
                    ("2024-02-01", "2024-02-29"),
                    ("2024-03-01", "2024-03-31"),
                    ("2024-04-01", "2024-04-30"),
                ],
            ),
            (
                "years",
                [
                    ("2023-01-01", "2023-12-31"),
                    ("2024-01-01", "2024-12-31"),
                    ("2025-01-01", "2025-12-31"),
                ],
            ),
        ] {
            self.custom_periods(&periods);
            for metric in metrics {
                self.expect_count(&format!("[data-metric='{metric}'] .legend li"), 3);
                self.ready(metric);
            }
            self.toggle_server("server:b");
            for metric in metrics {
                self.expect_count(
                    &format!("[data-metric='{metric}'] .legend li"),
                    if metric == "subscriptions" { 3 } else { 6 },
                );
            }
            self.expect_count(".correlation-entity", 2);
            self.toggle_server("server:b");
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
            self.ready("daily");
            assert_eq!(
                self.request(Method::POST, "/execute/sync", json!({
                    "script":"return document.querySelector('[data-metric=\"daily\"] .plot-surface').data.some(trace=>trace.text?.some(text=>text.includes(arguments[0])));",
                    "args":["29 Feb 2024, 23:00 UTC"]
                })),
                true,
                "comparison hover details retain the leap-day observation timestamp"
            );
            self.verify_download(expected, scratch);
            self.click("#comparison-mode");
            self.click(".remove-period");
            self.date("#compare-start", "2024-03-01");
            self.date("#compare-end", "2024-02-01");
            self.activate("#add-period");
            assert!(!self.text("#compare-error").is_empty());
            assert_eq!(
                self.request(
                    Method::GET,
                    &format!(
                        "/element/{}/attribute/aria-invalid",
                        self.element("#compare-start")
                    ),
                    Value::Null
                ),
                "true"
            );
            self.click("#comparison-mode");
            self.expect_count("[data-metric='daily'] .legend li", 2);
        }
        self.custom_periods(&[
            ("2024-02-01", "2024-02-01"),
            ("2024-03-01", "2024-03-01"),
            ("2024-04-01", "2024-04-02"),
        ]);
        for metric in metrics {
            self.expect_count(&format!("[data-metric='{metric}'] .legend li"), 3);
            self.ready(metric);
        }
        self.hover("[data-metric='zone-w'] .scatterlayer .point");
        assert!(
            self.text("[data-metric='zone-w'] .hoverlayer")
                .contains("01 Feb 2024, 00:00 UTC")
        );
        self.move_pointer("[data-metric='zone-w'] .plot-surface", 0, 0);
        self.expect_count(".hovertext", 0);
        self.click("#time-range");
        self.click("#custom-range");
        self.date("#range-end", "2024-01-01");
        self.activate("#apply-range");
        self.expect_text(
            "#range-error",
            "The end date must be on or after the start date.",
        );
        self.request(Method::POST, "/actions", json!({"actions":[{"type":"key","id":"keyboard","actions":[{"type":"keyDown","value":"\u{e00c}"},{"type":"keyUp","value":"\u{e00c}"}]}]}));
        self.expect_count(".date-menu[open]", 0);
        assert_eq!(
            self.request(
                Method::POST,
                "/execute/sync",
                json!({"script":"return document.activeElement.id;","args":[]})
            ),
            "time-range"
        );
        self.click("#comparison-mode");
        self.click("#nav-overview");
        self.expect_count(".date-menu[open]", 0);
        self.expect_count(".headline", 0);
        self.expect_count(".comparison-summary", 0);
        self.verify_download(expected, scratch);
        self.verify_requests(url, false);
        self.select("#comparison-mode", "disabled");
        // Capture the real browser for visual review; decoding is a separate local activity.
        self.servers(&[""]);
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
        self.servers(&["a"]);
        // Boundary markers remain reachable in a plot sized to the narrow container.
        self.hover("[data-metric='daily'] .scatterlayer .trace:first-child .point:last-child");
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
            "All metric families, all range presets, exact values, historical entities, three-period/entity comparisons, missing data, and filter-independent downloads verified."
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
        let displayed_time = chrono::DateTime::parse_from_rfc3339(at)
            .unwrap()
            .with_timezone(&chrono::Utc)
            .format("%d %b %Y, %H:%M UTC")
            .to_string();
        wait_until(
            || {
                let rows = self.request(Method::POST, "/execute/sync", json!({
                "script":"return Array.from(document.querySelectorAll(arguments[0]), row => row.textContent);",
                "args":[selector]
            }));
                rows.as_array().unwrap().iter().any(|row| {
                    let text = row.as_str().unwrap();
                    text.starts_with(&format!("{value} {series}"))
                        && text.ends_with(&displayed_time)
                })
            },
            &format!("{metric}: hover must show {series}, {at}, {value}"),
        );
    }

    fn computed(&self, selector: &str, property: &str) -> String {
        self.visit_chart_selector(selector);
        let mut value = None;
        wait_until(
            || {
                value = self.request(Method::POST, "/execute/sync", json!({
                "script":"const e=document.querySelector(arguments[0]);return e ? getComputedStyle(e).getPropertyValue(arguments[1]) : null;",
                "args":[selector,property]
            })).as_str().map(str::to_owned);
                value.is_some()
            },
            &format!("computed {property} of {selector}"),
        );
        let value = value.unwrap();
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
        self.select("#comparison-mode", "disabled");
        // Seven days keeps demo observations sparse enough to draw point markers.
        self.select("#time-range", "7");
        self.servers(&[""]);
        self.show_metric("daily");
        self.ready("daily");
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":1280,"height":1000}),
        );
        assert_eq!(
            self.computed("html", "background-color"),
            if changed {
                "rgb(231, 229, 228)"
            } else {
                "rgb(245, 245, 244)"
            }
        );
        assert_eq!(self.computed(".chart-card", "border-radius"), "12px");
        assert_eq!(
            self.computed(".legend", "font-size"),
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
        assert!((size - if changed { 18.0 } else { 14.0 }).abs() < 0.001);
        let color = if changed {
            "rgb(159, 45, 0)"
        } else {
            "rgb(0, 120, 111)"
        };
        assert_eq!(
            self.computed("[data-metric='daily'] .scatterlayer .point", "fill"),
            color
        );
        assert_eq!(
            self.computed("[data-metric='daily'] .scatterlayer .point", "fill-opacity"),
            "1"
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
        assert_eq!(self.computed(".plot-surface", "min-width"), "0px");
    }

    fn verify_extended_palette(&self) {
        self.request(
            Method::POST,
            "/window/rect",
            json!({"width":1280,"height":1000}),
        );
        self.custom_periods(&[
            ("2023-02-01", "2023-02-28"),
            ("2023-03-01", "2023-03-31"),
            ("2023-04-01", "2023-04-30"),
            ("2024-02-01", "2024-02-29"),
            ("2024-03-01", "2024-03-31"),
            ("2024-04-01", "2024-04-30"),
            ("2025-02-01", "2025-02-28"),
        ]);
        self.expect_count("[data-metric='daily'] .legend li", 7);
        for i in 0..7 {
            let swatch = format!(
                "[data-metric='daily'] .legend li:nth-child({}) .swatch line",
                i + 1
            );
            // Each selected fixture month has exactly three observations.
            let circle = format!(
                "[data-metric='daily'] .scatterlayer .trace:nth-child({}) .point",
                i + 1
            );
            let color = self.computed(&swatch, "stroke");
            assert_eq!(self.computed(&circle, "fill"), color);
            let line = format!(
                "[data-metric='daily'] .scatterlayer .trace:nth-child({}) .js-line",
                i + 1
            );
            assert_eq!(self.computed(&swatch, "stroke-dasharray"), "none");
            assert_eq!(self.computed(&line, "stroke-dasharray"), "none");
            let label = self.text(&format!(
                "[data-metric='daily'] .legend li:nth-child({})",
                i + 1
            ));
            self.hover(&circle);
            wait_until(
                || {
                    self.request(Method::POST, "/execute/sync", json!({
                    "script":"const row = Array.from(document.querySelectorAll(arguments[0])).find(e => e.textContent.includes(' ' + arguments[1])); return row ? getComputedStyle(row.querySelector('path')).stroke : null;",
                    "args":["[data-metric='daily'] .hovertext", label]
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
        self.select("#comparison-mode", "disabled");
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
                request == url
                    || request.ends_with("/fonts/IMFellEnglish-Regular.ttf")
                    || [".js", ".wasm"].iter().any(|ext| request.ends_with(ext)),
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
    let midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    times.extend([midnight - Duration::seconds(1), midnight, now]);
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
        if i == 0 { servers[0]["starting_zones"].as_array_mut().unwrap().push(json!({"id":"zz-archived","name":"An old starting zone retained only in the archive","online":1})); }
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
    invalid["chart"]["series"]["palette"]["01"]["$value"] = json!("{tailwind.color.absent.700}");
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
        &fs::read_to_string(root.join("mnm-stats/mnm-stats-dashboard/design-tokens.tokens.json"))
            .unwrap(),
    )
    .unwrap();
    tokens["color"]["surface"]["page"]["$value"] = json!("{tailwind.color.stone.200}");
    tokens["chart"]["series"]["palette"]["01"]["$value"] = json!("{tailwind.color.orange.800}");
    tokens["chart"]["axis"]["label"]["font-size"]["$value"]["value"] = json!(18);
    tokens["breakpoint"]["medium"]["$value"]["value"] = json!(900);
    // Shared primitives update their aliases; an input-only override stays local.
    tokens["scale"]["spacing"]["3"]["$value"] = json!("{tailwind.spacing.4}");
    tokens["scale"]["font-size"]["2"]["$value"] = json!("{tailwind.text.base}");
    tokens["spacing"]["input"]["padding"]["$value"] = json!("{tailwind.spacing.2}");
    tokens
}

fn verify_preview(root: &Path, scratch: &Path, browser: &Browser) {
    let tokens = scratch.join("preview-tokens.json");
    fs::copy(
        root.join("mnm-stats/mnm-stats-dashboard/design-tokens.tokens.json"),
        &tokens,
    )
    .unwrap();
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
    assert_eq!(parsed.snapshots().len(), 362);
    let end = parsed.snapshots().last().unwrap().observed_at;
    let intervals: std::collections::BTreeSet<_> = parsed
        .snapshots()
        .windows(2)
        .filter(|pair| pair[0].observed_at >= end - chrono::Duration::days(7))
        .map(|pair| (pair[1].observed_at - pair[0].observed_at).num_hours())
        .collect();
    assert_eq!(intervals, [1, 3, 6, 24, 30].into_iter().collect());
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
    browser.ready("online");
    let solid = "[data-metric='online'] .scatterlayer .trace:nth-child(1) .js-line";
    let subdued = "[data-metric='online'] .scatterlayer .trace:nth-child(2) .js-line";
    assert_eq!(browser.computed(solid, "stroke-dasharray"), "none");
    assert_eq!(browser.computed(subdued, "stroke-dasharray"), "none");
    assert_ne!(
        browser.computed(solid, "stroke"),
        browser.computed(subdued, "stroke")
    );
    assert_eq!(
        browser.computed(solid, "stroke-width"),
        browser.computed(subdued, "stroke-width")
    );
    browser.request(Method::POST, "/execute/async", json!({"script":"const done=arguments[0]; document.fonts.ready.then(() => {window.scrollTo(0,0); done(null);});","args":[]}));
    let first_view = browser.request(Method::POST, "/execute/sync", json!({"script":"return {cards:document.querySelector('.headline-grid').getBoundingClientRect().bottom, chart:document.querySelector('.plot-surface').getBoundingClientRect().top, width:document.documentElement.scrollWidth, viewport:innerWidth};","args":[]}));
    assert!(
        first_view["cards"].as_f64().unwrap() < 812.0
            && first_view["chart"].as_f64().unwrap() < 812.0,
        "mobile first view: {first_view}"
    );
    assert!(first_view["width"].as_u64().unwrap() <= first_view["viewport"].as_u64().unwrap());
    browser.servers(&["demo-0"]);
    browser.request(
        Method::POST,
        "/execute/sync",
        json!({"script":"window.scrollTo(0,0);","args":[]}),
    );
    assert_eq!(browser.request(Method::POST, "/execute/sync", json!({"script":"return document.querySelector('.plot-surface').getBoundingClientRect().top < 812 && document.documentElement.scrollWidth <= innerWidth;","args":[]})), true, "long source names fit the populated mobile overview");
    browser.servers(&[""]);

    browser.request(
        Method::POST,
        "/window/rect",
        json!({"width":1440,"height":900}),
    );
    browser.ready("online");
    assert_eq!(browser.request(Method::POST, "/execute/sync", json!({"script":"return document.querySelector('.scatterlayer').getBoundingClientRect().top < 900;","args":[]})), true);

    // The final subdued section ends 12 hours before the latest demo sample.
    // Its endpoint belongs to both SVG traces, but must show one actual observation.
    let endpoint = parsed
        .snapshots()
        .iter()
        .find(|s| s.observed_at == end - chrono::Duration::hours(12))
        .unwrap();
    let endpoint_total: u64 = endpoint.servers.iter().map(|s| s.online).sum();
    let point = browser.request(Method::POST, "/execute/async", json!({
        "script":r#"const done=arguments[arguments.length-1]; const line=Array.from(document.querySelectorAll("[data-metric='online'] .scatterlayer .trace:nth-child(2) .js-line")).at(-1); line.scrollIntoView({block:'center'}); requestAnimationFrame(() => {const p=line.getPointAtLength(line.getTotalLength()).matrixTransform(line.getScreenCTM()); done([p.x,p.y]);});"#,
        "args":[]
    }));
    browser.pointer_at(&point);
    browser.expect_hover(
        "online",
        "All Servers",
        &endpoint.observed_at.to_rfc3339(),
        &endpoint_total.to_string(),
    );
    browser.expect_count("[data-metric='online'] .hovertext", 1);

    assert_eq!(browser.count(".plot-surface[data-ready='true']"), 1);
    drop(demo_preview);
    let preview = start_preview(root, scratch, Some(&history), "explicit", preview_port);
    let expected: Value = serde_json::from_str(&parsed.to_json().unwrap()).unwrap();
    browser.verify(&origin, &expected, scratch);
    assert_eq!(
        browser.count(".plot-surface[data-ready='true']"),
        1,
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
        "script":r#"const done = arguments[arguments.length-1]; const line = Array.from(document.querySelectorAll("[data-metric='daily'] .scatterlayer .trace:first-child .js-line")).at(-1); line.scrollIntoView({block:'center',inline:'end'}); requestAnimationFrame(() => requestAnimationFrame(() => { const p = line.getPointAtLength(line.getTotalLength()).matrixTransform(line.getScreenCTM()); done([p.x+1,p.y]); }));"#,
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
    browser.expect_hover("daily", "All Servers", at, &total.to_string());
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
    fs::copy(
        root.join("mnm-stats/mnm-stats-dashboard/design-tokens.tokens.json"),
        &replacement,
    )
    .unwrap();
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
    assert_eq!(browser.count(".plot-surface[data-ready='true']"), 1);
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
