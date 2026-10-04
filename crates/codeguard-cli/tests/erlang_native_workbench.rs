#![cfg(unix)]

use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-erl-native-work-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.erl"), "-module(app).\nf( -> ok.\n").unwrap();
        let p = Self(root);
        assert_eq!(
            p.command()
                .args(["init"])
                .arg(&p.0)
                .arg("--apply")
                .output()
                .unwrap()
                .status
                .code(),
            Some(3)
        );
        p
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS");
        c
    }
    fn tool(&self) -> PathBuf {
        let tool = self.0.join("erl-tool");
        fs::write(&tool,r#"#!/bin/sh
case "$*" in *system_info*) printf 'OTP 28\n'; exit 0;; esac
input=$(/bin/cat)
case "$input" in
*'f( ->'*) printf '%s' '{"schema_version":"0.1.0","forms":2,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[{"line":2,"column":4,"rule_id":"erlang.syntax.error"}]}' ;;
*) printf '%s' '{"schema_version":"0.1.0","forms":2,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[]}' ;;
esac
"#).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, tool: &Path) -> Value {
        decode(
            &self
                .command()
                .args(["check", "all"])
                .arg(&self.0)
                .arg("--erl-tool")
                .arg(tool)
                .args(["--timeout", "30s", "--format=json"])
                .output()
                .unwrap(),
        )
    }
    fn next(&self) -> Value {
        decode(
            &self
                .command()
                .arg("next")
                .arg(&self.0)
                .arg("--format=json")
                .output()
                .unwrap(),
        )
    }
    fn verify(&self, id: &str, tool: &Path) -> Value {
        decode(
            &self
                .command()
                .args(["task", "verify", id])
                .arg(&self.0)
                .arg("--erl-tool")
                .arg(tool)
                .args(["--timeout", "30s", "--format=json"])
                .output()
                .unwrap(),
        )
    }
}
fn decode(out: &Output) -> Value {
    assert!(
        matches!(out.status.code(), Some(0 | 3)),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}

#[test]
fn native_first_scan_creates_one_task_and_supports_original_tool_recheck() {
    let p = Project::new();
    let tool = p.tool();
    let first = p.check(&tool);
    let scan = &first["native_results"]["erlang_lint"];
    let id = scan["files"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{scan}"));
    assert!(id.starts_with("CG-B-"));
    assert_eq!(scan["task_status"], "synced_partial");
    assert_eq!(
        p.check(&tool)["native_results"]["erlang_lint"]["files"][0]["task_id"],
        id
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    let next = p.next();
    let b = &next["repair_brief"];
    assert_eq!(b["task_id"], id);
    assert_eq!(b["action_id"], "repair-source");
    assert_eq!(b["native_confirmation_status"], "diagnostics_observed");
    assert_eq!(b["native_diagnostic_positions"][0]["line"], 2);
    let bad = p.verify(id, &tool);
    assert_eq!(bad["event_persisted"], true);
    assert_eq!(bad["observation"], "still_blocked");
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "stale"
    );
    let fixed = p.verify(id, &tool);
    assert_eq!(fixed["event_persisted"], true);
    assert_eq!(fixed["observation"], "candidate_absent_unverified_policy");
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(p.0.join("app.erl"), "-module(app).\nf( -> ok.\n").unwrap();
    assert_eq!(
        p.check(&tool)["native_results"]["erlang_lint"]["files"][0]["task_id"],
        id
    );
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "diagnostics_observed"
    );
    assert_eq!(
        p.next()["repair_brief"]["native_diagnostic_positions"],
        json!([{"line":2,"column":4,"rule_id":"erlang.syntax.error"}])
    );
}

#[test]
fn native_first_task_recheck_discovers_path_and_preserves_origin_protocol() {
    let p = Project::new();
    let tool = p.tool();
    let first = p.check(&tool);
    let id = first["native_results"]["erlang_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    let bin = p.0.join("path-bin");
    fs::create_dir(&bin).unwrap();
    std::os::unix::fs::symlink(&tool, bin.join("erl")).unwrap();
    let verify = || {
        decode(
            &p.command()
                .args(["task", "verify", id])
                .arg(&p.0)
                .args(["--timeout", "30s", "--format=json"])
                .env("PATH", &bin)
                .output()
                .unwrap(),
        )
    };
    let bad = verify();
    assert_eq!(bad["schema_version"], "0.14.0");
    assert_eq!(bad["native_scan"]["schema_version"], "0.3.0");
    assert_eq!(bad["event_persisted"], true, "{bad}");
    assert_eq!(bad["native_scan"]["tool_path"], tool.to_str().unwrap());
    assert_eq!(bad["observation"], "still_blocked");
    assert_eq!(p.next()["repair_brief"]["task_id"], id);
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    let good = verify();
    assert_eq!(good["observation"], "candidate_absent_unverified_policy");
    assert_eq!(good["event_persisted"], true, "{good}");
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn missing_native_tool_creates_environment_task_without_source_positions() {
    let p = Project::new();
    let out = p
        .command()
        .args(["check", "all"])
        .arg(&p.0)
        .args(["--timeout", "30s", "--format=json"])
        .env("PATH", &p.0)
        .output()
        .unwrap();
    let report = decode(&out);
    let file = &report["native_results"]["erlang_lint"]["files"][0];
    assert!(file["task_id"].as_str().is_some(), "{file}");
    let brief = &p.next()["repair_brief"];
    assert_eq!(brief["action_id"], "restore-checker-environment");
    assert_eq!(
        brief["native_confirmation_reason"],
        "erlang_tool_not_found_on_path"
    );
    assert_eq!(brief["native_diagnostic_positions"], json!([]));
}

#[test]
fn native_clean_first_scan_does_not_create_a_repair_task() {
    let p = Project::new();
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    let tool = p.tool();
    let report = p.check(&tool);
    assert_eq!(
        report["native_results"]["erlang_lint"]["files"][0]["native"]["status"],
        "completed"
    );
    assert_eq!(
        report["native_results"]["erlang_lint"]["files"][0]["task_id"],
        Value::Null
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        0
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn native_environment_and_preprocessing_failures_never_request_source_repair() {
    for (from, to, reason) in [
        (
            "OTP 28",
            "OTP 27",
            "erlang_version_unverified_or_unsupported",
        ),
        (
            "\"preprocessing\":false",
            "\"preprocessing\":true",
            "erlang_preprocessing_unresolved",
        ),
    ] {
        let p = Project::new();
        let tool = p.tool();
        let text = fs::read_to_string(&tool).unwrap().replace(from, to);
        fs::write(&tool, text).unwrap();
        let r = p.check(&tool);
        let file = &r["native_results"]["erlang_lint"]["files"][0];
        assert!(file["task_id"].as_str().is_some(), "{file}");
        assert_eq!(file["findings"], json!([]));
        let b = &p.next()["repair_brief"];
        assert_eq!(b["action_id"], "restore-checker-environment");
        assert_eq!(b["native_confirmation_reason"], reason);
        assert_eq!(b["native_diagnostic_positions"], json!([]));
    }
}

#[test]
fn persistence_failure_keeps_real_diagnostics_and_does_not_invent_task_id() {
    let p = Project::new();
    fs::remove_dir(p.0.join(".codeguard/reports")).unwrap();
    fs::write(p.0.join(".codeguard/reports"), "not a directory").unwrap();
    let report = p.check(&p.tool());
    let scan = &report["native_results"]["erlang_lint"];
    let file = &scan["files"][0];
    assert_eq!(scan["task_status"], "incomplete");
    assert_eq!(file["task_id"], Value::Null);
    assert!(file["task_sync_reason"].as_str().is_some());
    assert_eq!(file["findings"][0]["line"], 2);
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn changed_tool_or_consumption_record_withdraws_native_guidance() {
    for tamper_marker in [false, true] {
        let p = Project::new();
        let tool = p.tool();
        p.check(&tool);
        let next = p.next();
        let evidence = &next["repair_brief"]["native_confirmation_ref"];
        if tamper_marker {
            let run = evidence["run_id"].as_str().unwrap();
            let path = p.0.join(format!(".codeguard/state/consumed/{run}.json"));
            let mut marker: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            marker["report_sha256"] = json!("0".repeat(64));
            fs::write(path, serde_json::to_vec_pretty(&marker).unwrap()).unwrap();
        } else {
            let mut text = fs::read_to_string(&tool).unwrap();
            text.push_str("\n# changed tool identity\n");
            fs::write(&tool, text).unwrap();
        }
        let next = p.next();
        if tamper_marker {
            assert_eq!(next["disposition"], "verification_required");
            assert_eq!(next["reason"], "consumed_marker_invalid");
            assert_eq!(next["repair_brief"], Value::Null);
            continue;
        }
        assert_eq!(
            next["repair_brief"]["native_confirmation_status"], "stale",
            "{next}"
        );
        assert_eq!(
            next["repair_brief"]["native_diagnostic_positions"],
            json!([])
        );
        assert_ne!(next["repair_brief"]["action_id"], "repair-source");
    }
}

#[test]
fn repair_ready_hook_rechecks_native_first_task_and_returns_persisted_evidence() {
    use std::io::Write;
    use std::process::Stdio;
    let p = Project::new();
    let tool = p.tool();
    let first = p.check(&tool);
    let id = first["native_results"]["erlang_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    let mut c = p.command();
    c.args(["hook", "execute"])
        .arg(&p.0)
        .arg("--erl-tool")
        .arg(&tool)
        .args(["--timeout", "30s", "--format=json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = c.spawn().unwrap();
    let request = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{
        "event":"repair_ready","changed_paths":[],"task_id":id,"write_outcome":"confirmed","host_claims_blocking":false}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let report = decode(&out);
    let summary = &report["local_feedback"];
    assert_eq!(summary["event_persisted"], true, "{report}");
    assert_eq!(
        summary["native_confirmation_status"],
        "diagnostics_observed"
    );
    let reference = &summary["native_confirmation_ref"];
    let saved = p.0.join(reference["report_ref"].as_str().unwrap());
    let scan: Value = serde_json::from_slice(&fs::read(saved).unwrap()).unwrap();
    assert_eq!(scan["schema_version"], "0.3.0");
    assert_eq!(scan["original_report"]["grammar_sha256"], Value::Null);
    assert_eq!(summary["native_diagnostic_positions"][0]["line"], 2);
    assert_eq!(summary["native_column_unit"], "unicode_scalar");
}

#[test]
fn unconsumed_forged_native_reports_are_rejected_before_creating_tasks() {
    for mutation in 0..10 {
        let p = Project::new();
        let first = p.check(&p.tool());
        let id = first["native_results"]["erlang_lint"]["files"][0]["task_id"]
            .as_str()
            .unwrap();
        let next = p.next();
        let reference = &next["repair_brief"]["native_confirmation_ref"];
        let run = reference["run_id"].as_str().unwrap();
        let path = p.0.join(reference["report_ref"].as_str().unwrap());
        let mut report: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        fs::remove_file(p.0.join(format!(".codeguard/state/consumed/{run}.json"))).unwrap();
        fs::remove_dir_all(p.0.join(format!(".codeguard/findings/{id}"))).unwrap();
        fs::remove_file(p.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        match mutation {
            0 => report["schema_version"] = json!("99.0.0"),
            1 => report["delivery_decision"] = json!("allow"),
            2 => report["native_evidence"]["native"]["diagnostics"][0]["line"] = json!(9999),
            3 => report["native_evidence"]["target"]["source_sha256"] = json!("0".repeat(64)),
            4 => report["native_evidence"]["native"]["tool_sha256"] = json!("0".repeat(64)),
            5 => report["native_evidence"]["native"]["preprocessing_unresolved"] = json!(true),
            6 => report["native_evidence"]["native"]["version"] = json!("OTP 27"),
            7 => report["native_evidence"]["target"]["language"] = json!("zig"),
            8 => report["observations"] = json!([{"grammar_sha256":"0".repeat(64)}]),
            _ => {
                report["native_evidence"]["native"]["diagnostics"][0]["rule_id"] =
                    json!("wrong.rule")
            }
        }
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        let synced = decode(
            &p.command()
                .args(["work", "sync"])
                .arg(&p.0)
                .arg("--format=json")
                .output()
                .unwrap(),
        );
        assert_eq!(synced["failed_reports"], 1, "mutation {mutation}: {synced}");
        assert_eq!(
            fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
            0
        );
    }
}

#[test]
#[ignore = "requires explicitly selected real OTP 28"]
fn real_otp_native_first_task_repair_and_recheck() {
    let p = Project::new();
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_TEST_ERL").expect("select real OTP 28"));
    let first = p.check(&tool);
    let scan = &first["native_results"]["erlang_lint"];
    let id = scan["files"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{scan}"));
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "diagnostics_observed"
    );
    assert_eq!(p.verify(id, &tool)["observation"], "still_blocked");
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    let fixed = p.verify(id, &tool);
    assert_eq!(fixed["event_persisted"], true, "{fixed}");
    assert_eq!(fixed["observation"], "candidate_absent_unverified_policy");
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn standalone_lint_binds_nearest_initialized_workspace_and_reuses_check_task() {
    let p = Project::new();
    let tool = p.tool();
    let out = p
        .command()
        .args(["lint", "erlang"])
        .arg(p.0.join("app.erl"))
        .arg("--erl-tool")
        .arg(&tool)
        .arg("--format=json")
        .current_dir("/")
        .output()
        .unwrap();
    let lint = decode(&out);
    let id = lint["task_id"].as_str().unwrap_or_else(|| panic!("{lint}"));
    assert_eq!(lint["workspace_binding"], "bound");
    assert_eq!(
        p.check(&tool)["native_results"]["erlang_lint"]["files"][0]["task_id"],
        id
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "diagnostics_observed"
    );
}

#[test]
fn damaged_nearest_workspace_is_not_bypassed_to_create_an_outer_task() {
    let p = Project::new();
    let inner = p.0.join("module");
    fs::create_dir(&inner).unwrap();
    fs::create_dir(inner.join(".codeguard")).unwrap();
    fs::write(inner.join(".codeguard/workspace.json"), "{}").unwrap();
    fs::write(inner.join("app.erl"), "-module(app).\nf( -> ok.\n").unwrap();
    let out = p
        .command()
        .args(["lint", "erlang"])
        .arg(inner.join("app.erl"))
        .arg("--erl-tool")
        .arg(p.tool())
        .arg("--format=json")
        .output()
        .unwrap();
    let report = decode(&out);
    assert_eq!(report["workspace_binding"], "invalid");
    assert_eq!(report["task_sync_reason"], "workspace_invalid");
    assert_eq!(report["task_id"], Value::Null);
    assert_eq!(report["native"]["status"], "diagnostics_observed");
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        0
    );
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn wasm_first_and_native_first_observations_share_one_task_identity() {
    use std::io::Write;
    use std::process::Stdio;
    let p = Project::new();
    let mut c = p.command();
    c.args(["hook", "execute"])
        .arg(&p.0)
        .args(["--timeout", "30s", "--format=json"])
        .env("PATH", &p.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request",
        "input":{"event":"file_changed","changed_paths":["app.erl"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
    let report = decode(&child.wait_with_output().unwrap());
    let id = report["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{report}"));
    let tool = p.tool();
    assert_eq!(
        p.check(&tool)["native_results"]["erlang_lint"]["files"][0]["task_id"],
        id
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "diagnostics_observed"
    );
    let recheck = p.verify(id, &tool);
    assert_eq!(recheck["event_persisted"], true, "{recheck}");
    assert_eq!(recheck["native_scan"]["schema_version"], "0.2.0");
    assert!(
        recheck["native_scan"]["original_report"]["grammar_sha256"]
            .as_str()
            .is_some()
    );
}

#[test]
fn native_first_task_document_has_evidence_scope_and_original_tool_guidance() {
    let p = Project::new();
    let first = p.check(&p.tool());
    let id = first["native_results"]["erlang_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    let task = fs::read_to_string(p.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    for field in [
        "问题证据",
        "规则依据",
        "允许修改范围",
        "修复步骤",
        "复检命令",
        "历史尝试",
        "关闭条件",
    ] {
        assert!(task.contains(field), "{field}: {task}");
    }
    assert!(task.contains("原生诊断或环境阻塞"), "{task}");
    assert!(task.contains("--erl-tool"), "{task}");
    assert!(!task.contains("含固定 grammar"), "{task}");
    assert!(
        !task.contains("当前未接入该语言的原生确认 adapter"),
        "{task}"
    );
}
