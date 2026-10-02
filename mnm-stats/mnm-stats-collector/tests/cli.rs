use std::{fs, path::PathBuf, process::Command};
mod common;

#[test]
fn validates_local_inputs_and_never_changes_them() {
    let scratch = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../.session/tests/collector-{}",
        std::process::id()
    ));
    fs::create_dir_all(&scratch).unwrap();
    let path = scratch.join("history.jsonl");
    for (contents, success, diagnostic) in [("", true, "0 observations"), ("{", false, "line 1")] {
        fs::write(&path, contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_mnm-stats-collector"))
            .arg("validate-history")
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.success(), success);
        let message = if success {
            output.stdout
        } else {
            output.stderr
        };
        assert!(String::from_utf8(message).unwrap().contains(diagnostic));
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
    }
    let output = Command::new(env!("CARGO_BIN_EXE_mnm-stats-collector"))
        .arg("validate-history")
        .arg(scratch.join("missing.jsonl"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let output = Command::new(env!("CARGO_BIN_EXE_mnm-stats-collector"))
        .arg("unknown-command")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("usage:"));
    fs::remove_dir_all(&scratch).unwrap();
}

#[test]
fn replay_cli_appends_once_and_preserves_history_on_failures() {
    let scratch = common::Scratch::new();
    let history = scratch.0.join("history.jsonl");
    let fixture = scratch.0.join("frames.json");
    fs::write(&fixture, serde_json::to_string(&common::frames()).unwrap()).unwrap();
    let run = |time: &str| {
        Command::new(env!("CARGO_BIN_EXE_mnm-stats-collector"))
            .arg("collect")
            .arg("--history")
            .arg(&history)
            .arg("--replay")
            .arg(&fixture)
            .arg("--observed-at")
            .arg(time)
            .output()
            .unwrap()
    };
    let output = run("2026-10-02T15:20:00Z");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = fs::read_to_string(&history).unwrap();
    assert_eq!(first.lines().count(), 1);
    assert!(run("2026-10-02T15:59:00Z").status.success());
    assert_eq!(fs::read_to_string(&history).unwrap(), first);
    assert!(run("2026-10-02T17:20:00Z").status.success());
    let before = fs::read_to_string(&history).unwrap();
    assert_eq!(before.lines().count(), 2);
    assert!(before.starts_with(&first));
    for frames in [
        "{broken".to_owned(),
        serde_json::to_string(&common::frames()[..1]).unwrap(),
        serde_json::to_string(&common::frames())
            .unwrap()
            .replace("DAILY ACTIVE", "DAILY USERS"),
    ] {
        fs::write(&fixture, frames).unwrap();
        assert!(!run("2026-10-02T18:20:00Z").status.success());
        assert_eq!(fs::read_to_string(&history).unwrap(), before);
    }
    fs::remove_file(&fixture).unwrap();
    assert!(run("2026-10-02T17:59:00Z").status.success());
    assert_eq!(fs::read_to_string(&history).unwrap(), before);
    assert!(!run("2026-10-02T18:20:00Z").status.success());
    assert_eq!(fs::read_to_string(&history).unwrap(), before);
    fs::write(&history, "{corrupt").unwrap();
    assert!(!run("2026-10-02T18:20:00Z").status.success());
    assert_eq!(fs::read_to_string(&history).unwrap(), "{corrupt");
}
