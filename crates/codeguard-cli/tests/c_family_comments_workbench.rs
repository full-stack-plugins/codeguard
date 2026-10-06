#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-c-doc-work-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("api.c"), "/** API.\n * @param value\n * @param other\n */\nint api(int value, int other) { return value + other; }\n").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        let tool = root.join("clang");
        fs::write(&tool, format!("#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'Apple clang version 21.0.0 (clang-2100.3.34.2)'; exit 0; fi\ncat >/dev/null\ncat '{}' >&2\n", root.join("sarif.json").display())).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn scan(&self, lines: &[u64]) -> Value {
        let rows: Vec<_> = lines.iter().map(|line| json!({"level":"warning","ruleId":"warn_doc_block_command_empty_paragraph","ruleIndex":0,"message":{"text":"HOST_SECRET"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"file://","index":0},"region":{"startLine":line,"startColumn":4}}}]})).collect();
        let sarif = json!({"version":"2.1.0","runs":[{"columnKind":"unicodeCodePoints","invocations":[{"executionSuccessful":true}],"artifacts":[{"location":{"uri":"file://","index":0}}],"tool":{"driver":{"name":"clang","version":"Apple clang version 21.0.0 (clang-2100.3.34.2)","rules":[{"id":"warn_doc_block_command_empty_paragraph"}]}},"results":rows}]});
        fs::write(self.0.join("sarif.json"), sarif.to_string()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "c"])
            .arg(self.0.join("api.c"))
            .arg("--clang-tool")
            .arg(self.0.join("clang"))
            .args(["--standard", "c11", "--workspace"])
            .arg(&self.0)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        capture(&report);
        for entry in fs::read_dir(self.0.join(".codeguard/reports")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().is_some_and(|e| e == "json") {
                capture(
                    &serde_json::from_slice::<Value>(&fs::read(entry.path()).unwrap()).unwrap(),
                );
            }
        }
        report
    }
    fn next(&self) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next"])
            .arg(&self.0)
            .arg("--format=json")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        capture(&report);
        report
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn capture(report: &Value) {
    if let Some(directory) = std::env::var_os("CODEGUARD_C_DOC_WORKBENCH_REPORT_DIR") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!(
                "report-{}-{}.json",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )),
            serde_json::to_vec_pretty(report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn clean_initial_scan_has_no_invented_task_and_missing_tool_has_stable_environment_task() {
    let p = Project::new();
    let clean = p.scan(&[]);
    assert_eq!(clean["workbench"]["task_ids"], json!([]));
    assert_eq!(clean["next"], Value::Null);
    fs::remove_file(p.0.join("clang")).unwrap();
    let blocked = p.scan(&[]);
    let id = blocked["workbench"]["task_ids"][0].as_str().unwrap();
    assert!(id.starts_with("CG-B-"));
    assert_eq!(
        blocked["next"]["repair_brief"]["native_reason"],
        "clang_tool_unavailable"
    );
    assert_eq!(blocked["documentation_findings"], json!([]));
    fs::write(p.0.join("api.c"), "#include <missing.h>\nint api(void);\n").unwrap();
    let changed = p.scan(&[]);
    assert_eq!(changed["workbench"]["task_ids"][0], id);
    assert_eq!(
        changed["next"]["repair_brief"]["native_reason"],
        "clang_preprocessor_context_unresolved"
    );
}

#[test]
fn cpp_and_unadapted_rules_preserve_checker_identity_and_mapping_blockers() {
    let p = Project::new();
    p.scan(&[2]);
    fs::copy(p.0.join("api.c"), p.0.join("api.cpp")).unwrap();
    let original = fs::read_to_string(p.0.join("clang")).unwrap();
    fs::write(
        p.0.join("clang"),
        original.replace(
            "cat '",
            "sed 's/warn_doc_block_command_empty_paragraph/warn_doc_unadapted/g' '",
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "cpp"])
        .arg(p.0.join("api.cpp"))
        .arg("--clang-tool")
        .arg(p.0.join("clang"))
        .args(["--standard", "c++17", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    capture(&r);
    assert_eq!(r["workbench"]["status"], "synced_partial");
    assert_eq!(r["documentation_findings"], json!([]));
    assert_eq!(
        r["next"]["repair_brief"]["checker_id"],
        "cpp.clang.documentation"
    );
    assert_eq!(r["next"]["repair_brief"]["kind"], "blocker");
    assert_eq!(
        r["next"]["repair_brief"]["unclassified_native_diagnostics"][0]["rule_id"],
        "clang.warn_doc_unadapted"
    );
}

#[test]
fn foreign_explicit_scope_is_rejected_and_invalid_nearest_workspace_never_falls_back() {
    let p = Project::new();
    p.scan(&[]);
    let child = p.0.join("child");
    fs::create_dir(&child).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "c"])
        .arg(p.0.join("api.c"))
        .arg("--clang-tool")
        .arg(p.0.join("clang"))
        .args(["--standard", "c11", "--workspace"])
        .arg(&child)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    fs::write(child.join(".codeguard"), "invalid nearest workspace").unwrap();
    fs::copy(p.0.join("api.c"), child.join("api.c")).unwrap();
    let before = fs::read_dir(p.0.join(".codeguard/reports"))
        .unwrap()
        .count();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "c"])
        .arg(child.join("api.c"))
        .arg("--clang-tool")
        .arg(p.0.join("clang"))
        .args(["--standard", "c11", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    capture(&r);
    assert_eq!(r["workspace_binding"], "not_bound");
    assert_eq!(r["workbench"]["status"], "incomplete");
    assert_eq!(
        before,
        fs::read_dir(p.0.join(".codeguard/reports"))
            .unwrap()
            .count()
    );
}

#[test]
fn linked_source_cannot_redirect_observation_storage_into_another_project() {
    let p = Project::new();
    let other = Project::new();
    fs::remove_file(p.0.join("api.c")).unwrap();
    std::os::unix::fs::symlink(other.0.join("api.c"), p.0.join("api.c")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "c"])
        .arg(p.0.join("api.c"))
        .arg("--clang-tool")
        .arg(p.0.join("clang"))
        .args(["--standard", "c11", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["schema_version"], "0.1.0");
    assert_eq!(r["workspace_binding"], "not_bound");
    assert_eq!(r["documentation_findings"], json!([]));
    assert_eq!(
        fs::read_dir(other.0.join(".codeguard/reports"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/reports"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn changed_tool_retracts_authority_and_recheck_rejects_before_acquiring_leases() {
    let p = Project::new();
    let r = p.scan(&[2]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    let original = fs::read_to_string(p.0.join("clang")).unwrap();
    fs::write(p.0.join("clang"), format!("{original}\n# changed\n")).unwrap();
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["observation_status"],
        "tool_or_standard_changed"
    );
    assert_eq!(next["repair_brief"]["allowed_paths"], json!([]));
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("clang_documentation_original_tool_changed")
    );
    assert!(!p.0.join(".codeguard/state/task_locks").exists());
}

#[test]
fn malformed_packet_and_origin_tampering_cannot_create_or_authorize_a_finding() {
    let p = Project::new();
    let r = p.scan(&[2]);
    let run = r["workbench"]["run_id"].as_str().unwrap();
    let path = p.0.join(format!(".codeguard/reports/{run}.json"));
    let mut packet: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    packet["authority"] = json!("trusted");
    packet["run_id"] = json!("clangdoc-1-0-999999999999999999999");
    fs::write(
        p.0.join(".codeguard/reports/clangdoc-1-0-999999999999999999999.json"),
        packet.to_string(),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync"])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let sync: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(sync["failed_reports"], 1);
    assert_eq!(sync["new_findings"], 0);
    // 先隔离已拒绝的新报文，再单独检验首次报告摘要篡改，避免队列失败掩盖目标断言。
    fs::remove_file(p.0.join(".codeguard/reports/clangdoc-1-0-999999999999999999999.json"))
        .unwrap();
    fs::write(&path, b"{}").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next"])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let feedback = String::from_utf8(out.stdout).unwrap();
    let feedback: Value = serde_json::from_str(&feedback).unwrap();
    // 共享队列先核对已消费报告摘要，比专属候选读取更早拒绝篡改。
    assert_eq!(feedback["reason"], "consumed_marker_invalid");
    assert_eq!(feedback["repair_brief"], Value::Null);
}

#[test]
#[ignore = "requires existing Apple Clang21 via CODEGUARD_CLANG_BIN"]
fn actual_clang_documentation_rules_reach_stable_workbench_tasks() {
    let p = Project::new();
    let tool = std::env::var_os("CODEGUARD_CLANG_BIN").expect("explicit Clang required");
    fs::copy(p.0.join("api.c"), p.0.join("api.cpp")).unwrap();
    let mut cases = Vec::new();
    for (language, filename, standard) in [("c", "api.c", "c11"), ("cpp", "api.cpp", "c++17")] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", language])
            .arg(p.0.join(filename))
            .arg("--clang-tool")
            .arg(&tool)
            .args(["--standard", standard, "--workspace"])
            .arg(&p.0)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        capture(&r);
        assert_eq!(r["workbench"]["status"], "synced_partial");
        assert_eq!(
            r["next"]["repair_brief"]["native_positions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            r["next"]["repair_brief"]["observation_status"],
            "native_rule_observed"
        );
        assert_eq!(r["coverage_proven"], false);
        let id = r["workbench"]["task_ids"][0].as_str().unwrap();
        let (exit, lease) = task_command(&p, &["claim"], id, &["--owner", "native-doc"]);
        assert_eq!(exit, 0, "{lease}");
        let token = lease["lease_token"].as_str().unwrap();
        let show = || {
            let (exit, shown) = task_command(&p, &["show"], id, &[]);
            assert_eq!(exit, 0, "{shown}");
            assert_eq!(shown["task_id"], id);
            assert_eq!(shown["schema_version"], "0.4.0");
            assert_eq!(shown["next_actions"][0], shown["task"]["recheck_argv"]);
            capture(&shown);
            shown["task"].clone()
        };
        let start_attempt = || {
            let (exit, start) = task_command(
                &p,
                &["attempt", "start"],
                id,
                &[
                    "--owner",
                    "native-doc",
                    "--lease-token",
                    token,
                    "--action-id",
                    "repair-source",
                ],
            );
            assert_eq!(exit, 0, "{start}");
            start
        };
        let finish_attempt = |start: &Value, note: &str| {
            let (exit, finish) = task_command(
                &p,
                &["attempt", "finish"],
                id,
                &[
                    "--owner",
                    "native-doc",
                    "--lease-token",
                    token,
                    "--attempt-id",
                    start["attempt_id"].as_str().unwrap(),
                    "--outcome",
                    "ready-to-verify",
                    "--note-code",
                    note,
                ],
            );
            assert_eq!(exit, 0, "{finish}");
            finish
        };
        let first_start = start_attempt();
        let first_finish = finish_attempt(&first_start, "no_change");
        let verify = || {
            let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["task", "verify", id])
                .arg(&p.0)
                .args(["--owner", "native-doc", "--lease-token", token])
                .arg("--format=json")
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(3), "{out:?}");
            let v: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(v["event_persisted"], true, "{v}");
            assert_eq!(v["native_scan"]["language"], language);
            capture(&v);
            v
        };
        let present = verify();
        assert_eq!(present["observation"], "still_present");
        assert_eq!(show()["history"]["no_progress_count"], 1);
        let second_start = start_attempt();
        let source = fs::read_to_string(p.0.join(filename)).unwrap();
        fs::write(
            p.0.join(filename),
            source
                .replace("@param value", "@param value First operand.")
                .replace("@param other", "@param other Second operand."),
        )
        .unwrap();
        let second_finish = finish_attempt(&second_start, "source_edit");
        assert_eq!(second_finish["observed_change"], true);
        let absent = verify();
        assert_eq!(absent["observation"], "candidate_absent_unverified_policy");
        assert_eq!(show()["history"]["awaiting_verification"], false);
        cases.push(json!({"scan":r,"present":present,"absent":absent,"attempts":[{"start":first_start,"finish":first_finish},{"start":second_start,"finish":second_finish}],"history":show()["history"]}));
    }
    for e in fs::read_dir(p.0.join(".codeguard/reports")).unwrap() {
        capture(&serde_json::from_slice::<Value>(&fs::read(e.unwrap().path()).unwrap()).unwrap());
    }
    if let Some(path) = std::env::var_os("CODEGUARD_C_DOC_RECHECK_NATIVE_EVIDENCE") {
        use sha2::{Digest, Sha256};
        let binary = fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap();
        let evidence = json!({"evidence_kind":"local_native_task_recheck_regression","qualification":"not_granted","independent_holdout":false,"codeguard_sha256":format!("{:x}",Sha256::digest(binary)),"test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_comments_workbench.rs"))),"cases":cases});
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
fn native_rule_positions_share_stable_task_and_clean_rescan_does_not_close_it() {
    let p = Project::new();
    let first = p.scan(&[2, 3]);
    assert_eq!(first["workbench"]["status"], "synced_partial");
    let id = first["workbench"]["task_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(first["workbench"]["task_ids"].as_array().unwrap().len(), 1);
    assert_eq!(
        first["next"]["repair_brief"]["native_positions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let second = p.scan(&[3]);
    assert_eq!(second["workbench"]["task_ids"][0], id);
    assert_eq!(second["workbench"]["sync"]["new_findings"], 0);
    assert_eq!(p.next()["repair_brief"]["native_positions"][0]["line"], 3);
    assert!(!second.to_string().contains("HOST_SECRET"));
    let clean = p.scan(&[]);
    assert_eq!(clean["next"]["repair_brief"]["task_id"], id);
    assert_eq!(
        clean["next"]["repair_brief"]["observation_status"],
        "candidate_absent_unverified"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    assert_eq!(clean["coverage_proven"], false);
}

#[test]
fn changed_input_retracts_positions_and_missing_projection_can_be_restored() {
    let p = Project::new();
    let report = p.scan(&[2]);
    let id = report["workbench"]["task_ids"][0].as_str().unwrap();
    fs::write(p.0.join("api.c"), "int api(void) { return 0; }\n").unwrap();
    let next = p.next();
    assert_eq!(next["repair_brief"]["observation_status"], "input_changed");
    assert_eq!(next["repair_brief"]["allowed_paths"], json!([]));
    fs::remove_file(p.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync"])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let task = fs::read_to_string(p.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    for part in [
        "问题证据",
        "规则依据",
        "允许修改的范围",
        "修复步骤",
        "复检命令",
        "历史尝试",
        "关闭条件",
    ] {
        assert!(task.contains(part), "{part}");
    }
}

#[test]
fn original_documentation_task_recheck_records_presence_and_absence_without_closing() {
    let p = Project::new();
    let r = p.scan(&[2]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    let verify = || {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify", id])
            .arg(&p.0)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        capture(&v);
        assert_eq!(
            v["schema_version"],
            "0.36.0",
            "{v} STDERR {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(v["event_persisted"], true, "{v}");
        assert_eq!(v["native_scan"]["task_id"], id);
        assert_eq!(v["delivery_decision"], "not_evaluated");
        v
    };
    assert_eq!(verify()["observation"], "still_present");
    p.scan(&[]);
    assert_eq!(
        verify()["observation"],
        "candidate_absent_unverified_policy"
    );
    let n = p.next();
    assert_eq!(n["repair_brief"]["task_id"], id);
    assert_eq!(n["repair_brief"]["allowed_paths"], json!([]));
}

#[test]
fn different_tool_and_foreign_parameters_are_rejected_before_a_lease_or_native_launch() {
    let p = Project::new();
    let r = p.scan(&[2]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    let other = p.0.join("other-clang");
    fs::copy(p.0.join("clang"), &other).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id])
        .arg(&p.0)
        .arg("--clang-tool")
        .arg(&other)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["reason"], "clang_documentation_original_tool_required");
    for flag in ["--ruff-tool", "--shellcheck-tool"] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify", id])
            .arg(&p.0)
            .arg(flag)
            .arg(&other)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
    }
    assert!(!p.0.join(".codeguard/state/task_locks").exists());
}

#[test]
fn changed_source_can_be_rechecked_but_forged_task_binding_cannot_be_imported() {
    let p = Project::new();
    let r = p.scan(&[2]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    let source = fs::read_to_string(p.0.join("api.c")).unwrap();
    fs::write(p.0.join("api.c"), format!("{source}\n")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["observation"], "still_present", "{v}");
    assert_eq!(v["event_persisted"], true);
    assert_ne!(v["native_scan"]["source_sha256"], r["source_sha256"]);
    let mut forged = v["native_scan"].clone();
    forged["task_id"] = json!("CG-00000000000000000000000000000000");
    forged["run_id"] = json!("clangdoc-1-1-1");
    fs::write(
        p.0.join(".codeguard/reports/clangdoc-1-1-1.json"),
        serde_json::to_vec_pretty(&forged).unwrap(),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync"])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(
        !p.0.join(".codeguard/state/consumed/clangdoc-1-1-1.json")
            .exists()
    );
}

#[test]
fn source_change_during_native_recheck_and_deadline_cannot_certify_the_task() {
    let p = Project::new();
    let tool = p.0.join("clang");
    let original = fs::read_to_string(&tool).unwrap();
    fs::write(&tool,original.replace("cat >/dev/null", &format!("cat >/dev/null\nif [ -f '{}' ]; then echo ' ' >> '{}'; fi\nif [ -f '{}' ]; then sleep 1; fi",p.0.join("mutate").display(),p.0.join("api.c").display(),p.0.join("slow").display()))).unwrap();
    let r = p.scan(&[2]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    let source_before = fs::read(p.0.join("api.c")).unwrap();
    fs::write(p.0.join("mutate"), "").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["observation"], "incomplete", "{v}");
    assert_eq!(v["native_scan"]["input_stable"], false);
    assert_eq!(p.next()["repair_brief"]["allowed_paths"], json!([]));
    fs::write(p.0.join("api.c"), source_before).unwrap();
    assert_eq!(p.next()["repair_brief"]["allowed_paths"], json!([]));
    fs::remove_file(p.0.join("mutate")).unwrap();
    fs::write(p.0.join("slow"), "").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id])
        .arg(&p.0)
        .args(["--timeout", "50ms", "--format=json"])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["observation"], "incomplete", "{v}");
    assert_eq!(v["event_persisted"], false);
    assert_eq!(v["reason"], "request_deadline_exceeded");
}

#[test]
fn sigint_recheck_returns_cancelled_without_persisting_and_reaps_descendants() {
    let p = Project::new();
    let tool = p.0.join("clang");
    let script = fs::read_to_string(&tool).unwrap();
    fs::write(&tool,script.replace("cat >/dev/null", &format!("cat >/dev/null\nif [ -f '{}' ]; then : > '{}'; (sleep 2; echo ghost > '{}') & sleep 10; fi",p.0.join("slow").display(),p.0.join("scanning").display(),p.0.join("ghost").display()))).unwrap();
    let r = p.scan(&[2]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    fs::write(p.0.join("slow"), "").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id])
        .arg(&p.0)
        .args(["--timeout", "20s", "--format=json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while !p.0.join("scanning").exists() {
        assert!(child.try_wait().unwrap().is_none());
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["command_status"], "cancelled");
    assert_eq!(v["reason"], "request_cancelled");
    assert_eq!(v["event_persisted"], false);
    capture(&v);
    std::thread::sleep(std::time::Duration::from_millis(2200));
    assert!(!p.0.join("ghost").exists());
}

fn task_command(p: &Project, verb: &[&str], id: &str, extra: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("task")
        .args(verb)
        .arg(id)
        .arg(&p.0)
        .args(extra)
        .arg("--format=json")
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{out:?}")),
    )
}

#[test]
fn failed_original_rechecks_consume_attempt_budget_and_action_renaming_cannot_reset_it() {
    let p = Project::new();
    let r = p.scan(&[2]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    let (exit, lease) = task_command(&p, &["claim"], id, &["--owner", "doc-agent"]);
    assert_eq!(exit, 0, "{lease}");
    let token = lease["lease_token"].as_str().unwrap();
    for count in 1..=2 {
        let (exit, start) = task_command(
            &p,
            &["attempt", "start"],
            id,
            &[
                "--owner",
                "doc-agent",
                "--lease-token",
                token,
                "--action-id",
                "repair-source",
            ],
        );
        assert_eq!(exit, 0, "{start}");
        assert_eq!(p.next()["repair_brief"]["disposition"], "waiting");
        let (exit, finish) = task_command(
            &p,
            &["attempt", "finish"],
            id,
            &[
                "--owner",
                "doc-agent",
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
        assert_eq!(exit, 0, "{finish}");
        let n = p.next();
        assert_eq!(
            n["repair_brief"]["disposition"], "verification_required",
            "{n}"
        );
        let (exit, verify) = task_command(
            &p,
            &["verify"],
            id,
            &["--owner", "doc-agent", "--lease-token", token],
        );
        assert_eq!(exit, 3, "{verify}");
        assert_eq!(verify["observation"], "still_present");
        let n = p.next();
        assert_eq!(
            n["repair_brief"]["history"]["no_progress_count"], count,
            "{n}"
        );
        assert_eq!(n["repair_brief"]["history"]["awaiting_verification"], false);
    }
    let n = p.next();
    assert_eq!(n["repair_brief"]["disposition"], "needs_decision");
    assert_eq!(n["repair_brief"]["allowed_paths"], json!([]));
    let (exit, renamed) = task_command(
        &p,
        &["attempt", "start"],
        id,
        &[
            "--owner",
            "doc-agent",
            "--lease-token",
            token,
            "--action-id",
            "restore-checker-environment",
        ],
    );
    assert_eq!(exit, 3);
    assert_eq!(renamed["reason"], "action_id_invalid");
    let (exit, exhausted) = task_command(
        &p,
        &["attempt", "start"],
        id,
        &[
            "--owner",
            "doc-agent",
            "--lease-token",
            token,
            "--action-id",
            "repair-source",
        ],
    );
    assert_eq!(exit, 3);
    assert_eq!(exhausted["reason"], "no_progress_budget_exhausted");
    p.scan(&[2]);
    assert_eq!(p.next()["repair_brief"]["history"]["no_progress_count"], 2);
    let mut observed: Vec<Value> =
        fs::read_dir(p.0.join(format!(".codeguard/findings/{id}/events")))
            .unwrap()
            .filter_map(|e| {
                let path = e.unwrap().path();
                if path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("verify-")
                {
                    Some(serde_json::from_slice(&fs::read(path).unwrap()).unwrap())
                } else {
                    None
                }
            })
            .collect();
    observed.sort_by_key(|e| {
        e["run_id"]
            .as_str()
            .unwrap()
            .rsplit('-')
            .next()
            .unwrap()
            .parse::<u128>()
            .unwrap()
    });
    let earlier = p.0.join(format!(
        ".codeguard/reports/{}.json",
        observed[0]["run_id"].as_str().unwrap()
    ));
    let retained = fs::read(&earlier).unwrap();
    fs::remove_file(&earlier).unwrap();
    let n = p.next();
    assert_eq!(
        n["repair_brief"]["reason_code"], "historical_verification_evidence_unavailable",
        "{n}"
    );
    assert_eq!(
        n["repair_brief"]["history"]["unverified_prior_attempt_count"],
        1
    );
    assert_eq!(n["repair_brief"]["allowed_paths"], json!([]));
    fs::write(&earlier, retained).unwrap();
    assert_eq!(p.next()["repair_brief"]["history"]["no_progress_count"], 2);
    let event_path = p.0.join(format!(
        ".codeguard/findings/{id}/events/verify-{}.json",
        observed[0]["run_id"].as_str().unwrap()
    ));
    let original_event = fs::read(&event_path).unwrap();
    let mut forged: Value = serde_json::from_slice(&original_event).unwrap();
    forged["observation"] = json!("candidate_absent_unverified_policy");
    fs::write(&event_path, serde_json::to_vec_pretty(&forged).unwrap()).unwrap();
    let (exit, shown) = task_command(&p, &["show"], id, &[]);
    assert_eq!(exit, 3);
    assert_eq!(shown["reason"], "verification_event_invalid");
    fs::write(&event_path, original_event).unwrap();
    assert_eq!(p.next()["repair_brief"]["history"]["no_progress_count"], 2);
}

#[test]
fn restoring_the_original_missing_tool_is_recorded_as_environment_change_without_source_edits() {
    let p = Project::new();
    let tool = p.0.join("clang");
    let original = fs::read(&tool).unwrap();
    fs::remove_file(&tool).unwrap();
    let r = p.scan(&[]);
    let id = r["workbench"]["task_ids"][0].as_str().unwrap();
    assert!(id.starts_with("CG-B-"));
    let n = p.next();
    assert_eq!(n["repair_brief"]["disposition"], "actionable", "{n}");
    assert_eq!(
        n["repair_brief"]["action_id"],
        "restore-checker-environment"
    );
    assert_eq!(n["repair_brief"]["allowed_paths"], json!([]));
    let (exit, lease) = task_command(&p, &["claim"], id, &["--owner", "env-agent"]);
    assert_eq!(exit, 0, "{lease}");
    let token = lease["lease_token"].as_str().unwrap();
    let (exit, start) = task_command(
        &p,
        &["attempt", "start"],
        id,
        &[
            "--owner",
            "env-agent",
            "--lease-token",
            token,
            "--action-id",
            "restore-checker-environment",
        ],
    );
    assert_eq!(exit, 0, "{start}");
    let source = fs::read(p.0.join("api.c")).unwrap();
    fs::write(&tool, original).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let (exit, finish) = task_command(
        &p,
        &["attempt", "finish"],
        id,
        &[
            "--owner",
            "env-agent",
            "--lease-token",
            token,
            "--attempt-id",
            start["attempt_id"].as_str().unwrap(),
            "--outcome",
            "ready-to-verify",
            "--note-code",
            "tool_restored",
        ],
    );
    assert_eq!(exit, 0, "{finish}");
    assert_eq!(finish["observed_change"], true);
    assert_eq!(source, fs::read(p.0.join("api.c")).unwrap());
    assert_eq!(
        p.next()["repair_brief"]["disposition"],
        "verification_required"
    );
    let (exit, verify) = task_command(
        &p,
        &["verify"],
        id,
        &["--owner", "env-agent", "--lease-token", token],
    );
    assert_eq!(exit, 3, "{verify}");
    assert_eq!(verify["event_persisted"], true);
    assert_eq!(
        verify["observation"],
        "environment_restored_unverified_policy"
    );
    let n = p.next();
    assert_eq!(n["repair_brief"]["history"]["awaiting_verification"], false);
    assert_eq!(n["repair_brief"]["task_id"], id);
    assert_eq!(n["repair_brief"]["disposition"], "needs_decision");
}
