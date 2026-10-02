use std::{fs, path::PathBuf, process::Command};

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
        .arg("collect")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("usage:"));
    fs::remove_dir_all(&scratch).unwrap();
}
