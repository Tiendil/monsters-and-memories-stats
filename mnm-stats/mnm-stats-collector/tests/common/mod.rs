#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

pub const CONNECTED: &str = include_str!("../fixtures/metrics-connected.html");
pub const INITIAL: &str = include_str!("../fixtures/metrics-initial.html");
pub fn time() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-10-02T15:20:00Z")
        .unwrap()
        .to_utc()
}
pub fn frames() -> Vec<Value> {
    serde_json::from_str(include_str!("../fixtures/liveview.json")).unwrap()
}

// Synthetic completed rows reproducing the four-zone shape seen in the incident.
// These counts are local test inputs, not a refresh of the captured fixture.
pub const NUNAVOTH_ZONES: [(&str, &str, &str); 4] = [
    ("evergrove", "Evershade Weald", "10"),
    ("nightharbore", "Night Harbor (East)", "18"),
    ("nightharborw", "Night Harbor (West)", "27"),
    ("underdocks", "Underdocks", "16"),
];

pub fn with_nunavoth_zones(zones: &[(&str, &str, &str)], total: &str) -> Vec<Value> {
    let mut frames = frames();
    let rows = frames.last_mut().unwrap()[4]["0"]["1"]["0"]["1"]["0"]["d"]
        .as_array_mut()
        .unwrap();
    let server = rows.iter_mut().find(|row| row[0] == "nunavoth").unwrap();
    server[12] = Value::from(total);
    server[14] = Value::from(
        zones.iter().map(|(id, name, count)| {
            format!("<div id=\"starting-zone-nunavoth-{id}\"><span>{name}</span><span>{count}</span></div>")
        }).collect::<String>(),
    );
    frames
}

pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../.session/tests/collector-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
