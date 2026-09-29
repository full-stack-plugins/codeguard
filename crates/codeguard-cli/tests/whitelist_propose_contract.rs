#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use codeguard_adapters::bundled_ruff_rulepack;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new(configured: bool) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-whitelist-propose-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.py"), "import os\n").unwrap();
        if configured {
            fs::write(root.join("ruff.toml"), "[lint]\nselect = [\"F401\"]\n").unwrap();
        }
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        Self(root)
    }

    fn lint(&self, tool: Option<&str>) {
        let mut args = vec!["lint", "python", self.0.to_str().unwrap()];
        if let Some(tool) = tool {
            args.extend(["--ruff-tool", tool]);
        }
        args.push("--format=json");
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
    }

    fn task_id(&self) -> String {
        let ids = fs::read_dir(self.0.join(".codeguard/tasks"))
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .trim_end_matches(".md")
                    .to_owned()
            })
            .filter(|id| {
                let fact: Value = serde_json::from_slice(
                    &fs::read(
                        self.0
                            .join(format!(".codeguard/findings/{id}/finding.json")),
                    )
                    .unwrap(),
                )
                .unwrap();
                fact["reason_code"] != "python_syntax_confirmation_needed"
            })
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 1);
        ids[0].clone()
    }

    fn propose(&self, id: &str) -> (i32, Value) {
        self.propose_with_tool(id, None)
    }

    fn propose_with_tool(&self, id: &str, tool: Option<&str>) -> (i32, Value) {
        let mut args = vec![
            "rules",
            "whitelist",
            "propose",
            id,
            self.0.to_str().unwrap(),
        ];
        if let Some(tool) = tool {
            args.extend(["--ruff-tool", tool]);
        }
        args.push("--format=json");
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.stderr.is_empty());
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn correct(&self, id: &str, extra: &[&str]) -> (i32, Value) {
        let mut args = vec![
            "rules",
            "whitelist",
            "propose",
            id,
            self.0.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        args.push("--format=json");
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.stderr.is_empty());
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn verify(&self, id: &str, tool: &str) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                id,
                self.0.to_str().unwrap(),
                "--ruff-tool",
                tool,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

fn candidate(id: &str, decision_id: &str) -> Value {
    json!({
        "schema_version":"1.0", "kind":"false_positive", "id":decision_id,
        "identity":{
            "finding_id":id, "checker_id":"python.ruff", "native_rule_id":"F401",
            "category":"lint", "target":{"kind":"source","path":"app.py","file_sha256":"a".repeat(64)},
            "finding_fingerprint":"b".repeat(64), "tool_sha256":"c".repeat(64),
            "adapter_sha256":"d".repeat(64), "rulepack_sha256":"e".repeat(64)
        },
        "reason_code":"native_false_positive", "rationale":"最小复现待人工复核",
        "reproducer_ref":"evidence:one", "approved_policy_revision":"policy-r41",
        "approval_ref":"review:one", "reviewer":"reviewer-one",
        "created_at":1790000000_u64, "expires_at":1791000000_u64
    })
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn environment_blocker_cannot_become_false_positive_candidate() {
    let project = Project::new(false);
    project.lint(None);
    let id = project.task_id();
    let (exit, report) = project.propose(&id);
    assert_eq!(exit, 3);
    assert_eq!(report["status"], "not_a_finding");
    assert!(report["candidate"].is_null());
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/decisions"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn missing_task_does_not_produce_candidate() {
    let project = Project::new(false);
    let (exit, report) = project.propose("CG-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(exit, 3);
    assert!(report["candidate"].is_null());
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/decisions"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn proposal_rejects_relative_or_repeated_native_tool_before_reading_tasks() {
    let project = Project::new(false);
    for extra in [
        vec!["--ruff-tool", "ruff"],
        vec!["--ruff-tool", "/tmp/ruff", "--ruff-tool", "/tmp/other"],
    ] {
        let mut args = vec![
            "rules",
            "whitelist",
            "propose",
            "CG-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            project.0.to_str().unwrap(),
        ];
        args.extend(extra);
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn correction_of_an_environment_blocker_never_creates_a_proposal() {
    let project = Project::new(false);
    project.lint(None);
    let id = project.task_id();
    let prior = project.0.join("prior.json");
    fs::write(
        &prior,
        serde_json::to_vec(&candidate(&id, "CG-FP-1")).unwrap(),
    )
    .unwrap();
    let (exit, report) = project.correct(
        &id,
        &[
            "--correct-decision",
            prior.to_str().unwrap(),
            "--correction-reason",
            "erroneous_approval",
            "--verification-run",
            "lint-missing-1",
            "--record",
        ],
    );
    assert_eq!(exit, 3);
    assert_eq!(report["status"], "not_a_finding");
    assert!(report["proposal"].is_null());
    assert_eq!(report["gate_effect"], "none");
    assert!(report.get("record_ref").is_none());
}

#[test]
fn correction_preview_schema_forbids_gate_authority() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/whitelist-correction-preview.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["gate_effect"]["const"], "none");
    assert_eq!(schema["properties"]["authority"]["const"], "unverified");
    assert_eq!(schema["$defs"]["proposal"]["additionalProperties"], false);
    let event_schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/whitelist-correction-event.schema.json"
    ))
    .unwrap();
    assert_eq!(event_schema["additionalProperties"], false);
    assert_eq!(
        event_schema["properties"]["authority"]["const"],
        "unverified"
    );
    assert_eq!(event_schema["properties"]["gate_effect"]["const"], "none");
    assert!(!event_schema.to_string().contains(".schema.json"));
}

#[test]
#[ignore = "requires pinned native Ruff executable via CODEGUARD_RUFF_BIN"]
fn correction_preview_requires_current_original_checker_receipt_and_never_self_approves() {
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let project = Project::new(true);
    project.lint(Some(&tool));
    let id = project.task_id();
    let prior = project.0.join("prior.json");
    fs::write(
        &prior,
        serde_json::to_vec(&candidate(&id, "CG-FP-1")).unwrap(),
    )
    .unwrap();
    let fields = [
        "--correct-decision",
        prior.to_str().unwrap(),
        "--correction-reason",
        "erroneous_approval",
        "--verification-run",
        "lint-missing-1",
    ];
    let (_, before) = project.correct(&id, &fields);
    assert_eq!(before["status"], "verification_required");
    assert!(before["proposal"].is_null());

    let verified = project.verify(&id, &tool);
    assert_eq!(verified["observation"], "still_present");
    let run_id = verified["native_scan"]["run_id"].as_str().unwrap();
    let mut fields = fields.to_vec();
    fields[5] = run_id;
    let (exit, report) = project.correct(&id, &fields);
    assert_eq!(exit, 3);
    assert_eq!(report["status"], "review_required");
    let mut wrong_run = fields.clone();
    wrong_run[5] = "lint-missing-1";
    let (_, mismatch) = project.correct(&id, &wrong_run);
    assert_eq!(mismatch["status"], "verification_reference_mismatch");
    assert!(mismatch["proposal"].is_null());
    assert_eq!(report["proposal"]["old_decision_id"], "CG-FP-1");
    assert_eq!(report["proposal"]["finding_id"], id);
    assert_eq!(report["proposal"]["verification_ref"]["run_id"], run_id);
    assert_eq!(
        report["proposal"]["revoke_decision_ids"],
        json!(["CG-FP-1"])
    );
    assert!(report["proposal"]["replacement_candidate"].is_null());
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["gate_effect"], "none");

    let mut recorded_fields = fields.clone();
    recorded_fields.push("--record");
    let (_, recorded) = project.correct(&id, &recorded_fields);
    assert_eq!(recorded["proposal"], report["proposal"]);
    let reference = recorded["record_ref"].as_str().unwrap();
    assert!(reference.starts_with(&format!(
        ".codeguard/findings/{id}/events/correction-proposed-"
    )));
    let event_path = project.0.join(reference);
    let first_bytes = fs::read(&event_path).unwrap();
    assert_eq!(recorded["projection_status"], "recorded");
    let projection_ref = recorded["projection_ref"].as_str().unwrap();
    let projection_path = project.0.join(projection_ref);
    let projection = fs::read_to_string(&projection_path).unwrap();
    for section in [
        "问题证据",
        "规则依据",
        "允许修改的范围",
        "修复步骤",
        "复检命令",
        "历史尝试",
        "关闭条件",
    ] {
        assert!(projection.contains(section), "{section}: {projection}");
    }
    assert!(projection.contains(reference));
    assert!(projection.contains("CG-FP-1"));
    assert!(projection.contains("python.ruff"));
    assert!(projection.contains("F401"));
    assert!(projection.contains("app.py"));
    assert!(projection.contains("待独立评审"));
    let event: Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(event["task_id"], id);
    assert_eq!(event["proposal"], report["proposal"]);
    assert_eq!(event["authority"], "unverified");
    assert_eq!(event["gate_effect"], "none");
    let (_, replay) = project.correct(&id, &recorded_fields);
    assert_eq!(replay["record_ref"], reference);
    assert_eq!(fs::read(event_path).unwrap(), first_bytes);
    assert_eq!(replay["projection_ref"], projection_ref);
    fs::write(&projection_path, "用户维护的附件\n").unwrap();
    let (_, conflict) = project.correct(&id, &recorded_fields);
    assert_eq!(conflict["record_ref"], reference);
    assert_eq!(conflict["projection_status"], "failed");
    assert_eq!(
        fs::read_to_string(&projection_path).unwrap(),
        "用户维护的附件\n"
    );
    let changed_brief = codeguard_cli::next_command::read_task_brief(&project.0, &id).unwrap();
    assert!(changed_brief.get("correction_task_refs").is_none());
    assert_eq!(changed_brief["disposition"], "needs_decision");
    fs::remove_file(&projection_path).unwrap();
    let (_, rebuilt) = project.correct(&id, &recorded_fields);
    assert_eq!(rebuilt["projection_status"], "recorded");
    assert_eq!(fs::read_to_string(&projection_path).unwrap(), projection);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(next.status.code(), Some(0));
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(next["repair_brief"]["task_id"], id);
    assert_eq!(next["disposition"], "needs_decision");
    assert_eq!(
        next["repair_brief"]["correction_proposal_refs"],
        json!([reference])
    );
    assert_eq!(next["delivery_decision"], "not_evaluated");
    assert_eq!(
        next["repair_brief"]["correction_task_refs"],
        json!([projection_ref])
    );
    let config = project.0.join("ruff.toml");
    let original_config = fs::read(&config).unwrap();
    for change in ["bytes", "priority"] {
        if change == "bytes" {
            fs::write(&config, "[lint]\nselect = [\"E501\"]\n").unwrap();
        } else {
            fs::write(project.0.join(".ruff.toml"), &original_config).unwrap();
        }
        let brief = codeguard_cli::next_command::read_task_brief(&project.0, &id).unwrap();
        assert_eq!(
            brief["disposition"], "verification_required",
            "{change}: {brief}"
        );
        assert!(brief.get("correction_proposal_refs").is_none());
        assert!(brief.get("correction_task_refs").is_none());
        let (_, changed) = project.correct(&id, &recorded_fields);
        assert_eq!(changed["status"], "verification_required");
        assert!(changed["proposal"].is_null());
        fs::write(&config, &original_config).unwrap();
        if change == "priority" {
            fs::remove_file(project.0.join(".ruff.toml")).unwrap();
        }
    }

    let replacement = project.0.join("replacement.json");
    let mut revised = candidate(&id, "CG-FP-2");
    revised["schema_version"] = json!("1.1");
    revised["replaces_decision_id"] = json!("CG-FP-1");
    let (_, current) = project.propose(&id);
    let observed = &current["observed_artifacts"];
    revised["identity"]["target"]["file_sha256"] = observed["source_sha256"].clone();
    revised["identity"]["finding_fingerprint"] = observed["finding_fingerprint"].clone();
    revised["identity"]["tool_sha256"] = observed["tool_sha256"].clone();
    revised["identity"]["rulepack_sha256"] = observed["rulepack_sha256"].clone();
    fs::write(&replacement, serde_json::to_vec(&revised).unwrap()).unwrap();
    let mut with_replacement = fields.clone();
    with_replacement.extend(["--replacement", replacement.to_str().unwrap()]);
    let (_, proposed) = project.correct(&id, &with_replacement);
    assert_eq!(proposed["status"], "evidence_incomplete");
    assert_eq!(
        proposed["proposal"]["replacement_candidate"]["id"],
        "CG-FP-2"
    );
    assert_eq!(
        proposed["proposal"]["replacement_candidate"]["replaces_decision_id"],
        "CG-FP-1"
    );
    revised["identity"]["target"]["path"] = json!("other.py");
    fs::write(&replacement, serde_json::to_vec(&revised).unwrap()).unwrap();
    let (_, escaped) = project.correct(&id, &with_replacement);
    assert_eq!(escaped["status"], "replacement_scope_mismatch");
    assert!(escaped["proposal"].is_null());
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/decisions"))
            .unwrap()
            .count(),
        0
    );
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    let fixed = project.verify(&id, &tool);
    assert_eq!(fixed["observation"], "candidate_absent_unverified_policy");
    let fixed_run = fixed["native_scan"]["run_id"].as_str().unwrap();
    let revoke_only = [
        "--correct-decision",
        prior.to_str().unwrap(),
        "--correction-reason",
        "root_cause_fixed",
        "--verification-run",
        fixed_run,
    ];
    let (_, fixed_proposal) = project.correct(&id, &revoke_only);
    assert_eq!(fixed_proposal["status"], "review_required");
    assert!(fixed_proposal["proposal"]["replacement_candidate"].is_null());
    fs::write(&config, "[lint]\nselect = [\"E501\"]\n").unwrap();
    let (_, changed_absence) = project.correct(&id, &revoke_only);
    assert_eq!(changed_absence["status"], "verification_required");
    assert!(changed_absence["proposal"].is_null());
    fs::write(&config, &original_config).unwrap();
    fs::write(
        project.0.join("app.py"),
        "pass\n# changed after verification\n",
    )
    .unwrap();
    let (_, stale) = project.correct(&id, &revoke_only);
    assert_eq!(stale["status"], "verification_required");
    assert!(stale["proposal"].is_null());
    let mut stale_record = revoke_only.to_vec();
    stale_record.push("--record");
    let (_, stale_recorded) = project.correct(&id, &stale_record);
    assert!(stale_recorded["proposal"].is_null());
    assert!(stale_recorded.get("record_ref").is_none());
    assert_eq!(
        fs::read_dir(project.0.join(format!(".codeguard/findings/{id}/events")))
            .unwrap()
            .filter(|entry| entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("correction-proposed-"))
            .count(),
        1
    );
}

#[test]
#[ignore = "requires pinned native Ruff executable via CODEGUARD_RUFF_BIN"]
fn native_finding_binds_current_adapter_but_never_self_approves() {
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let project = Project::new(true);
    project.lint(Some(&tool));
    let id = project.task_id();
    let (exit, report) = project.propose(&id);
    assert_eq!(exit, 3);
    assert_eq!(report["status"], "evidence_incomplete");
    assert_eq!(report["schema_version"], "0.4.0");
    assert_eq!(report["finding_id"], id);
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["gate_effect"], "none");
    assert!(report["candidate"].is_null());
    let (_, same_tool) = project.propose_with_tool(&id, Some(&tool));
    assert_eq!(same_tool["status"], "evidence_incomplete");
    assert_eq!(
        same_tool["observed_artifacts"]["tool_sha256"],
        report["observed_artifacts"]["tool_sha256"]
    );
    let wrong_tool = project.0.join("other-ruff");
    fs::write(&wrong_tool, b"not the scanned binary").unwrap();
    let (_, changed_tool) = project.propose_with_tool(&id, Some(wrong_tool.to_str().unwrap()));
    assert_eq!(changed_tool["status"], "native_tool_changed_since_scan");
    assert!(changed_tool["observed_artifacts"].is_null());
    assert!(changed_tool["candidate"].is_null());
    let (_, missing_tool) = project.propose_with_tool(&id, Some("/nonexistent/codeguard-ruff"));
    assert_eq!(missing_tool["status"], "native_tool_unavailable");
    assert!(missing_tool["observed_artifacts"].is_null());
    assert_eq!(
        report["recheck_argv"],
        serde_json::json!(["codeguard", "lint", "python", "."])
    );
    assert_eq!(
        report["observed_artifacts"]["tool_sha256"],
        format!("{:x}", Sha256::digest(fs::read(&tool).unwrap()))
    );
    assert_eq!(
        report["observed_artifacts"]["config_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(project.0.join("ruff.toml")).unwrap())
        )
    );
    assert_eq!(
        report["observed_artifacts"]["rulepack_sha256"],
        bundled_ruff_rulepack().unwrap().sha256
    );
    assert_eq!(
        report["observed_artifacts"]["rulepack_status"],
        "candidate_unapproved"
    );
    assert_eq!(
        report["observed_artifacts"]["adapter_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
        )
    );
    assert!(
        !report["missing_evidence"]
            .as_array()
            .unwrap()
            .contains(&Value::String("tool_sha256".into()))
    );
    assert!(
        !report["missing_evidence"]
            .as_array()
            .unwrap()
            .contains(&Value::String("adapter_sha256".into()))
    );
    assert!(
        report["missing_evidence"]
            .as_array()
            .unwrap()
            .contains(&Value::String("approved_rulepack_identity".into()))
    );
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/decisions"))
            .unwrap()
            .count(),
        0
    );

    let run_id = report["observed_artifacts"]["run_id"].as_str().unwrap();
    let original_path = project.0.join(format!(".codeguard/reports/{run_id}.json"));
    let original_report = fs::read(&original_path).unwrap();
    let marker_path = project
        .0
        .join(format!(".codeguard/state/consumed/{run_id}.json"));
    let original_marker = fs::read(&marker_path).unwrap();
    let mut tampered: Value = serde_json::from_slice(&original_report).unwrap();
    tampered["adapter_sha256"] = json!("1".repeat(64));
    let tampered_bytes = serde_json::to_vec(&tampered).unwrap();
    fs::write(&original_path, &tampered_bytes).unwrap();
    let mut marker: Value = serde_json::from_slice(&original_marker).unwrap();
    marker["report_sha256"] = json!(format!("{:x}", Sha256::digest(&tampered_bytes)));
    fs::write(&marker_path, serde_json::to_vec_pretty(&marker).unwrap()).unwrap();
    let (_, wrong_adapter) = project.propose(&id);
    assert_eq!(wrong_adapter["status"], "adapter_binary_changed_since_scan");
    assert!(wrong_adapter["observed_artifacts"].is_null());
    assert!(wrong_adapter["candidate"].is_null());
    fs::write(&original_path, original_report).unwrap();
    fs::write(&marker_path, original_marker).unwrap();

    let (prefix, nanos) = run_id.rsplit_once('-').unwrap();
    let later = format!("{prefix}-{}", nanos.parse::<u128>().unwrap() + 1);
    let mut unconsumed: Value = serde_json::from_slice(
        &fs::read(project.0.join(format!(".codeguard/reports/{run_id}.json"))).unwrap(),
    )
    .unwrap();
    unconsumed["run_id"] = Value::String(later.clone());
    let unconsumed_path = project.0.join(format!(".codeguard/reports/{later}.json"));
    fs::write(&unconsumed_path, serde_json::to_vec(&unconsumed).unwrap()).unwrap();
    let (_, pending) = project.propose(&id);
    assert_eq!(pending["status"], "latest_report_not_synced");
    assert!(pending["observed_artifacts"].is_null());
    fs::remove_file(&unconsumed_path).unwrap();

    let config_path = project.0.join("ruff.toml");
    let original_config = fs::read(&config_path).unwrap();
    fs::write(&config_path, "[lint]\nselect = [\"E501\"]\n").unwrap();
    let (_, changed_config) = project.propose(&id);
    assert_eq!(changed_config["status"], "latest_configuration_changed");
    assert!(changed_config["observed_artifacts"].is_null());
    fs::write(&config_path, original_config).unwrap();
    fs::write(
        project.0.join(".ruff.toml"),
        "[lint]\nselect = [\"F401\"]\n",
    )
    .unwrap();
    let (_, priority_changed) = project.propose(&id);
    assert_eq!(priority_changed["status"], "latest_configuration_changed");
    assert!(priority_changed["observed_artifacts"].is_null());
    fs::remove_file(project.0.join(".ruff.toml")).unwrap();

    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    let (_, stale) = project.propose(&id);
    assert_eq!(stale["status"], "finding_not_in_latest_scan");
    assert!(stale["observed_artifacts"].is_null());
    assert!(stale["candidate"].is_null());
    project.lint(Some(&tool));
    let (_, clean) = project.propose(&id);
    assert_eq!(clean["status"], "finding_not_in_latest_scan");
}
