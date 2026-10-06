//! Exercise orchestration with a Git command double; never operate on a Git repository.
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn publishing_is_limited_to_data_and_preserves_failures_and_noops() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let scratch = root.join(format!(
        ".session/tests/publish-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(scratch.join("bin")).unwrap();
    fs::create_dir_all(scratch.join("checkout/data")).unwrap();
    let history = scratch.join("checkout/data/history.jsonl");
    fs::write(&history, "{\"schema_version\":1,\"observed_at\":\"2026-10-06T01:17:00.123Z\",\"active_subscriptions\":0,\"servers\":[]}\n").unwrap();
    let git = scratch.join("bin/git");
    fs::write(&git, r#"#!/usr/bin/env bash
set -eu
printf '%s\n' "$*" >> "$TEST_GIT_LOG"
case "$*" in
    'rev-parse --show-toplevel') pwd ;;
    'symbolic-ref --short HEAD') printf '%s\n' "$TEST_BRANCH" ;;
    'diff --quiet HEAD -- data/history.jsonl') exit "$TEST_DIFF" ;;
    'push origin HEAD:refs/heads/data') exit "$TEST_PUSH" ;;
    *'commit --only -m collector: record snapshot at 2026-10-06T01:17:00.123Z -- data/history.jsonl') ;;
    *) echo "Unexpected Git invocation: $*" >&2; exit 90 ;;
esac
"#).unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    let log = scratch.join("git.log");
    let path = format!(
        "{}:{}",
        scratch.join("bin").display(),
        std::env::var("PATH").unwrap()
    );
    for (branch, diff, push, success, commits, pushes) in [
        ("data", "1", "0", true, true, true),
        ("data", "0", "0", true, false, false),
        ("data", "2", "0", false, false, false),
        ("main", "1", "0", false, false, false),
        ("data", "1", "1", false, true, true),
    ] {
        fs::write(&log, "").unwrap();
        let output = Command::new(root.join("bin/publish-history.sh"))
            .arg(scratch.join("checkout"))
            .env("PATH", &path)
            .env("TEST_GIT_LOG", &log)
            .env("TEST_BRANCH", branch)
            .env("TEST_DIFF", diff)
            .env("TEST_PUSH", push)
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let calls = fs::read_to_string(&log).unwrap();
        assert_eq!(calls.contains("commit --only"), commits, "{calls}");
        assert_eq!(
            calls.contains("push origin HEAD:refs/heads/data"),
            pushes,
            "{calls}"
        );
        assert!(!calls.contains("--force"));
    }
}
