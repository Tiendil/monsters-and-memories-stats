use chrono::{DateTime, SecondsFormat, Utc};
use mnm_stats_model::History;
use serde_json::json;
use std::{error::Error, fs, path::Path, time::SystemTime};

fn revision(value: &str) -> Result<Option<&str>, Box<dyn Error>> {
    if value.is_empty() {
        return Ok(None);
    }
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("build revisions must be complete hexadecimal commit IDs".into());
    }
    Ok(Some(value))
}

pub fn package(
    input: &Path,
    staging: &Path,
    code_revision: &str,
    data_revision: &str,
) -> Result<(), Box<dyn Error>> {
    let code_revision = revision(code_revision)?;
    let data_revision = revision(data_revision)?;
    let source = fs::read_to_string(input)?;
    History::from_jsonl(&source)?;
    let built_at: DateTime<Utc> = SystemTime::now().into();
    let metadata = json!({
        "code_revision": code_revision,
        "data_revision": data_revision,
        "built_at": built_at.to_rfc3339_opts(SecondsFormat::Secs, true),
    });
    // Trunk promotes staging only after every hook succeeds.
    fs::write(staging.join("history.jsonl"), source)?;
    fs::write(staging.join("build-info.json"), format!("{metadata:#}\n"))?;
    Ok(())
}
