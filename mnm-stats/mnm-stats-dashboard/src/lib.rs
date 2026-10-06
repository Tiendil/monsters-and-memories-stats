//! The same shared observations are used by the UI and its complete-history export.
include!(concat!(env!("OUT_DIR"), "/history.rs"));

pub mod analysis;
pub mod charts;
pub mod view_state;

#[allow(dead_code)]
pub mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}

#[cfg(target_arch = "wasm32")]
pub mod app;
