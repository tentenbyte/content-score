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
fn score_accepts_strict_json_file() {
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
    fs::write(
        temp.path().join("score.json"),
        r#"{
  "ER": {"score": 4, "reason": "specific emotional recognition"},
  "HP": {"score": 5, "reason": "strong opening contrast"},
  "QL": {"score": 3, "reason": "one reusable line"},
  "NA": {"score": 3, "reason": "clear but simple arc"},
  "AB": {"score": 4, "reason": "broad creator audience"},
  "SR": {"score": 2, "reason": "weak social conflict"},
  "SAT": {"score": 1, "reason": "little irony"}
}"#,
    )
    .unwrap();

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["score", "scripts/foo.md", "--score-json", "score.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("composite: 6.29 / 10"));
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
fn retro_import_csv_records_rows_and_continues_after_errors() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    let first_id = create_prediction(temp.path(), "a.md", "强情绪开头，猫猫救场。");
    let second_id = create_prediction(temp.path(), "b.md", "社会议题明确，但情绪较弱。");
    let second_path = temp
        .path()
        .join("predictions")
        .join(format!("{second_id}.md"));
    fs::write(second_path, "edited prediction").unwrap();

    fs::write(
        temp.path().join("douyin.csv"),
        format!(
            "prediction_id,plays,likes,comments,shares,saves,top_comments,notes\n\
             {first_id},1200,80,12,4,9,\"评论1|评论2\",T+3\n\
             {second_id},900,60,8,3,5,\"评论A|评论B\",T+3 tampered\n\
             missing_prediction,10,1,0,0,0,,bad row\n"
        ),
    )
    .unwrap();

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["retro", "import", "douyin.csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported: 2"))
        .stdout(predicate::str::contains("failed: 1"))
        .stdout(predicate::str::contains("contaminated: 1"))
        .stdout(predicate::str::contains("missing_prediction"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 1"));
}

#[test]
fn retro_import_json_records_rows() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    let prediction_id = create_prediction(temp.path(), "json.md", "普通经验分享，开头一般。");
    fs::write(
        temp.path().join("douyin.json"),
        format!(
            r#"[
  {{
    "prediction_id": "{prediction_id}",
    "plays": 1500,
    "likes": 120,
    "comments": 18,
    "shares": 7,
    "saves": 11,
    "top_comments": ["评论1", "评论2"],
    "notes": "T+3"
  }}
]"#
        ),
    )
    .unwrap();

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["retro", "import", "douyin.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported: 1"))
        .stdout(predicate::str::contains("failed: 0"))
        .stdout(predicate::str::contains("contaminated: 0"));
}

#[test]
fn retro_import_rejects_duplicate_prediction_by_default() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    let prediction_id = create_prediction(temp.path(), "duplicate.md", "普通经验分享，开头一般。");
    write_retro_json(temp.path(), "first.json", &prediction_id, 1200);
    write_retro_json(temp.path(), "second.json", &prediction_id, 1800);

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["retro", "import", "first.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported: 1"))
        .stdout(predicate::str::contains("failed: 0"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["retro", "import", "second.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported: 0"))
        .stdout(predicate::str::contains("failed: 1"))
        .stdout(predicate::str::contains("already has a retro"));
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

fn create_prediction(root: &std::path::Path, file_name: &str, body: &str) -> String {
    fs::create_dir_all(root.join("scripts")).unwrap();
    let script_path = format!("scripts/{file_name}");
    fs::write(root.join(&script_path), body).unwrap();
    let before = prediction_files(root);

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(root)
        .args([
            "predict",
            &script_path,
            "--scores",
            "ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1",
            "--bet",
            "sample bet",
        ])
        .assert()
        .success();

    let after = prediction_files(root);
    let prediction_path = after.difference(&before).next().unwrap().clone();
    prediction_path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string()
}

fn write_retro_json(root: &std::path::Path, file_name: &str, prediction_id: &str, plays: i64) {
    fs::write(
        root.join(file_name),
        format!(
            r#"[
  {{
    "prediction_id": "{prediction_id}",
    "plays": {plays},
    "likes": 120,
    "comments": 18,
    "shares": 7,
    "saves": 11,
    "top_comments": ["评论1", "评论2"],
    "notes": "T+3"
  }}
]"#
        ),
    )
    .unwrap();
}
