mod common;

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn checked(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn git(repo: &Path, args: &[&str]) -> String {
    checked(
        Command::new("git")
            .current_dir(repo)
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
            ])
            .args(args)
            .output()
            .unwrap(),
    )
}

fn project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn prepare(scratch: &Path) -> (PathBuf, PathBuf) {
    let remote = scratch.join("remote.git");
    git(
        scratch,
        &["init", "--bare", "--initial-branch=main", "remote.git"],
    );
    let checkout = scratch.join("checkout");
    git(scratch, &["clone", remote.to_str().unwrap(), "checkout"]);
    fs::create_dir(checkout.join("data")).unwrap();
    fs::write(checkout.join("data/history.jsonl"), "").unwrap();
    git(&checkout, &["add", "data/history.jsonl"]);
    git(&checkout, &["commit", "-m", "Initial history"]);
    git(&checkout, &["push", "origin", "HEAD:main"]);
    (checkout, remote)
}

fn collect(repo: &Path) {
    checked(
        Command::new(env!("CARGO_BIN_EXE_mnm-stats-collector"))
            .args(["collect", "--history"])
            .arg(repo.join("data/history.jsonl"))
            .arg("--replay")
            .arg(project().join("mnm-stats/mnm-stats-collector/tests/fixtures/liveview.json"))
            .args(["--observed-at", "2026-10-02T15:20:00Z"])
            .output()
            .unwrap(),
    );
}

fn publish(repo: &Path) -> Output {
    Command::new(project().join("bin/publish-history.sh"))
        .current_dir(repo)
        .arg("main")
        .output()
        .unwrap()
}

#[test]
fn publication_commits_only_history_and_skips_unchanged_data() {
    let scratch = common::Scratch::new();
    let (repo, remote) = prepare(&scratch.0);
    collect(&repo);
    let history = fs::read_to_string(repo.join("data/history.jsonl")).unwrap();
    assert_eq!(history.lines().count(), 1);
    fs::write(repo.join("unrelated.txt"), "Keep this staged edit").unwrap();
    git(&repo, &["add", "unrelated.txt"]);

    checked(publish(&repo));
    assert_eq!(
        git(&remote, &["show", "main:data/history.jsonl"]),
        history.trim()
    );
    assert_eq!(
        git(
            &repo,
            &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]
        ),
        "data/history.jsonl"
    );
    assert_eq!(
        git(&repo, &["diff", "--cached", "--name-only"]),
        "unrelated.txt"
    );
    let head = git(&repo, &["rev-parse", "HEAD"]);
    assert!(checked(publish(&repo)).contains("History unchanged"));
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&remote, &["rev-parse", "main"]), head);
}

#[test]
fn publication_rejects_invalid_history_before_committing() {
    let scratch = common::Scratch::new();
    let (repo, remote) = prepare(&scratch.0);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    fs::write(repo.join("data/history.jsonl"), "{broken\n").unwrap();
    let result = publish(&repo);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("line 1"));
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&remote, &["rev-parse", "main"]), head);
    assert_eq!(
        fs::read_to_string(repo.join("data/history.jsonl")).unwrap(),
        "{broken\n"
    );
}

#[test]
fn rejected_push_preserves_remote_changes_and_local_observation() {
    let scratch = common::Scratch::new();
    let (repo, remote) = prepare(&scratch.0);
    git(
        &scratch.0,
        &["clone", remote.to_str().unwrap(), "concurrent"],
    );
    let concurrent = scratch.0.join("concurrent");
    fs::write(concurrent.join("code.txt"), "Concurrent code update").unwrap();
    git(&concurrent, &["add", "code.txt"]);
    git(&concurrent, &["commit", "-m", "Concurrent update"]);
    git(&concurrent, &["push", "origin", "HEAD:main"]);
    let remote_head = git(&remote, &["rev-parse", "main"]);
    collect(&repo);
    let history = fs::read_to_string(repo.join("data/history.jsonl")).unwrap();

    let result = publish(&repo);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("History push failed"));
    assert_eq!(git(&remote, &["rev-parse", "main"]), remote_head);
    assert_eq!(
        git(&remote, &["show", "main:code.txt"]),
        "Concurrent code update"
    );
    assert_eq!(git(&remote, &["show", "main:data/history.jsonl"]), "");
    assert_eq!(
        git(&repo, &["show", "HEAD:data/history.jsonl"]),
        history.trim()
    );
    assert_eq!(
        fs::read_to_string(repo.join("data/history.jsonl")).unwrap(),
        history
    );
}

#[test]
fn notification_probe_fails_with_local_fixture_without_changing_repository_history() {
    let history = project().join("data/history.jsonl");
    let before = fs::read(&history).unwrap();
    let result = Command::new(project().join("bin/verify-collection-failure.sh"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("Intentional notification verification"));
    assert!(error.contains("replay"), "{error}");
    assert_eq!(fs::read(history).unwrap(), before);
}
