//! Current-state collection from the public metrics LiveView.

pub mod acquisition;
mod liveview;
pub mod parser;
pub mod storage;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub const SOURCE: &str = "https://account.monstersandmemories.com/metrics";
