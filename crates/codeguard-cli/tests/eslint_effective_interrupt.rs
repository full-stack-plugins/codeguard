#![cfg(unix)]
use serde_json::Value;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn invoke(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn interrupted_effective_query_preserves_cancellation_and_releases_task_lease() {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-eslint-query-interrupt-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let project = Project(root);
    let source = project.0.join("app.js");
    let config = project.0.join("eslint.config.cjs");
    let entry = project.0.join("eslint.cjs");
    let node = project.0.join("node");
    let started = project.0.join("query-started");
    let delayed = project.0.join("delayed-write");
    std::fs::write(&source, "debugger;").unwrap();
    std::fs::write(&config, "fixture").unwrap();
    std::fs::write(&entry, "fixture").unwrap();
    let native = serde_json::json!([{"filePath":source,"messages":[{"ruleId":"no-debugger","severity":2,"message":"fixture","line":1,"column":1}],"suppressedMessages":[],"errorCount":1,"warningCount":0,"fatalErrorCount":0,"fixableErrorCount":0,"fixableWarningCount":0}]);
    std::fs::write(&node, format!("#!/bin/sh\nfor arg in \"$@\"; do\nif [ \"$arg\" = --version ]; then printf 'v10.11.0\\n'; exit 0; fi\nif [ \"$arg\" = --print-config ]; then\nprintf started > '{}'\n(/bin/sleep 2; printf escaped > '{}') &\n/bin/sleep 10\nprintf '{{\"rules\":{{\"no-debugger\":[2]}}}}'\nexit 0\nfi\ndone\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; shift; done\nprintf '%s' '{}' > \"$report\"\nexit 1\n", started.display(), delayed.display(), native)).unwrap();
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o700)).unwrap();
    let root = project.0.to_str().unwrap();
    invoke(&["init", root, "--apply", "--format", "json"]);
    let context = [
        "--node-tool",
        node.to_str().unwrap(),
        "--eslint-entry",
        entry.to_str().unwrap(),
        "--eslint-version",
        "10.11.0",
        "--config",
        config.to_str().unwrap(),
        "--cwd",
        root,
    ];
    let mut lint = vec![
        "lint",
        "typescript",
        source.to_str().unwrap(),
        "--workspace",
        root,
        "--format",
        "json",
    ];
    lint.extend(context);
    let first = invoke(&lint);
    let task = first["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let mut args = vec![
        "task",
        "verify",
        task,
        root,
        "--format",
        "json",
        "--timeout",
        "20s",
    ];
    args.extend(context);
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !started.exists() && Instant::now() < deadline {
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(started.exists(), "effective query did not start");
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    std::thread::sleep(Duration::from_millis(2200));
    assert!(!delayed.exists(), "cancelled query descendant escaped");
    assert_eq!(
        feedback["native_scan"]["effective_rule"]["reason"], "request_cancelled",
        "{feedback}"
    );
    assert_eq!(feedback["observation"], "incomplete");
    assert_eq!(feedback["event_persisted"], false);
    assert_eq!(feedback["reason"], "request_cancelled");
    let second = invoke(&["task", "verify", task, root, "--format", "json"]);
    assert_eq!(
        second["event_persisted"], true,
        "cancelled lease was not released: {second}"
    );
}
