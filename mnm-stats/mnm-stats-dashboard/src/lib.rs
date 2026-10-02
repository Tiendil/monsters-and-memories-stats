//! The same shared observations are used by the UI and its complete-history export.
include!(concat!(env!("OUT_DIR"), "/history.rs"));

pub mod analysis;
pub mod charts;

#[cfg(target_arch = "wasm32")]
pub mod app;

pub fn is_stale(
    observed_at: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    now.signed_duration_since(observed_at) > chrono::Duration::hours(3)
}
