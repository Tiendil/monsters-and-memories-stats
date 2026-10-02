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
