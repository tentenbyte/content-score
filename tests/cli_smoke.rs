use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn init_creates_local_project() {
    let temp = tempdir().unwrap();

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized"));

    assert!(temp.path().join(".content-score/content.sqlite").exists());
    assert!(temp.path().join(".content-score/rubric.toml").exists());
}
