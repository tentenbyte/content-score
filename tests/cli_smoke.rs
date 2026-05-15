use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
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

#[test]
fn score_and_candidates_work() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    fs::create_dir_all(temp.path().join("scripts")).unwrap();
    fs::write(
        temp.path().join("scripts/foo.md"),
        "第七页 PPT 上突然出现一只加油猫猫。",
    )
    .unwrap();

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args([
            "score",
            "scripts/foo.md",
            "--scores",
            "ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("composite: 6.29 / 10"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args([
            "candidates",
            "add",
            "AI makes one-person companies possible",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("candidate #1"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args([
            "candidates",
            "score",
            "1",
            "--scores",
            "ER=3,HP=4,QL=3,NA=2,AB=5,SR=4,SAT=1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("candidate_score"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["candidates", "top"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "AI makes one-person companies possible",
        ))
        .stdout(predicate::str::contains("candidate_score"));
}

#[test]
fn predict_and_retro_work() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    fs::create_dir_all(temp.path().join("scripts")).unwrap();
    fs::write(
        temp.path().join("scripts/foo.md"),
        "第七页 PPT 上突然出现一只加油猫猫。",
    )
    .unwrap();

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args([
            "predict",
            "scripts/foo.md",
            "--scores",
            "ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1",
            "--bet",
            "strong hook, weak satire",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("prediction"));

    let prediction_path = fs::read_dir(temp.path().join("predictions"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(prediction_path.exists());
    let prediction_id = prediction_path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args([
            "retro",
            &prediction_id,
            "--plays",
            "1200",
            "--likes",
            "80",
            "--comments",
            "12",
            "--shares",
            "4",
            "--saves",
            "9",
            "--notes",
            "solid base",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("retro recorded"));
}
