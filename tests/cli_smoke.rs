use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;
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
fn douyin_help_lists_subcommands() {
    Command::cargo_bin("content-score")
        .unwrap()
        .args(["douyin", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("fetch"));
}

#[test]
fn douyin_fetch_uses_adapter_and_imports_by_default() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id = create_prediction(temp.path(), "douyin.md", "强情绪开头，猫猫救场。");
    let adapter = write_fake_douyin_adapter(temp.path(), 1200);

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args(["douyin", "fetch", &prediction_id, "7333333333333333333"])
        .assert()
        .success()
        .stdout(predicate::str::contains("aweme_id: 7333333333333333333"))
        .stdout(predicate::str::contains("imported: 1"))
        .stdout(predicate::str::contains("failed: 0"))
        .stdout(predicate::str::contains("imported: yes"));

    assert!(temp
        .path()
        .join(".content-score/imports")
        .join(format!("douyin-{prediction_id}.json"))
        .exists());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 1"));
}

#[test]
fn douyin_fetch_no_import_writes_json_without_completed_sample() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id = create_prediction(temp.path(), "douyin-no-import.md", "普通经验分享。");
    let adapter = write_fake_douyin_adapter(temp.path(), 1400);
    let output_path = temp
        .path()
        .join(".content-score/imports")
        .join(format!("douyin-{prediction_id}.json"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args([
            "douyin",
            "fetch",
            &prediction_id,
            "7333333333333333333",
            "--no-import",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("aweme_id: 7333333333333333333"))
        .stdout(predicate::str::contains(format!(
            "json: {}",
            output_path.display()
        )))
        .stdout(predicate::str::contains("imported: no"));

    assert!(output_path.exists());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 0"));
}

#[test]
fn douyin_fetch_rejects_adapter_output_for_wrong_prediction_id() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let requested_id = create_prediction(temp.path(), "douyin-requested.md", "普通经验分享。");
    let other_id = create_prediction(temp.path(), "douyin-other.md", "另一个预测。");
    let adapter = write_fake_douyin_adapter_with_rows(
        temp.path(),
        format!(
            r#"[{{
        "prediction_id": {},
        "plays": 1700,
        "likes": 80,
        "comments": 12,
        "shares": 4,
        "saves": 9,
        "top_comments": ["评论1"],
        "notes": "wrong prediction"
    }}]"#,
            serde_json::to_string(&other_id).unwrap()
        ),
    );
    let output_path = temp
        .path()
        .join(".content-score/imports")
        .join(format!("douyin-{requested_id}.json"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args(["douyin", "fetch", &requested_id, "7333333333333333333"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("aweme_id: 7333333333333333333"))
        .stderr(predicate::str::contains("adapter output prediction_id"));

    assert!(output_path.exists());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 0"));
}

#[test]
fn douyin_fetch_rejects_adapter_output_with_multiple_rows() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id = create_prediction(temp.path(), "douyin-multiple.md", "普通经验分享。");
    let encoded_id = serde_json::to_string(&prediction_id).unwrap();
    let adapter = write_fake_douyin_adapter_with_rows(
        temp.path(),
        format!(
            r#"[{{
        "prediction_id": {encoded_id},
        "plays": 1700,
        "likes": 80,
        "comments": 12,
        "shares": 4,
        "saves": 9,
        "top_comments": ["评论1"],
        "notes": "first"
    }}, {{
        "prediction_id": {encoded_id},
        "plays": 1800,
        "likes": 81,
        "comments": 13,
        "shares": 5,
        "saves": 10,
        "top_comments": ["评论2"],
        "notes": "second"
    }}]"#
        ),
    );
    let output_path = temp
        .path()
        .join(".content-score/imports")
        .join(format!("douyin-{prediction_id}.json"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args(["douyin", "fetch", &prediction_id, "7333333333333333333"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("aweme_id: 7333333333333333333"))
        .stderr(predicate::str::contains(
            "adapter output must contain exactly one row",
        ));

    assert!(output_path.exists());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 0"));
}

#[test]
fn douyin_fetch_dry_run_invokes_adapter_writes_json_without_import() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id = create_prediction(temp.path(), "douyin-dry-run.md", "普通经验分享。");
    let adapter = write_fake_douyin_adapter(temp.path(), 1500);
    let marker_path = fake_douyin_adapter_marker(temp.path());
    let output_path = temp
        .path()
        .join(".content-score/imports")
        .join(format!("douyin-{prediction_id}.json"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args([
            "douyin",
            "fetch",
            &prediction_id,
            "7333333333333333333",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("aweme_id: 7333333333333333333"))
        .stdout(predicate::str::contains(format!(
            "json: {}",
            output_path.display()
        )))
        .stdout(predicate::str::contains("dry-run: yes"))
        .stdout(predicate::str::contains("imported: no"));

    assert_eq!(
        fs::read_to_string(marker_path).unwrap(),
        "fetch:7333333333333333333"
    );
    assert!(output_path.exists());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 0"));
}

#[test]
fn douyin_fetch_duplicate_default_fails_before_adapter_execution() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id = create_prediction(temp.path(), "douyin-duplicate.md", "普通经验分享。");
    write_retro_json(temp.path(), "existing.json", &prediction_id, 1200);
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["retro", "import", "existing.json"])
        .assert()
        .success();

    let adapter = write_fake_douyin_adapter(temp.path(), 1600);
    let marker_path = fake_douyin_adapter_marker(temp.path());
    let output_path = temp
        .path()
        .join(".content-score/imports")
        .join(format!("douyin-{prediction_id}.json"));

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args(["douyin", "fetch", &prediction_id, "7333333333333333333"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("aweme_id:").not())
        .stderr(predicate::str::contains("already has a retro"));

    assert!(!marker_path.exists());
    assert!(!output_path.exists());
}

#[test]
fn douyin_fetch_replace_replaces_existing_retro() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id = create_prediction(temp.path(), "douyin-replace.md", "普通经验分享。");
    let first_adapter = write_fake_douyin_adapter(temp.path(), 1200);

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &first_adapter)
        .args(["douyin", "fetch", &prediction_id, "7333333333333333333"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported: yes"));

    let second_adapter = write_fake_douyin_adapter(temp.path(), 1800);
    let marker_path = fake_douyin_adapter_marker(temp.path());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &second_adapter)
        .args([
            "douyin",
            "fetch",
            &prediction_id,
            "7333333333333333333",
            "--replace",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("aweme_id: 7333333333333333333"))
        .stdout(predicate::str::contains("imported: 1"))
        .stdout(predicate::str::contains("replaced: yes"))
        .stdout(predicate::str::contains("imported: yes"));

    assert_eq!(
        fs::read_to_string(marker_path).unwrap(),
        "fetch:7333333333333333333"
    );
    assert_retro_plays(temp.path(), &prediction_id, 1, 1800, 1800);

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("calibrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("samples: 1"));
}

#[test]
fn douyin_fetch_replace_without_existing_retro_does_not_claim_replaced() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id = create_prediction(temp.path(), "douyin-replace-new.md", "普通经验分享。");
    let adapter = write_fake_douyin_adapter(temp.path(), 1800);

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args([
            "douyin",
            "fetch",
            &prediction_id,
            "7333333333333333333",
            "--replace",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported: 1"))
        .stdout(predicate::str::contains("replaced: yes").not())
        .stdout(predicate::str::contains("imported: yes"));

    assert_retro_plays(temp.path(), &prediction_id, 1, 1800, 1800);
}

#[test]
fn douyin_fetch_rejects_replace_with_no_import() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id =
        create_prediction(temp.path(), "douyin-replace-no-import.md", "普通经验分享。");
    let adapter = write_fake_douyin_adapter(temp.path(), 1600);
    let marker_path = fake_douyin_adapter_marker(temp.path());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args([
            "douyin",
            "fetch",
            &prediction_id,
            "7333333333333333333",
            "--replace",
            "--no-import",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("aweme_id:").not())
        .stderr(predicate::str::contains(
            "--replace cannot be used with --no-import",
        ));

    assert!(!marker_path.exists());
}

#[test]
fn douyin_fetch_rejects_replace_with_dry_run() {
    let temp = tempdir().unwrap();
    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    let prediction_id =
        create_prediction(temp.path(), "douyin-replace-dry-run.md", "普通经验分享。");
    let adapter = write_fake_douyin_adapter(temp.path(), 1600);
    let marker_path = fake_douyin_adapter_marker(temp.path());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args([
            "douyin",
            "fetch",
            &prediction_id,
            "7333333333333333333",
            "--replace",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("aweme_id:").not())
        .stderr(predicate::str::contains(
            "--replace cannot be used with --dry-run",
        ));

    assert!(!marker_path.exists());
}

#[test]
fn douyin_doctor_delegates_to_adapter_and_streams_output() {
    let temp = tempdir().unwrap();
    let adapter = write_fake_douyin_adapter(temp.path(), 1600);
    let marker_path = fake_douyin_adapter_marker(temp.path());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args(["douyin", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor: ok"));

    assert_eq!(fs::read_to_string(marker_path).unwrap(), "doctor");
}

#[test]
fn douyin_login_delegates_to_adapter_and_streams_output() {
    let temp = tempdir().unwrap();
    let adapter = write_fake_douyin_adapter(temp.path(), 1600);
    let marker_path = fake_douyin_adapter_marker(temp.path());

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .env("CONTENT_SCORE_DOUYIN_ADAPTER", &adapter)
        .args(["douyin", "login"])
        .assert()
        .success()
        .stdout(predicate::str::contains("login: ok"));

    assert_eq!(fs::read_to_string(marker_path).unwrap(), "login");
}

#[test]
fn douyin_fetch_rejects_unsupported_url_input() {
    Command::cargo_bin("content-score")
        .unwrap()
        .args([
            "douyin",
            "fetch",
            "pred_1",
            "https://example.com/video/7333333333333333333",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("unsupported Douyin input")
                .or(predicate::str::contains("Douyin")),
        );
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

fn write_fake_douyin_adapter(root: &std::path::Path, plays: i64) -> PathBuf {
    write_fake_douyin_adapter_with_rows(
        root,
        format!(
            r#"[{{
        "prediction_id": "__REQUESTED_PREDICTION_ID__",
        "plays": {plays},
        "likes": 80,
        "comments": 12,
        "shares": 4,
        "saves": 9,
        "top_comments": ["评论1"],
        "notes": "fake douyin"
    }}]"#
        ),
    )
}

fn write_fake_douyin_adapter_with_rows(root: &std::path::Path, rows_json: String) -> PathBuf {
    let adapter = root.join("fake_douyin_adapter.py");
    let marker =
        serde_json::to_string(&fake_douyin_adapter_marker(root).display().to_string()).unwrap();
    let rows_json = serde_json::to_string(&rows_json).unwrap();
    fs::write(
        &adapter,
        format!(
            r#"#!/usr/bin/env python3
import argparse
import json

parser = argparse.ArgumentParser()
subparsers = parser.add_subparsers(dest="command", required=True)
subparsers.add_parser("doctor")
subparsers.add_parser("login")
fetch = subparsers.add_parser("fetch")
fetch.add_argument("input")
fetch.add_argument("--prediction-id", required=True)
fetch.add_argument("--output", required=True)
args = parser.parse_args()

with open({marker}, "w", encoding="utf-8") as marker:
    if args.command == "fetch":
        marker.write(f"fetch:{{args.input}}")
    else:
        marker.write(args.command)

if args.command == "doctor":
    print("doctor: ok")
    raise SystemExit(0)

if args.command == "login":
    print("login: ok")
    raise SystemExit(0)

rows_json = {rows_json}.replace("__REQUESTED_PREDICTION_ID__", args.prediction_id)
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(json.loads(rows_json), handle, ensure_ascii=False)
print(f"aweme_id: {{args.input}}")
"#
        ),
    )
    .unwrap();
    adapter
}

fn fake_douyin_adapter_marker(root: &std::path::Path) -> PathBuf {
    root.join("fake_douyin_adapter.invoked")
}

fn assert_retro_plays(
    root: &std::path::Path,
    prediction_id: &str,
    expected_count: i64,
    expected_min: i64,
    expected_max: i64,
) {
    let conn = Connection::open(root.join(".content-score/content.sqlite")).unwrap();
    let (count, min_plays, max_plays): (i64, i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(MIN(plays), 0), COALESCE(MAX(plays), 0) \
             FROM retros WHERE prediction_id = ?1",
            [prediction_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert_eq!(
        (count, min_plays, max_plays),
        (expected_count, expected_min, expected_max)
    );
}
