#![cfg(unix)]
use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Project {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-check-eslint-{label}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let p = Self(root);
        fs::create_dir_all(p.0.join("frontend/node_modules/eslint/bin")).unwrap();
        fs::write(
            p.0.join("frontend/package.json"),
            r#"{"devDependencies":{"eslint":"10.11.0"}}"#,
        )
        .unwrap();
        fs::write(
            p.0.join("frontend/node_modules/eslint/package.json"),
            r#"{"name":"eslint","version":"10.11.0","bin":{"eslint":"bin/eslint.js"}}"#,
        )
        .unwrap();
        fs::write(
            p.0.join("frontend/node_modules/eslint/bin/eslint.js"),
            "fixture",
        )
        .unwrap();
        fs::write(
            p.0.join("frontend/eslint.config.cjs"),
            "module.exports = [];\n",
        )
        .unwrap();
        fs::write(p.0.join("frontend/app.js"), "debugger;\n").unwrap();
        fs::write(p.0.join("node"), "#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.11.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; source=$1; shift; done\nprintf '[{\"filePath\":\"%s\",\"messages\":[{\"ruleId\":\"no-debugger\",\"severity\":2,\"message\":\"fixture\",\"line\":1,\"column\":1}],\"suppressedMessages\":[],\"errorCount\":1,\"warningCount\":0,\"fatalErrorCount\":0,\"fixableErrorCount\":0,\"fixableWarningCount\":0}]' \"$source\" > \"$report\"\nexit 1\n").unwrap();
        fs::set_permissions(p.0.join("node"), fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
    fn check(&self) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "all"])
            .arg(&self.0)
            .arg("--node-tool")
            .arg(self.0.join("node"))
            .args(["--format=json", "--timeout", "60s"])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
#[test]
fn aggregate_eslint_preserves_findings_and_reuses_repair_tasks() {
    let p = Project::new("tasks");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    for round in 0..2 {
        let r = p.check();
        let scan = &r["native_results"]["node_lint"];
        assert_eq!(scan["status"], "local_observation", "{scan}");
        let files = scan["files"].as_array().unwrap();
        let app = files
            .iter()
            .find(|f| f["path"] == "frontend/app.js")
            .unwrap();
        assert_eq!(app["feedback"]["findings"][0]["rule_id"], "no-debugger");
        assert_eq!(scan["backlog_status"], "synced_partial", "{scan}");
        assert_eq!(scan["new_findings"], if round == 0 { 2 } else { 0 });
        assert!(!r["next"]["repair_brief"].is_null(), "{}", r["next"]);
        assert!(
            r["execution_tasks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["id"] == "node.lint" && t["status"] == "native_observed_unverified")
        );
        assert_eq!(r["delivery_decision"], "incomplete");
        #[cfg(feature = "wasm-precheck")]
        assert_eq!(r["syntax_candidates"]["native_preferred_count"], 2);
    }
}
#[test]
#[cfg(feature = "wasm-precheck")]
fn missing_module_context_keeps_fallback_while_completed_native_files_skip_it() {
    let p = Project::new("mixed");
    fs::create_dir(p.0.join("other")).unwrap();
    fs::write(p.0.join("other/package.json"), "{}").unwrap();
    fs::write(p.0.join("other/app.ts"), "const x: number = ;\n").unwrap();
    let r = p.check();
    assert_eq!(r["syntax_candidates"]["native_preferred_count"], 2);
    let rows = r["syntax_candidates"]["observations"].as_array().unwrap();
    assert!(!rows.iter().any(|r| r["path"] == "frontend/app.js"));
    assert!(
        rows.iter()
            .any(|r| r["path"] == "other/app.ts" && r["recovery_count"].as_u64().unwrap_or(0) > 0)
    );
    let files = r["native_results"]["node_lint"]["files"]
        .as_array()
        .unwrap();
    assert_eq!(
        files.iter().find(|f| f["path"] == "other/app.ts").unwrap()["feedback"]["local_coherent"],
        false
    );
}

#[test]
#[cfg(feature = "wasm-precheck")]
fn ignored_native_file_is_not_mistaken_for_completed_syntax_coverage() {
    let p = Project::new("ignored");
    let node = p.0.join("node");
    let script = fs::read_to_string(&node).unwrap().replace(
        "printf '[",
        "case \"$source\" in */app.js) printf '[{\"filePath\":\"%s\",\"messages\":[{\"ruleId\":null,\"severity\":1,\"message\":\"File ignored\"}],\"suppressedMessages\":[],\"errorCount\":0,\"warningCount\":1,\"fatalErrorCount\":0,\"fixableErrorCount\":0,\"fixableWarningCount\":0}]' \"$source\" > \"$report\"; exit 0;; esac\nprintf '[",
    );
    fs::write(&node, script).unwrap();
    let r = p.check();
    assert_eq!(r["syntax_candidates"]["native_preferred_count"], 1);
    assert!(
        r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"] == "frontend/app.js")
    );
    let files = r["native_results"]["node_lint"]["files"]
        .as_array()
        .unwrap();
    let ignored = files
        .iter()
        .find(|row| row["path"] == "frontend/app.js")
        .unwrap();
    assert_eq!(ignored["source_sha256"], Value::Null);
    assert_eq!(
        ignored["feedback"]["reason"],
        "eslint_unattributed_diagnostic"
    );
    assert!(!p.0.join(".codeguard").exists());
}

#[test]
fn config_gap_is_reported_without_running_local_eslint() {
    let p = Project::new("config-gap");
    fs::remove_file(p.0.join("frontend/eslint.config.cjs")).unwrap();
    fs::write(p.0.join("node"), "#!/bin/sh\nexit 99\n").unwrap();
    let r = p.check();
    let scan = &r["native_results"]["node_lint"];
    assert_eq!(
        scan["files"][0]["feedback"]["reason"],
        "eslint_config_selection_unresolved"
    );
    assert_eq!(scan["files"][0]["feedback"]["local_coherent"], false);
    assert_eq!(scan["files"][0]["source_sha256"], Value::Null);
}

#[test]
fn selected_workspace_does_not_execute_an_ancestor_projects_checker() {
    let p = Project::new("boundary");
    let child = p.0.join("frontend/child");
    fs::create_dir(&child).unwrap();
    fs::write(child.join("app.js"), "debugger;\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&child)
        .arg("--node-tool")
        .arg(p.0.join("node"))
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        r["native_results"]["node_lint"]["files"][0]["feedback"]["reason"],
        "eslint_execution_context_missing"
    );
}

#[test]
fn native_timeout_retains_an_explicit_incomplete_task() {
    let p = Project::new("timeout");
    fs::write(p.0.join("node"), "#!/bin/sh\n/bin/sleep 3\nexit 0\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&p.0)
        .arg("--node-tool")
        .arg(p.0.join("node"))
        .args(["--format=json", "--timeout", "200ms", "--jobs", "1"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        r["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "node.lint"
                && matches!(
                    task["status"].as_str(),
                    Some("deadline_exceeded" | "deadline_before_start")
                ))
    );
    assert_eq!(r["delivery_decision"], "incomplete");
}

#[test]
fn source_presence_alone_does_not_enqueue_required_lint_installation() {
    let p = Project::new("optional");
    fs::remove_dir_all(p.0.join("frontend/node_modules")).unwrap();
    fs::remove_file(p.0.join("frontend/eslint.config.cjs")).unwrap();
    fs::write(p.0.join("frontend/package.json"), "{}").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    for _ in 0..2 {
        let r = p.check();
        let scan = &r["native_results"]["node_lint"];
        assert_eq!(
            scan["files"][0]["feedback"]["reason"],
            "eslint_execution_context_missing"
        );
        assert_eq!(scan["new_blockers"], 0);
        assert_eq!(scan["backlog_status"], "not_connected");
        assert_eq!(scan["next"], Value::Null);
    }
}

fn edited_hook(p: &Project, paths: &[&str], claude: bool) -> Value {
    use std::io::Write;
    use std::process::Stdio;
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command.arg("hook");
    let payload = if claude {
        command.args(["claude", "post-tool-use"]);
        serde_json::json!({"hook_event_name":"PostToolUse","cwd":p.0,
            "tool_name":"Edit","tool_input":{"file_path":p.0.join(paths[0])},
            "tool_response":{"success":true}})
    } else {
        command.arg("execute");
        serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request",
            "input":{"event":"file_changed","changed_paths":paths,"task_id":null,
            "write_outcome":"confirmed","host_claims_blocking":false}})
    };
    let mut child = command
        .arg(&p.0)
        .arg("--node-tool")
        .arg(p.0.join("node"))
        .args(["--timeout=60s", "--format=json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&payload).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(if claude { 0 } else { 3 }),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn edited_javascript_uses_native_result_and_safe_dialogue() {
    let p = Project::new("hook-native");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let report = edited_hook(&p, &["frontend/app.js"], false);
    let feedback = &report["local_feedback"];
    assert_eq!(feedback["node_lint"]["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        feedback["node_lint"]["files"][0]["feedback"]["findings"][0]["rule_id"],
        "no-debugger"
    );
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(feedback["syntax_candidates"]["native_preferred_count"], 1);
    assert_eq!(feedback["node_lint"]["new_findings"], 1);
    let second = edited_hook(&p, &["frontend/app.js"], false);
    assert_eq!(second["local_feedback"]["node_lint"]["new_findings"], 0);
    let host = edited_hook(&p, &["frontend/app.js"], true);
    let context = host["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("no-debugger"), "{context}");
    assert!(context.contains("CG-"), "{context}");
    assert!(context.contains("task show"), "{context}");
    assert!(context.contains("交付未评估"));
    assert!(!context.contains("fixture"));
}

#[test]
fn edited_symlink_does_not_execute_native_tool_or_expand_scope() {
    let p = Project::new("hook-symlink");
    std::os::unix::fs::symlink("frontend", p.0.join("linked")).unwrap();
    let report = edited_hook(&p, &["linked/app.js"], false);
    let feedback = &report["local_feedback"];
    assert_eq!(feedback["unavailable_files"][0]["path"], "linked/app.js");
    assert!(
        feedback["node_lint"]["files"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn edited_missing_lint_uses_wasm_and_distinguishes_required_from_recommended() {
    let p = Project::new("hook-fallback");
    fs::remove_dir_all(p.0.join("frontend/node_modules")).unwrap();
    let report = edited_hook(&p, &["frontend/app.js"], false);
    assert_eq!(
        report["local_feedback"]["next_action"],
        "recommend_native_lint"
    );
    assert_eq!(
        report["local_feedback"]["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::write(p.0.join("frontend/app.js"), "const = ;\n").unwrap();
    let report = edited_hook(&p, &["frontend/app.js"], false);
    assert!(
        report["local_feedback"]["candidate_recovery_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        report["local_feedback"]["next_action"],
        "require_native_lint_confirmation"
    );
    let host = edited_hook(&p, &["frontend/app.js"], true);
    let context = host["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("必须准备或修复适用的原生"), "{context}");
    assert!(context.contains("不要仅凭候选结果修改源码"), "{context}");
    assert!(!context.contains("const ="));
}

#[test]
fn edited_native_sync_failure_remains_visible_in_dialogue() {
    let p = Project::new("hook-sync-failed");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    fs::remove_dir_all(p.0.join(".codeguard/reports")).unwrap();
    fs::write(p.0.join(".codeguard/reports"), "occupied").unwrap();
    let host = edited_hook(&p, &["frontend/app.js"], true);
    let context = host["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("no-debugger"), "{context}");
    assert!(context.contains("任务同步未完成"), "{context}");
    assert!(!context.contains("task show"));
}

#[test]
#[cfg(feature = "wasm-precheck")]
fn module_extensions_keep_native_priority_and_independent_fallback_scope() {
    let p = Project::new("module-extensions");
    fs::create_dir(p.0.join("other")).unwrap();
    for extension in ["mts", "cts"] {
        fs::write(
            p.0.join(format!("frontend/module.{extension}")),
            "debugger;\n",
        )
        .unwrap();
        fs::write(
            p.0.join(format!("other/module.{extension}")),
            "export const value: number = ;\n",
        )
        .unwrap();
    }
    let report = p.check();
    let files = report["native_results"]["node_lint"]["files"]
        .as_array()
        .unwrap();
    for extension in ["mts", "cts"] {
        let path = format!("frontend/module.{extension}");
        let file = files.iter().find(|f| f["path"] == path).unwrap();
        assert_eq!(file["feedback"]["local_coherent"], true, "{file}");
        assert_eq!(file["feedback"]["findings"][0]["rule_id"], "no-debugger");
    }
    assert_eq!(
        report["syntax_candidates"]["native_preferred_count"], 4,
        "{report}"
    );
    let rows = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    for extension in ["mts", "cts"] {
        assert!(
            !rows
                .iter()
                .any(|r| r["path"] == format!("frontend/module.{extension}"))
        );
        let row = rows
            .iter()
            .find(|r| r["path"] == format!("other/module.{extension}"))
            .unwrap();
        assert_eq!(row["language"], "typescript");
        assert!(row["recovery_count"].as_u64().unwrap() > 0, "{row}");
    }
    assert_eq!(report["delivery_decision"], "incomplete");
}
