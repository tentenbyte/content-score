use assert_cmd::Command;
use predicates::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
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

#[test]
fn calibrate_and_upgrade_work() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    fs::create_dir_all(temp.path().join("scripts")).unwrap();
    let samples = [
        (
            "a.md",
            "强情绪开头，猫猫救场。",
            "ER=5,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1",
            5000,
        ),
        (
            "b.md",
            "社会议题明确，但情绪较弱。",
            "ER=2,HP=3,QL=2,NA=3,AB=4,SR=5,SAT=2",
            900,
        ),
        (
            "c.md",
            "普通经验分享，开头一般。",
            "ER=3,HP=2,QL=2,NA=3,AB=3,SR=2,SAT=1",
            600,
        ),
    ];

    for (file_name, body, scores, plays) in samples {
        let script_path = format!("scripts/{file_name}");
        fs::write(temp.path().join(&script_path), body).unwrap();
        let before = prediction_files(temp.path());
        Command::cargo_bin("content-score")
            .unwrap()
            .current_dir(temp.path())
            .args([
                "predict",
                &script_path,
                "--scores",
                scores,
                "--bet",
                "sample bet",
            ])
            .assert()
            .success();

        let after = prediction_files(temp.path());
        let prediction_path = after.difference(&before).next().unwrap().clone();
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
                &plays.to_string(),
                "--likes",
                "80",
                "--comments",
                "12",
                "--shares",
                "4",
                "--saves",
                "9",
            ])
            .assert()
            .success();
    }

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 3"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["upgrade", "--propose"])
        .assert()
        .success()
        .stdout(predicate::str::contains("upgrade proposal #1"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["upgrade", "--apply", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("active rubric: v1"));
}

fn prediction_files(root: &std::path::Path) -> HashSet<PathBuf> {
    let dir = root.join("predictions");
    if !dir.exists() {
        return HashSet::new();
    }
    fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect()
}
