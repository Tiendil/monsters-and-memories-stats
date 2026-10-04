use std::{fs, path::PathBuf, process::Command};
mod common;

#[test]
fn changing_zone_lists_preserve_history_and_invalid_rows_never_append() {
    let scratch = common::Scratch::new();
    let history = scratch.0.join("history.jsonl");
    let fixture = scratch.0.join("frames.json");
    let run = |frames: Vec<serde_json::Value>, at: &str| {
        fs::write(&fixture, serde_json::to_string(&frames).unwrap()).unwrap();
        Command::new(env!("CARGO_BIN_EXE_mnm-stats-collector"))
            .arg("collect")
            .arg("--history")
            .arg(&history)
            .arg("--replay")
            .arg(&fixture)
            .arg("--observed-at")
            .arg(at)
            .output()
            .unwrap()
    };
    assert!(
        run(common::frames(), "2026-10-02T15:20:00Z")
            .status
            .success()
    );
    let original = fs::read_to_string(&history).unwrap();
    let output = run(
        common::with_nunavoth_zones(&common::NUNAVOTH_ZONES, "71"),
        "2026-10-02T16:20:00Z",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let before = fs::read_to_string(&history).unwrap();
    assert!(before.starts_with(&original));
    let stored = mnm_stats_model::History::from_jsonl(&before).unwrap();
    assert_eq!(stored.snapshots().len(), 2);
    let zones: Vec<_> = stored
        .snapshots()
        .iter()
        .map(|s| {
            &s.servers
                .iter()
                .find(|s| s.id == "nunavoth")
                .unwrap()
                .starting_zones
        })
        .collect();
    assert_eq!((zones[0].len(), zones[1].len()), (5, 4));
    assert!(zones[1].iter().all(|z| z.id != "ailvorith"));
    for frames in [
        common::with_nunavoth_zones(&[("duplicate", "One", "0"), ("duplicate", "Two", "0")], "0"),
        common::with_nunavoth_zones(&[("", "Empty identity", "0")], "0"),
        common::with_nunavoth_zones(&[("zone", "Zone", "broken")], "0"),
        common::with_nunavoth_zones(&common::NUNAVOTH_ZONES, "72"),
        common::with_nunavoth_zones(&[], "1"),
    ] {
        let output = run(frames, "2026-10-02T17:20:00Z");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("server \"nunavoth\""));
        assert_eq!(fs::read_to_string(&history).unwrap(), before);
    }
}

#[test]
fn notification_probe_fails_with_local_fixture_without_changing_repository_history() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let history = project.join("data/history.jsonl");
    let before = fs::read(&history).unwrap();
    let result = Command::new(project.join("bin/verify-collection-failure.sh"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("Intentional notification verification"));
    assert!(error.contains("replay"), "{error}");
    assert_eq!(fs::read(history).unwrap(), before);
}

#[test]
fn latest_observed_at_uses_the_complete_validated_history() {
    let scratch = common::Scratch::new();
    let path = scratch.0.join("history.jsonl");
    let record = |timestamp| {
        format!(
            "{}\n",
            serde_json::json!({
                "schema_version": 1,
                "observed_at": timestamp,
                "active_subscriptions": 0,
                "servers": []
            })
        )
    };
    let first = record("2001-02-03T04:05:06Z");
    let latest = record("2001-02-03T05:06:07.123456789+00:00");
    for (contents, expected) in [
        (String::new(), Some("")),
        (first.clone(), Some("2001-02-03T04:05:06Z\n")),
        (
            format!("{first}{latest}"),
            Some("2001-02-03T05:06:07.123456789Z\n"),
        ),
        (format!("{{broken\n{latest}"), None),
    ] {
        fs::write(&path, &contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_mnm-stats-collector"))
            .arg("validate-history")
            .arg(&path)
            .arg("--latest-observed-at")
            .output()
            .unwrap();
        assert_eq!(output.status.success(), expected.is_some());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected.unwrap_or("")
        );
        if expected.is_none() {
            assert!(String::from_utf8(output.stderr).unwrap().contains("line 1"));
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
    }
}

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
    let mut invalid_zones = common::frames();
    let rows = invalid_zones.last_mut().unwrap()[4]["0"]["1"]["0"]["1"]["0"]["d"]
        .as_array_mut()
        .unwrap();
    rows.iter_mut().find(|row| row[0] == "vespyra").unwrap()[14] =
        serde_json::json!("<p>Starting zones unavailable</p>");
    for frames in [
        "{broken".to_owned(),
        serde_json::to_string(&common::frames()[..1]).unwrap(),
        serde_json::to_string(&common::frames())
            .unwrap()
            .replace("DAILY ACTIVE", "DAILY USERS"),
        serde_json::to_string(&invalid_zones).unwrap(),
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
