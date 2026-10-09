#![cfg(feature = "wasm-precheck")]
use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn worker(source: &str) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["__syntax-worker", "cfquery"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn distinct_without_projection_is_separate_candidate_not_fake_recovery() {
    let source = "SELECT DISTINCT FROM users";
    let report = worker(source);
    assert_eq!(
        report["recoveries"].as_array().unwrap().len(),
        0,
        "{report}"
    );
    assert_eq!(
        report["structural_observations"].as_array().map(Vec::len),
        Some(1),
        "{report}"
    );
}

#[test]
fn literals_comments_interpolation_and_valid_projection_do_not_match() {
    for source in [
        "SELECT FROM users",
        "SELECT DISTINCT id FROM users",
        "SELECT DISTINCT 'FROM' FROM users",
        "SELECT DISTINCT \"from\" FROM users",
        "SELECT DISTINCT $$FROM$$ FROM users",
        "SELECT DISTINCT #columns# FROM users",
        "-- SELECT DISTINCT FROM users\nSELECT id FROM users",
        "/* SELECT DISTINCT FROM users */ SELECT id FROM users",
        "SELECT id FROM users WHERE name='SELECT DISTINCT FROM users'",
    ] {
        let report = worker(source);
        assert!(
            report.get("structural_observations").is_none(),
            "{source}: {report}"
        );
    }
    for source in [
        "select distinct from users",
        "SELECT DISTINCT /* comment */ FROM users",
        "SELECT DISTINCT -- comment\nFROM users",
    ] {
        let report = worker(source);
        assert_eq!(
            report["structural_observations"].as_array().map(Vec::len),
            Some(1),
            "{source}: {report}"
        );
    }
}

#[test]
fn embedded_structure_reuses_task_and_restores_whole_file_identity_and_positions() {
    use serde_json::json;
    use std::fs;
    let temp = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-cfquery-structure-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(temp.clone());
    let source = "<!--- 注释 --->\n<cfquery name=\"q\">\nSELECT DISTINCT FROM users\n</cfquery>\n";
    fs::write(temp.join("query.cfm"), source).unwrap();
    let run = |args: Vec<String>, input: Option<Value>, code: i32| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env("PATH", "/no/tools")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(v) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(v.to_string().as_bytes())
                .unwrap();
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(code), "{out:?}");
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    run(
        vec![
            "init".into(),
            temp.to_str().unwrap().into(),
            "--apply".into(),
            "--format=json".into(),
        ],
        None,
        3,
    );
    let event = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["query.cfm"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
    let args = vec![
        "hook".into(),
        "execute".into(),
        temp.to_str().unwrap().into(),
        "--timeout".into(),
        "30s".into(),
        "--format=json".into(),
    ];
    let first = run(args.clone(), Some(event.clone()), 3);
    assert_eq!(first["schema_version"], "0.23.0", "{first}");
    let feedback = &first["local_feedback"];
    let row = feedback["syntax_candidates"]["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["language"] == "cfquery")
        .unwrap();
    assert_eq!(row["recovery_count"], 0);
    assert!(
        row["known_limitations"]
            .to_string()
            .contains("SELECT DISTINCT FROM users")
    );
    assert!(
        !row["known_limitations"]
            .to_string()
            .contains("despite missing SQL select list")
    );
    assert_eq!(row["structural_observations"][0]["start_row"], 2);
    assert_eq!(
        row["structural_observations"][0]["start_byte"],
        source.find("SELECT").unwrap()
    );
    assert_eq!(feedback["syntax_tasks"]["failures"], json!([]), "{first}");
    let id = feedback["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    let next = run(
        vec![
            "next".into(),
            temp.to_str().unwrap().into(),
            "--format=json".into(),
        ],
        None,
        0,
    );
    assert!(next.to_string().contains("datasource"), "{next}");
    assert!(!next.to_string().contains("Zig 0.16.0"));
    let repeat = run(args.clone(), Some(event.clone()), 3);
    assert_eq!(
        repeat["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"],
        id
    );
    let check = run(
        vec![
            "check".into(),
            "all".into(),
            temp.to_str().unwrap().into(),
            "--format=json".into(),
        ],
        None,
        3,
    );
    assert_eq!(check["schema_version"], "0.53.0", "{check}");
    assert_eq!(check["syntax_tasks"]["failures"], json!([]), "{check}");
    assert_eq!(check["syntax_tasks"]["tasks"][0]["task_id"], id);
    fs::write(
        temp.join("query.cfm"),
        source.replace("SELECT DISTINCT FROM", "SELECT DISTINCT id FROM"),
    )
    .unwrap();
    let fixed = run(args, Some(event), 3);
    let row = fixed["local_feedback"]["syntax_candidates"]["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["language"] == "cfquery")
        .unwrap();
    assert!(row.get("structural_observations").is_none(), "{fixed}");
    let fact: Value = serde_json::from_slice(
        &fs::read(temp.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn candidate_record_budget_remains_visible_in_worker_feedback() {
    let report = worker(&"SELECT DISTINCT FROM users;\n".repeat(129));
    assert_eq!(
        report["structural_observations"].as_array().map(Vec::len),
        Some(128)
    );
    assert_eq!(report["truncated"], true);
    assert_eq!(report["recoveries"], serde_json::json!([]));
}
