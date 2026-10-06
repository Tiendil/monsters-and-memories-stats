#[path = "../build_publication.rs"]
mod publication;

use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn scratch() -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../.session/tests/publication-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn packages_exact_history_and_current_revision_pair_without_stale_metadata() {
    let scratch = scratch();
    let input = scratch.join("input history.jsonl");
    let staging = scratch.join("staging");
    fs::create_dir(&staging).unwrap();
    let code = "0123456789abcdef0123456789abcdef01234567";
    let data = "fedcba9876543210fedcba9876543210fedcba98";
    for (source, code, data) in [
        (
            "{\"schema_version\":1, \"observed_at\":\"2026-10-06T01:17:00.123Z\", \"active_subscriptions\":0, \"servers\":[]}\n",
            code,
            data,
        ),
        ("", data, code),
        ("", "", ""),
    ] {
        fs::write(&input, source).unwrap();
        publication::package(&input, &staging, code, data).unwrap();
        assert_eq!(
            fs::read_to_string(staging.join("history.jsonl")).unwrap(),
            source
        );
        let metadata: Value =
            serde_json::from_slice(&fs::read(staging.join("build-info.json")).unwrap()).unwrap();
        assert_eq!(
            metadata["code_revision"],
            if code.is_empty() {
                Value::Null
            } else {
                json!(code)
            }
        );
        assert_eq!(
            metadata["data_revision"],
            if data.is_empty() {
                Value::Null
            } else {
                json!(data)
            }
        );
        assert!(metadata["built_at"].as_str().unwrap().ends_with('Z'));
        assert!(
            metadata["built_at"]
                .as_str()
                .unwrap()
                .parse::<chrono::DateTime<chrono::Utc>>()
                .is_ok()
        );
    }
}

#[test]
fn rejects_invalid_inputs_before_replacing_published_files() {
    let scratch = scratch();
    let input = scratch.join("input.jsonl");
    let staging = scratch.join("staging");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("history.jsonl"), "previous archive").unwrap();
    fs::write(staging.join("build-info.json"), "previous metadata").unwrap();
    for (source, code, data) in [
        ("{invalid", "", ""),
        ("", "main", ""),
        ("", "", "bad\"revision"),
    ] {
        fs::write(&input, source).unwrap();
        assert!(publication::package(&input, &staging, code, data).is_err());
        assert_eq!(
            fs::read_to_string(staging.join("history.jsonl")).unwrap(),
            "previous archive"
        );
        assert_eq!(
            fs::read_to_string(staging.join("build-info.json")).unwrap(),
            "previous metadata"
        );
    }
}
