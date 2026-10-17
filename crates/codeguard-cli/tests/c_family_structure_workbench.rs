use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires installed fixed Apple Clang21; set CODEGUARD_CLANG_BIN"]
fn structural_deficits_share_stable_file_tasks_and_original_rescan_does_not_close_them() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("existing tool"));
    let mut evidence = Vec::new();
    for (language, standard, ext) in [("c", "c11", "c"), ("cpp", "c++17", "cpp")] {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-structure-work-{language}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let _fixture = Fixture(root.clone());
        let source = root.join(format!("api.{ext}"));
        let initial = if language == "c" {
            "int f(int x) { return x; }\nint g(int x) { return x+1; }\n"
        } else {
            "int f(int x) { return x; }\nint f(double x) { return (int)x; }\n"
        };
        fs::write(&source, initial).unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        let scan = || {
            let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["comments", language])
                .arg(&source)
                .arg("--clang-tool")
                .arg(&tool)
                .args(["--standard", standard, "--format=json"])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(3));
            serde_json::from_slice::<Value>(&output.stdout).unwrap()
        };
        let first = scan();
        let ids = first["structural_workbench"]["task_ids"]
            .as_array()
            .expect("structural stable tasks");
        assert_eq!(ids.len(), 1);
        let id = ids[0].as_str().unwrap().to_owned();
        assert_eq!(
            first["structural_next"]["repair_brief"]["structural_positions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            first["structural_next"]["repair_brief"]["rule_source"],
            "codeguard_structural_policy"
        );
        assert_eq!(
            first["structural_next"]["repair_brief"]["allowed_paths"],
            json!([format!("api.{ext}")])
        );
        let second = scan();
        assert_eq!(second["structural_workbench"]["task_ids"], json!([id]));
        let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify"])
            .arg(&id)
            .arg(&root)
            .arg("--clang-tool")
            .arg(&tool)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(verify.status.code(), Some(3));
        let verify_present: Value = serde_json::from_slice(&verify.stdout).unwrap();
        assert_eq!(verify_present["observation"], "still_present");
        assert_eq!(verify_present["event_persisted"], true);
        let task_call = |words: &[&str], options: &[&str]| {
            let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .arg("task")
                .args(words)
                .arg(&id)
                .arg(&root)
                .args(options)
                .arg("--format=json")
                .output()
                .unwrap();
            (
                output.status.code(),
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            )
        };
        let (claim_exit, lease) = task_call(&["claim"], &["--owner", "struct-agent"]);
        assert_eq!(claim_exit, Some(0), "{lease}");
        let token = lease["lease_token"].as_str().unwrap();
        let show = || task_call(&["show"], &[]).1["task"].clone();
        let mut attempts = Vec::new();
        for count in 1..=2 {
            let (exit, start) = task_call(
                &["attempt", "start"],
                &[
                    "--owner",
                    "struct-agent",
                    "--lease-token",
                    token,
                    "--action-id",
                    "repair-source",
                ],
            );
            assert_eq!(exit, Some(0), "{start}");
            assert_eq!(show()["disposition"], "waiting");
            let (exit, finish) = task_call(
                &["attempt", "finish"],
                &[
                    "--owner",
                    "struct-agent",
                    "--lease-token",
                    token,
                    "--attempt-id",
                    start["attempt_id"].as_str().unwrap(),
                    "--outcome",
                    "ready-to-verify",
                    "--note-code",
                    "source_edit",
                ],
            );
            assert_eq!(exit, Some(0), "{finish}");
            assert_eq!(show()["disposition"], "verification_required");
            let (exit, blocked_retry) = task_call(
                &["attempt", "start"],
                &[
                    "--owner",
                    "struct-agent",
                    "--lease-token",
                    token,
                    "--action-id",
                    "repair-source",
                ],
            );
            assert_eq!(exit, Some(3));
            assert_eq!(
                blocked_retry["reason"],
                "verification_required_before_retry"
            );
            let (exit, recheck) = task_call(
                &["verify"],
                &["--owner", "struct-agent", "--lease-token", token],
            );
            assert_eq!(exit, Some(3));
            assert_eq!(recheck["observation"], "still_present");
            assert_eq!(show()["history"]["no_progress_count"], count);
            attempts.push(json!({"start":start,"finish":finish,"recheck":recheck}));
        }
        let exhausted = show();
        assert_eq!(exhausted["reason_code"], "no_progress_budget_exhausted");
        assert_eq!(exhausted["allowed_paths"], json!([]));
        let (_, renamed) = task_call(
            &["attempt", "start"],
            &[
                "--owner",
                "struct-agent",
                "--lease-token",
                token,
                "--action-id",
                "restore-checker-environment",
            ],
        );
        assert_eq!(renamed["reason"], "action_id_invalid");
        let (_, retry) = task_call(
            &["attempt", "start"],
            &[
                "--owner",
                "struct-agent",
                "--lease-token",
                token,
                "--action-id",
                "repair-source",
            ],
        );
        assert_eq!(retry["reason"], "no_progress_budget_exhausted");
        let repeat_budget = scan();
        assert_eq!(show()["history"]["no_progress_count"], 2);
        fs::remove_file(root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        scan();
        assert_eq!(show()["history"]["no_progress_count"], 2);
        let first_recheck_run = attempts[0]["recheck"]["native_scan"]["run_id"]
            .as_str()
            .unwrap();
        let report_path = root.join(format!(".codeguard/reports/{first_recheck_run}.json"));
        let recheck_bytes = fs::read(&report_path).unwrap();
        fs::remove_file(&report_path).unwrap();
        let missing_history = show();
        assert_eq!(
            missing_history["reason_code"],
            "historical_verification_evidence_unavailable"
        );
        assert_eq!(missing_history["allowed_paths"], json!([]));
        fs::write(&report_path, &recheck_bytes).unwrap();
        assert_eq!(show()["history"]["no_progress_count"], 2);
        let receipt_path = root.join(format!(
            ".codeguard/state/consumed/{first_recheck_run}.json"
        ));
        let receipt_bytes = fs::read(&receipt_path).unwrap();
        fs::remove_file(&receipt_path).unwrap();
        assert_eq!(
            show()["reason_code"],
            "historical_verification_evidence_unavailable"
        );
        fs::write(&receipt_path, &receipt_bytes).unwrap();
        let mut forged_receipt = receipt_bytes.clone();
        forged_receipt.push(b'\n');
        fs::write(&receipt_path, forged_receipt).unwrap();
        let (exit, bad_receipt) = task_call(&["show"], &[]);
        assert_eq!(exit, Some(3));
        assert_eq!(
            bad_receipt["reason"],
            "clang_structure_observation_not_consumed"
        );
        fs::write(&receipt_path, receipt_bytes).unwrap();
        let event_path = root.join(format!(
            ".codeguard/findings/{id}/events/verify-{first_recheck_run}.json"
        ));
        let event_bytes = fs::read(&event_path).unwrap();
        let mut forged_event: Value = serde_json::from_slice(&event_bytes).unwrap();
        forged_event["observation"] = json!("candidate_absent_unverified_policy");
        fs::write(
            &event_path,
            serde_json::to_vec_pretty(&forged_event).unwrap(),
        )
        .unwrap();
        let (exit, rejected) = task_call(&["show"], &[]);
        assert_eq!(exit, Some(3));
        assert_eq!(rejected["reason"], "verification_event_invalid");
        fs::write(&event_path, event_bytes).unwrap();
        task_call(
            &["release"],
            &["--owner", "struct-agent", "--lease-token", token],
        );
        fs::write(&source, format!("\n\n{initial}")).unwrap();
        let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("task")
            .arg("show")
            .arg(&id)
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(next.status.code(), Some(0));
        let stale: Value = serde_json::from_slice(&next.stdout).unwrap();
        assert_eq!(stale["task"]["allowed_paths"], json!([]));
        let moved = scan();
        assert_eq!(moved["structural_workbench"]["task_ids"], json!([id]));
        assert_eq!(
            moved["structural_next"]["repair_brief"]["structural_positions"][0]["line"],
            3
        );
        let text = fs::read_to_string(root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        for heading in [
            "问题证据",
            "规则依据",
            "允许修改",
            "修复步骤",
            "复检",
            "历史尝试",
            "关闭条件",
        ] {
            assert!(text.contains(heading), "{heading}");
        }
        fs::remove_file(root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        let restored = scan();
        assert!(root.join(format!(".codeguard/tasks/{id}.md")).exists());
        assert_eq!(restored["structural_workbench"]["task_ids"], json!([id]));
        assert!(
            fs::read_to_string(root.join(format!(".codeguard/tasks/{id}.md")))
                .unwrap()
                .contains("codeguard.documentation.function_structure_required")
        );
        let fixed = if language == "c" {
            "/** Read value.\n * @param x input value\n * @return same value\n */\nint f(int x) { return x; }\n/** Increment.\n * @param x input value\n * @return incremented value\n */\nint g(int x) { return x+1; }\n"
        } else {
            "/** Read integer.\n * @param x input value\n * @return same value\n */\nint f(int x) { return x; }\n/** Convert value.\n * @param x floating input\n * @return converted integer\n */\nint f(double x) { return (int)x; }\n"
        };
        fs::write(&source, fixed).unwrap();
        let clean = scan();
        assert_eq!(clean["structural_workbench"]["task_ids"], json!([]));
        assert!(
            !clean["structural_next"]["repair_brief"].is_null(),
            "{clean}"
        );
        assert_eq!(
            clean["structural_next"]["repair_brief"]["observation_status"],
            "candidate_absent_unverified_policy"
        );
        let argv = clean["structural_next"]["repair_brief"]["recheck_argv"]
            .as_array()
            .unwrap();
        let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(argv.iter().skip(1).map(|v| v.as_str().unwrap()))
            .output()
            .unwrap();
        assert_eq!(verified.status.code(), Some(3));
        let verify_absent: Value = serde_json::from_slice(&verified.stdout).unwrap();
        assert_eq!(
            verify_absent["observation"],
            "candidate_absent_unverified_policy"
        );
        assert_eq!(verify_absent["event_persisted"], true);
        let shown_after = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "show"])
            .arg(&id)
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        let current_after: Value = serde_json::from_slice(&shown_after.stdout).unwrap();
        assert_eq!(
            current_after["task"]["observation_status"],
            "candidate_absent_unverified_policy"
        );
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        let origin = root.join(format!(
            ".codeguard/reports/{}.json",
            first["structural_workbench"]["run_id"].as_str().unwrap()
        ));
        let bytes = fs::read(&origin).unwrap();
        let mut altered = bytes.clone();
        altered.push(b'\n');
        fs::write(&origin, altered).unwrap();
        let shown = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "show"])
            .arg(&id)
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(shown.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&shown.stdout).contains("clang_structure_origin_changed"));
        fs::write(origin, bytes).unwrap();
        let packets = fs::read_dir(root.join(".codeguard/reports"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("clangdocstruct-"))
            })
            .map(|p| serde_json::from_slice::<Value>(&fs::read(p).unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(packets.len(), 11);
        evidence.push(json!({"language":language,"task_id":id,"first":first,"repeated":second,"stale":stale,"moved":moved,"restored":restored,"clean":clean,"packets":packets,"verify_present":verify_present,"verify_absent":verify_absent,"attempts":attempts,"exhausted":exhausted,"repeat_budget":repeat_budget,"missing_history":missing_history}));
    }
    if let Some(path) = std::env::var_os("CODEGUARD_STRUCTURE_WORKBENCH_EVIDENCE") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"development_native_structure_workbench","qualification":"not_granted","test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_structure_workbench.rs"))),"codeguard_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),"cases":evidence})).unwrap()).unwrap();
    }
}
