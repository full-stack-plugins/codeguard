#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
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
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-kotlin-first-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("App.kt"), source).unwrap();
        let p = Self(root);
        p.run(&["init", "--apply"]);
        p
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.env("PATH", &self.0)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS");
        c
    }
    fn run(&self, args: &[&str]) -> Value {
        let mut c = self.command();
        match args {
            ["check", rest @ ..] => {
                c.arg("check").args(rest).arg(&self.0);
            }
            ["task", "verify", id, rest @ ..] => {
                c.args(["task", "verify", id]).arg(&self.0).args(rest);
            }
            [command, rest @ ..] => {
                c.arg(command).arg(&self.0).args(rest);
            }
            _ => panic!(),
        };
        let out = c.arg("--format=json").output().unwrap();
        assert!(matches!(out.status.code(), Some(0 | 3)), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn tool(&self, version: &str) {
        let body = format!(
            "#!/bin/sh\nif [ \"$1\" = -version ]; then echo 'info: kotlinc-jvm {version} (JRE 21)' >&2; exit 0; fi\ninput=$(/bin/cat \"$1\")\ncase \"$input\" in *'x: )'*) printf '%s:1:10: error: [SYNTAX] Type expected.\\nfun f(x: ) = x\\n         ^\\n' \"$1\" >&2; exit 1;; *) exit 0;; esac\n"
        );
        let p = self.0.join("kotlinc");
        fs::write(&p, body).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fn hook(&self) -> Value {
        self.hook_with_tool(false)
    }
    fn hook_with_tool(&self, explicit: bool) -> Value {
        let mut c = self.command();
        c.args(["hook", "execute"])
            .arg(&self.0)
            .args(["--timeout", "30s", "--format=json"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if explicit {
            c.arg("--kotlinc-tool").arg(self.0.join("kotlinc"));
        }
        let mut child = c.spawn().unwrap();
        child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["App.kt"],"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
#[test]
fn first_native_kotlin_check_and_save_reuse_one_task_without_wasm_replay() {
    let p = Project::new("fun f(x: ) = x\n");
    p.tool("2.4.10");
    let check = p.run(&["check", "all", "--timeout", "30s"]);
    assert_eq!(check["execution_budget"]["native_task_count"], 1);
    assert_eq!(check["execution_budget"]["started_native_task_count"], 1);
    let scan = &check["native_results"]["kotlin_lint"];
    assert_eq!(
        scan["files"][0]["native"]["status"], "diagnostics_observed",
        "{check}"
    );
    let id = scan["files"][0]["task_id"].as_str().unwrap().to_owned();
    assert!(
        check["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["language"] != "kotlin")
    );
    let next = p.run(&["next"]);
    assert_eq!(next["repair_brief"]["action_id"], "repair-source");
    let repeat = p.run(&["check", "all", "--timeout", "30s"]);
    assert_eq!(
        repeat["native_results"]["kotlin_lint"]["files"][0]["task_id"],
        id
    );
    let verify = p.run(&["task", "verify", &id, "--timeout", "30s"]);
    assert_eq!(verify["event_persisted"], true, "{verify}");
    assert_eq!(
        verify["native_scan"]["original_report"]["grammar_sha256"],
        Value::Null
    );
    let hook = p.hook_with_tool(true);
    assert_eq!(
        hook["local_feedback"]["kotlin_lint"]["files"][0]["task_id"], id,
        "{hook}"
    );
    assert!(
        hook["local_feedback"]["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["language"] != "kotlin")
    );
    fs::write(p.0.join("App.kt"), "object C { val value = 1 }\n").unwrap();
    let clean = p.run(&["check", "all", "--timeout", "30s"]);
    assert_eq!(
        clean["native_results"]["kotlin_lint"]["files"][0]["native"]["status"],
        "completed"
    );
    let next = p.run(&["next"]);
    assert_eq!(next["repair_brief"]["task_id"], id);
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
}
#[test]
fn selected_kotlin_failure_retains_blocker_and_does_not_parse_wasm() {
    let p = Project::new("fun f(x: ) = x\n");
    p.tool("2.3.0");
    let h = p.hook();
    let scan = &h["local_feedback"]["kotlin_lint"];
    assert_eq!(
        scan["files"][0]["native"]["reason"], "kotlin_version_unverified_or_unsupported",
        "{h}"
    );
    assert!(scan["files"][0]["task_id"].is_string());
    assert!(
        h["local_feedback"]["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["language"] != "kotlin")
    );
    assert_ne!(
        p.run(&["next"])["repair_brief"]["action_id"],
        "repair-source"
    );
}
#[test]
fn absent_kotlin_tool_uses_wasm_and_clean_native_source_creates_no_task() {
    let p = Project::new("fun f(x: ) = x\n");
    let h = p.hook();
    assert_eq!(
        h["local_feedback"]["kotlin_lint"]["tool_selection"]["source"], "not_found",
        "{h}"
    );
    assert_eq!(
        h["local_feedback"]["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let clean = Project::new("object C { val value = 1 }\n");
    clean.tool("2.4.10");
    let h = clean.hook();
    assert_eq!(
        h["local_feedback"]["kotlin_lint"]["files"][0]["native"]["status"], "completed",
        "{h}"
    );
    assert_eq!(
        fs::read_dir(clean.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn kotlin_first_native_human_feedback_contains_actionable_locations() {
    let p = Project::new("fun f(x: ) = x\n");
    p.tool("2.4.10");
    let out = p
        .command()
        .args(["check", "all"])
        .arg(&p.0)
        .args(["--timeout", "30s", "--format=human"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("kotlin.syntax") && text.contains(":1:10"),
        "{text}"
    );
}

#[test]
fn kotlin_script_does_not_become_single_file_native_compile() {
    let p = Project::new("object C {}\n");
    fs::rename(p.0.join("App.kt"), p.0.join("build.gradle.kts")).unwrap();
    p.tool("2.4.10");
    let report = p.run(&["check", "all", "--timeout", "30s"]);
    assert!(
        report["native_results"].get("kotlin_lint").is_none(),
        "{report}"
    );
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["id"] != "kotlin.lint")
    );
}
