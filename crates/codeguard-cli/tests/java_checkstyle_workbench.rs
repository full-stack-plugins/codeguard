use serde_json::Value;
use std::{fs, process::Command};
static NEXT_PROJECT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Project(std::path::PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn project() -> Project {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT_PROJECT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-checkstyle-workbench-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Foo.java"), "public class Foo {}\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    Project(root)
}
#[test]
fn workspace_without_native_observation_does_not_create_source_tasks() {
    let p = project();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            p.0.join("Foo.java").to_str().unwrap(),
            "--checker=checkstyle",
            "--workspace",
            p.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["workbench"]["status"], "synced_partial");
    assert_eq!(report["workbench"]["new_blockers"], 1);
    assert_eq!(report["workbench"]["new_findings"], 0);
    assert_eq!(
        report["workbench"]["next"]["repair_brief"]["kind"],
        "blocker"
    );
    assert_eq!(
        report["workbench"]["next"]["repair_brief"]["checker_id"],
        "java.checkstyle.preparation"
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
}
#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn native_checkstyle_syncs_stable_tasks_and_next_without_closing() {
    let p = project();
    let config = p.0.join("checks.xml");
    fs::write(&config,"<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"><property name=\"id\" value=\"publicType\"/></module></module></module>".replace("\\\"","\"")).unwrap();
    for iteration in 0..2 {
        if iteration == 1 {
            fs::write(p.0.join("Foo.java"), "\npublic class Foo {}\n").unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                p.0.join("Foo.java").to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &std::env::var("CODEGUARD_JAVA_BIN").unwrap(),
                "--checkstyle-jar",
                &std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let r: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(r["workbench"]["status"], "synced_partial", "{r}");
        assert_eq!(
            r["workbench"]["new_findings"],
            if iteration == 0 { 1 } else { 0 }
        );
        assert_eq!(
            r["workbench"]["next"]["repair_brief"]["checkstyle_guidance"]["rule_summary"],
            "类型缺少 Javadoc"
        );
        assert_eq!(
            r["workbench"]["next"]["repair_brief"]["disposition"],
            "actionable"
        );
        assert_eq!(
            r["workbench"]["next"]["repair_brief"]["checker_id"],
            "java.checkstyle"
        );
    }
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    let task = fs::read_dir(p.0.join(".codeguard/tasks"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let contents = fs::read_to_string(&task).unwrap();
    for field in [
        "问题证据",
        "规则依据",
        "允许范围",
        "修复步骤",
        "复检",
        "历史尝试",
        "关闭条件",
    ] {
        assert!(contents.contains(field));
    }
    fs::write(&task, "# 已人工勾选\n- [x] 完成\n").unwrap();
    fs::write(&config, fs::read_to_string(&config).unwrap() + "\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(next["repair_brief"]["disposition"], "verification_required");
    assert_eq!(
        next["repair_brief"]["checkstyle_guidance"]["configuration_current"],
        false
    );
    let file = fs::read_dir(p.0.join(".codeguard/reports"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut report: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
    report["run_id"] = serde_json::json!("checkstyle-stale-1");
    fs::write(
        p.0.join(".codeguard/reports/checkstyle-stale-1.json"),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let sync: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sync["failed_reports"], 1);
    assert_eq!(sync["new_findings"], 0);
    assert_eq!(
        fs::read_to_string(task).unwrap(),
        "# 已人工勾选\n- [x] 完成\n"
    );
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn task_verify_replays_native_rules_and_retains_open_task() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("checks.xml");
    let original = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"><property name=\"id\" value=\"publicType\"/></module></module></module>";
    fs::write(&config, original).unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            source.to_str().unwrap(),
            "--checker=checkstyle",
            "--workspace",
            p.0.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--java-tool",
            &java,
            "--checkstyle-jar",
            &jar,
            "--format=json",
        ])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = report["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    for (expected, mode) in [
        ("still_present", 0),
        ("candidate_absent_unverified_policy", 1),
        ("rule_coverage_requires_review", 2),
        ("incomplete", 3),
    ] {
        if mode == 1 {
            fs::write(&source, "/** 实际用途文档。 */\npublic class Foo {}\n").unwrap();
        }
        if mode == 2 {
            fs::write(
                &config,
                original.replace("MissingJavadocType", "JavadocType"),
            )
            .unwrap();
        }
        let selected_jar = if mode == 3 {
            "/nonexistent/codeguard-checkstyle.jar"
        } else {
            jar.as_str()
        };
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                id,
                p.0.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                selected_jar,
                "--config",
                config.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["observation"], expected, "{result}");
        assert_eq!(result["event_persisted"], true, "{result}");
        let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", p.0.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap();
        let view: Value = serde_json::from_slice(&next.stdout).unwrap();
        assert_ne!(view["command_status"], "incomplete", "{view}");
        if mode == 0 {
            fs::write(&config, format!("{original}\n<!-- unrelated edit -->\n")).unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["task", "show", id, p.0.to_str().unwrap(), "--format=json"])
                .output()
                .unwrap();
            let changed: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                changed["task"]["disposition"], "verification_required",
                "{changed}"
            );
            assert_eq!(
                changed["task"]["verification_invalidated_reason"],
                "configuration_input_changed_or_unavailable"
            );
            assert!(changed["task"]["verification_observation"].is_null());
            fs::write(&config, original).unwrap();
            fs::write(&source, "\npublic class Foo {}\n").unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["task", "show", id, p.0.to_str().unwrap(), "--format=json"])
                .output()
                .unwrap();
            let changed: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                changed["task"]["verification_invalidated_reason"],
                "source_input_changed_or_unavailable",
                "{changed}"
            );
            assert!(changed["task"]["verification_observation"].is_null());
            fs::write(&source, "public class Foo {}\n").unwrap();
        }
        if mode == 1 {
            assert_eq!(view["repair_brief"]["verification_observation"], expected);
            assert_eq!(view["repair_brief"]["disposition"], "verification_required");
        }
        let fact: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
    }
    // 改变 JAR 字节但仍可被原生 Java 运行，重复复检也不得滚动认可新工具身份。
    fs::write(&source, "public class Foo {}\n").unwrap();
    fs::write(&config, original).unwrap();
    let changed = p.0.join("changed-checkstyle.jar");
    let mut changed_bytes = fs::read(&jar).unwrap();
    changed_bytes.extend_from_slice(b"codeguard-test-trailing-data");
    fs::write(&changed, changed_bytes).unwrap();
    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                id,
                p.0.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                changed.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["observation"], "incomplete", "{result}");
        assert_eq!(result["native_scan"]["tool_identity_matches"], false);
        assert_eq!(result["event_persisted"], true);
    }
}

#[test]
fn checkstyle_verify_requires_absolute_and_nonduplicated_native_paths() {
    let id = "CG-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    for extra in [
        vec!["--java-tool", "relative"],
        vec!["--config", "relative"],
        vec![
            "--checkstyle-jar",
            "/tmp/one",
            "--checkstyle-jar",
            "/tmp/two",
        ],
    ] {
        let mut args = vec!["task", "verify", id, "."];
        args.extend(extra);
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}

#[test]
fn changed_preparation_diagnosis_reuses_task_and_preserves_markdown() {
    let p = project();
    fs::write(p.0.join("Bar.java"), "public class Bar {}\n").unwrap();
    let mut id = None;
    for (source, extra, expected) in [
        ("Foo.java", vec![], "prerequisites_missing"),
        (
            "Bar.java",
            vec![
                "--java-tool",
                "/nonexistent/java",
                "--checkstyle-jar",
                "/nonexistent/checkstyle.jar",
                "--config",
                "/nonexistent/config.xml",
            ],
            "configuration_unavailable",
        ),
    ] {
        let file = p.0.join(source);
        let mut args = vec![
            "lint",
            "java",
            file.to_str().unwrap(),
            "--checker=checkstyle",
            "--workspace",
            p.0.to_str().unwrap(),
            "--format=json",
        ];
        args.extend(extra);
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["workbench"]["status"], "synced_partial", "{report}");
        let brief = &report["workbench"]["next"]["repair_brief"];
        assert_eq!(brief["preparation_guidance"]["diagnostic_reason"], expected);
        assert_eq!(brief["preparation_guidance"]["affected_paths"][0], source);
        let current = brief["task_id"].as_str().unwrap().to_owned();
        if let Some(old) = id.as_ref() {
            assert_eq!(old, &current);
        } else {
            id = Some(current.clone());
            fs::write(
                p.0.join(format!(".codeguard/tasks/{current}.md")),
                "# 人工备注\n- [x] 已处理\n",
            )
            .unwrap();
        }
    }
    let id = id.unwrap();
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    assert_eq!(
        fs::read_to_string(p.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap(),
        "# 人工备注\n- [x] 已处理\n"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn source_outside_workspace_never_creates_preparation_task() {
    let p = project();
    let other = project();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            other.0.join("Foo.java").to_str().unwrap(),
            "--checker=checkstyle",
            "--workspace",
            p.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(r["workbench"]["status"], "source_outside_workspace");
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        0
    );
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn same_scope_native_recovery_changes_preparation_next_without_closing() {
    let p = project();
    let source = p.0.join("Foo.java");
    fs::write(&source, "/** 实际用途说明。 */\npublic class Foo {}\n").unwrap();
    let config = p.0.join("checks.xml");
    fs::write(&config,"<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"/></module></module>").unwrap();
    let base = [
        "lint",
        "java",
        source.to_str().unwrap(),
        "--checker=checkstyle",
        "--workspace",
        p.0.to_str().unwrap(),
        "--format=json",
    ];
    let initial: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(base)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let id = initial["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let result: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(base)
            .args([
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &std::env::var("CODEGUARD_JAVA_BIN").unwrap(),
                "--checkstyle-jar",
                &std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap(),
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(result["local_status"], "observed");
    assert_eq!(result["findings"].as_array().unwrap().len(), 0);
    let next: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", p.0.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let brief = &next["repair_brief"];
    assert_eq!(brief["task_id"], id);
    assert_eq!(brief["disposition"], "verification_required");
    assert_eq!(
        brief["preparation_guidance"]["later_native_observation"]["status"],
        "same_scope_native_observed_unverified"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(&config, fs::read_to_string(&config).unwrap() + "\n").unwrap();
    let stale: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", p.0.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert!(stale["repair_brief"]["preparation_guidance"]["later_native_observation"].is_null());
}

#[test]
fn preparation_verify_records_blocked_observation_and_keeps_task_open() {
    let p = project();
    let initial: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                p.0.join("Foo.java").to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let id = initial["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id, p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let verified: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(verified["observation"], "still_blocked", "{verified}");
    assert_eq!(verified["event_persisted"], true);
    let view: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", p.0.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(
        view["repair_brief"]["verification_observation"],
        "still_blocked"
    );
    assert_eq!(
        view["repair_brief"]["verification_reason"],
        "prerequisites_missing"
    );
    let status = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["status", p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let status: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(
        status["tasks"][0]["verification_reason"], "prerequisites_missing",
        "{status}"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(p.0.join("Foo.java"), "public class Foo { int changed; }\n").unwrap();
    let view = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let view: Value = serde_json::from_slice(&view.stdout).unwrap();
    assert_eq!(view["disposition"], "verification_required", "{view}");
    assert_eq!(
        view["repair_brief"]["preparation_guidance"]["source_current"],
        false
    );
}
#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn preparation_verify_native_recovery_creates_source_task_but_keeps_blocker_open() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("checks.xml");
    let jar = p.0.join("checkstyle.jar");
    fs::copy(std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap(), &jar).unwrap();
    fs::write(&config,"<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"/></module></module>").unwrap();
    let initial: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let id = initial["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        )
    };
    let root = p.0.to_str().unwrap();
    let action = initial["workbench"]["next"]["repair_brief"]["action_id"]
        .as_str()
        .unwrap();
    let (exit, claimed) = run(&["task", "claim", id, root, "--owner", "agent-a"]);
    assert_eq!(exit, 0);
    let token = claimed["lease_token"].as_str().unwrap();
    let (exit, started) = run(&[
        "task",
        "attempt",
        "start",
        id,
        root,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--action-id",
        action,
    ]);
    assert_eq!(exit, 0, "{started}");
    let attempt = started["attempt_id"].as_str().unwrap();
    let (exit, finished) = run(&[
        "task",
        "attempt",
        "finish",
        id,
        root,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--attempt-id",
        attempt,
        "--outcome",
        "ready-to-verify",
        "--note-code",
        "tool_restored",
    ]);
    assert_eq!(exit, 0, "{finished}");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--config",
            config.to_str().unwrap(),
            "--java-tool",
            &std::env::var("CODEGUARD_JAVA_BIN").unwrap(),
            "--checkstyle-jar",
            jar.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        result["observation"], "environment_restored_unverified_policy",
        "{result}"
    );
    assert_eq!(result["event_persisted"], true, "{result}");
    let event: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(
            ".codeguard/findings/{id}/events/verify-{}.json",
            result["native_scan"]["run_id"].as_str().unwrap()
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(event["attempt_id"], attempt);
    let (_, prep_view) = run(&["task", "show", id, root]);
    assert_eq!(
        prep_view["task"]["history"]["awaiting_verification"], false,
        "{prep_view}"
    );
    assert_eq!(prep_view["task"]["history"]["no_progress_count"], 0);

    assert_eq!(
        result["native_scan"]["scan"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let view: Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", p.0.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(
        view["repair_brief"]["checker_id"], "java.checkstyle",
        "{view}"
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        2
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(&jar, b"changed native tool").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "show", id, p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let view: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(view["task"]["verification_observation"].is_null(), "{view}");
    assert!(
        view["task"]["step"].as_str().unwrap().contains("工具"),
        "{view}"
    );
    assert_eq!(view["task"]["disposition"], "verification_required");
    assert_eq!(
        view["task"]["verification_invalidated_reason"],
        "tool_inputs_changed_or_unavailable"
    );
    fs::remove_file(&jar).unwrap();
    let (_, unavailable) = run(&["task", "show", id, root]);
    assert_eq!(
        unavailable["task"]["verification_invalidated_reason"],
        "tool_inputs_changed_or_unavailable"
    );
    assert!(unavailable["task"]["verification_observation"].is_null());
    let (_, status) = run(&["status", root]);
    let task = status["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["task_id"] == id)
        .unwrap();
    assert_eq!(
        task["verification_invalidated_reason"],
        "tool_inputs_changed_or_unavailable"
    );
}

#[test]
fn preparation_verify_wrong_lease_cannot_write_recovery_observation() {
    let p = project();
    let initial = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            p.0.join("Foo.java").to_str().unwrap(),
            "--checker=checkstyle",
            "--workspace",
            p.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let initial: Value = serde_json::from_slice(&initial.stdout).unwrap();
    let id = initial["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let claimed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            id,
            p.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(claimed.status.code(), Some(0));
    let before = fs::read_dir(p.0.join(".codeguard/reports"))
        .unwrap()
        .count();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            &"0".repeat(64),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let rejected: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rejected["reason"], "lease_token_mismatch", "{rejected}");
    assert_eq!(rejected["event_persisted"], false);
    assert!(rejected["native_scan"].is_null());
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/reports"))
            .unwrap()
            .count(),
        before
    );
}

#[test]
fn preparation_failed_rechecks_count_attempts_and_stop_repetition() {
    let p = project();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        )
    };
    let root = p.0.to_str().unwrap();
    let source = p.0.join("Foo.java");
    let (_, initial) = run(&[
        "lint",
        "java",
        source.to_str().unwrap(),
        "--checker=checkstyle",
        "--workspace",
        root,
    ]);
    let id = initial["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let action = initial["workbench"]["next"]["repair_brief"]["action_id"]
        .as_str()
        .unwrap();
    let (exit, claim) = run(&["task", "claim", id, root, "--owner", "agent-a"]);
    assert_eq!(exit, 0);
    let token = claim["lease_token"].as_str().unwrap();
    for iteration in 1..=2 {
        let (exit, started) = run(&[
            "task",
            "attempt",
            "start",
            id,
            root,
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--action-id",
            action,
        ]);
        assert_eq!(exit, 0, "{started}");
        let attempt = started["attempt_id"].as_str().unwrap();
        let (exit, finished) = run(&[
            "task",
            "attempt",
            "finish",
            id,
            root,
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--attempt-id",
            attempt,
            "--outcome",
            "ready-to-verify",
            "--note-code",
            "tool_restored",
        ]);
        assert_eq!(exit, 0, "{finished}");
        let (exit, premature) = run(&[
            "task",
            "attempt",
            "start",
            id,
            root,
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--action-id",
            action,
        ]);
        assert_eq!(exit, 3);
        assert_eq!(premature["reason"], "verification_required_before_retry");
        let (_, verified) = run(&[
            "task",
            "verify",
            id,
            root,
            "--owner",
            "agent-a",
            "--lease-token",
            token,
        ]);
        assert_eq!(verified["observation"], "still_blocked", "{verified}");
        assert_eq!(verified["event_persisted"], true);
        let event: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(
                ".codeguard/findings/{id}/events/verify-{}.json",
                verified["native_scan"]["run_id"].as_str().unwrap()
            )))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(event["attempt_id"], attempt);
        let (_, next) = run(&["next", root]);
        assert_eq!(
            next["repair_brief"]["history"]["no_progress_count"], iteration,
            "{next}"
        );
    }
    let (exit, denied) = run(&[
        "task",
        "attempt",
        "start",
        id,
        root,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--action-id",
        action,
    ]);
    assert_eq!(exit, 3);
    assert_eq!(denied["reason"], "no_progress_budget_exhausted");
    let (_, next) = run(&["next", root]);
    assert_eq!(next["disposition"], "needs_decision");
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn native_field_javadoc_respects_scope_and_rechecks_stable_task() {
    let p = project();
    let root = p.0.to_str().unwrap();
    let source = p.0.join("Foo.java");
    let config = p.0.join("fields.xml");
    let original = "/** 类型用途。 */\npublic class Foo {\n public int value;\n private int hidden;\n public int log;\n public static final long serialVersionUID = 1L;\n public void method() { int local = 1; }\n}\n";
    fs::write(&source, original).unwrap();
    fs::write(&config, "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocVariable\"><property name=\"id\" value=\"fieldDocs\"/><property name=\"scope\" value=\"public\"/><property name=\"ignoreNamePattern\" value=\"log|logger\"/></module></module></module>").unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .args([
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--config",
                config.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let scan = run(&[
        "lint",
        "java",
        source.to_str().unwrap(),
        "--checker=checkstyle",
        "--workspace",
        root,
    ]);
    assert_eq!(scan["findings"].as_array().unwrap().len(), 1, "{scan}");
    assert_eq!(scan["findings"][0]["rule_id"], "fieldDocs");
    assert_eq!(scan["findings"][0]["line"], 3);
    assert_eq!(
        scan["findings"][0]["checker_class"],
        "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocVariableCheck"
    );
    let id = scan["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    assert!(
        scan["workbench"]["next"]["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("字段")
    );
    assert_eq!(
        run(&["task", "verify", id, root])["observation"],
        "still_present"
    );
    fs::write(
        &source,
        original.replace(" public int value;", " /** 当前值。 */\n public int value;"),
    )
    .unwrap();
    let verified = run(&["task", "verify", id, root]);
    assert_eq!(
        verified["observation"], "candidate_absent_unverified_policy",
        "{verified}"
    );
    assert_eq!(verified["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn native_field_ranges_and_enum_tokens_preserve_configured_targets() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("fields.xml");
    fs::write(&source,"public class Foo {\n public int exposed;\n protected int protectedField;\n int packageField;\n private int privateField;\n public enum State {\n  FIRST,\n  /** Second value. */\n  SECOND;\n }\n}\n").unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    for (properties, expected) in [
        ("<property name=\"scope\" value=\"public\"/>", vec![2, 7]),
        (
            "<property name=\"scope\" value=\"protected\"/>",
            vec![2, 3, 7],
        ),
        (
            "<property name=\"scope\" value=\"package\"/>",
            vec![2, 3, 4, 7],
        ),
        (
            "<property name=\"scope\" value=\"private\"/>",
            vec![2, 3, 4, 5, 7],
        ),
        (
            "<property name=\"scope\" value=\"private\"/><property name=\"excludeScope\" value=\"protected\"/>",
            vec![4, 5],
        ),
        (
            "<property name=\"tokens\" value=\"VARIABLE_DEF\"/>",
            vec![2, 3, 4, 5],
        ),
        (
            "<property name=\"tokens\" value=\"ENUM_CONSTANT_DEF\"/>",
            vec![2, 3, 4, 5, 7],
        ),
    ] {
        fs::write(&config,format!("<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocVariable\"><property name=\"id\" value=\"fieldDocs\"/>{properties}</module></module></module>")).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--config",
                config.to_str().unwrap(),
                "--workspace",
                p.0.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "observed", "{properties}: {scan}");
        let lines: Vec<_> = scan["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                assert_eq!(f["rule_id"], "fieldDocs");
                f["line"].as_u64().unwrap()
            })
            .collect();
        assert_eq!(lines, expected, "{properties}: {scan}");
        assert_eq!(scan["workbench"]["status"], "synced_partial");
        assert_eq!(scan["coverage_proven"], false);
    }
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn official_module_aliases_share_native_finding_and_task_identity() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("fields.xml");
    fs::write(&source, "public class Foo { public int value; }\n").unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    let mut task_id = None;
    for (index, (checker, tree, rule)) in [
        ("Checker", "TreeWalker", "JavadocVariable"),
        (
            "com.puppycrawl.tools.checkstyle.Checker",
            "com.puppycrawl.tools.checkstyle.TreeWalker",
            "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocVariableCheck",
        ),
        ("Checker", "TreeWalker", "JavadocVariableCheck"),
    ]
    .into_iter()
    .enumerate()
    {
        fs::write(&config, format!("<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"{checker}\"><module name=\"{tree}\"><module name=\"{rule}\"><property name=\"scope\" value=\"public\"/><property name=\"tokens\" value=\"VARIABLE_DEF\"/></module></module></module>")).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--config",
                config.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "observed", "{scan}");
        assert_eq!(scan["findings"].as_array().unwrap().len(), 1);
        assert_eq!(
            scan["findings"][0]["rule_id"],
            "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocVariableCheck"
        );
        assert_eq!(
            scan["workbench"]["new_findings"],
            if index == 0 { 1 } else { 0 }
        );
        assert_eq!(scan["workbench"]["new_blockers"], 0);
        let id = scan["workbench"]["next"]["repair_brief"]["task_id"]
            .as_str()
            .unwrap()
            .to_owned();
        if let Some(prior) = &task_id {
            assert_eq!(prior, &id);
        } else {
            task_id = Some(id);
        }
    }
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            task_id.as_deref().unwrap(),
            p.0.to_str().unwrap(),
            "--java-tool",
            &java,
            "--checkstyle-jar",
            &jar,
            "--config",
            config.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let verified: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(verified["observation"], "still_present", "{verified}");
    assert_eq!(verified["event_persisted"], true);
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn original_missing_method_properties_select_native_tasks() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("methods.xml");
    fs::write(&source,"public class Foo {\n private int value;\n @SkipDocs public void annotated() {}\n public void needsDoc() {}\n public int getValue() {\n  return value;\n }\n public void setValue(int value) {\n  this.value = value;\n }\n public void ignoredHelper() {}\n private void hidden() {}\n}\n").unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    for (properties, expected) in [
        (
            "<property name=\"scope\" value=\"public\"/><property name=\"allowedAnnotations\" value=\"SkipDocs\"/><property name=\"allowMissingPropertyJavadoc\" value=\"true\"/><property name=\"ignoreMethodNamesRegex\" value=\"ignored.*\"/>",
            vec![4],
        ),
        (
            "<property name=\"scope\" value=\"public\"/>",
            vec![3, 4, 5, 8, 11],
        ),
        (
            "<property name=\"scope\" value=\"public\"/><property name=\"allowMissingPropertyJavadoc\" value=\"false\"/>",
            vec![3, 4, 5, 8, 11],
        ),
        (
            "<property name=\"scope\" value=\"public\"/><property name=\"minLineCount\" value=\"2\"/>",
            vec![],
        ),
    ] {
        fs::write(&config, format!("<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"com.puppycrawl.tools.checkstyle.checks.javadoc.MissingJavadocMethodCheck\"><property name=\"id\" value=\"methodDocs\"/>{properties}</module></module></module>")).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "observed", "{scan}");
        let lines: Vec<_> = scan["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                assert_eq!(f["rule_id"], "methodDocs");
                f["line"].as_u64().unwrap()
            })
            .collect();
        assert_eq!(lines, expected, "{properties}: {scan}");
        assert_eq!(scan["workbench"]["new_blockers"], 0);
        assert_eq!(scan["workbench"]["status"], "synced_partial");
        assert_eq!(scan["delivery_decision"], "not_evaluated");
    }
    // 原配置使本轮零诊断，但其它历史方法任务不得因此关闭。
    for entry in fs::read_dir(p.0.join(".codeguard/findings")).unwrap() {
        let fact: Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path().join("finding.json")).unwrap())
                .unwrap();
        assert_eq!(fact["state"], "open");
    }
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn native_method_tokens_select_constructors_and_annotation_members() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("tokens.xml");
    let original = "public class Foo {\n public Foo() {}\n public void method() {}\n public record Data(int value) {\n  public Data {}\n }\n public @interface Config {\n  String value();\n }\n}\n";
    fs::write(&source, original).unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    let mut compact_task = None;
    for (tokens, expected) in [
        (
            "METHOD_DEF, CTOR_DEF, COMPACT_CTOR_DEF, ANNOTATION_FIELD_DEF",
            vec![2, 3, 5, 8],
        ),
        ("METHOD_DEF", vec![3]),
        ("CTOR_DEF", vec![2]),
        ("ANNOTATION_FIELD_DEF", vec![8]),
        ("COMPACT_CTOR_DEF", vec![5]),
    ] {
        fs::write(&config, format!("<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocMethod\"><property name=\"id\" value=\"methodDocs\"/><property name=\"scope\" value=\"public\"/><property name=\"tokens\" value=\"{tokens}\"/></module></module></module>")).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "observed", "{scan}");
        let lines: Vec<_> = scan["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                assert_eq!(f["rule_id"], "methodDocs");
                f["line"].as_u64().unwrap()
            })
            .collect();
        assert_eq!(lines, expected, "{tokens}: {scan}");
        assert_eq!(scan["workbench"]["new_blockers"], 0);
        assert_eq!(
            scan["workbench"]["new_findings"],
            if tokens.contains(",") { 4 } else { 0 }
        );
        assert_eq!(scan["coverage_proven"], false);
        // 从原生报告投影选择紧凑构造器，不能依赖 next 的任务排序。
        for entry in fs::read_dir(p.0.join(".codeguard/reports")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().is_some_and(|ext| ext == "json") {
                let report: Value =
                    serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
                if report["report_type"] == "java_checkstyle_workbench_observation" {
                    for finding in report["findings"].as_array().unwrap() {
                        if finding["line"] == 5 {
                            compact_task = Some(finding["finding_id"].as_str().unwrap().to_owned());
                        }
                    }
                }
            }
        }
    }
    let task_id = compact_task.expect("compact constructor must have a stable task");
    for (fixed, expected) in [
        (false, "still_present"),
        (true, "rule_coverage_requires_review"),
    ] {
        if fixed {
            fs::write(
                &source,
                original.replace(
                    "  public Data {}",
                    "  /** Creates a validated data value. */\n  public Data {}",
                ),
            )
            .unwrap();
        }
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                &task_id,
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        let result: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result["observation"], expected, "{result}");
        assert_eq!(result["event_persisted"], true);
    }
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        4
    );
    for entry in fs::read_dir(p.0.join(".codeguard/findings")).unwrap() {
        let fact: Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path().join("finding.json")).unwrap())
                .unwrap();
        assert_eq!(fact["state"], "open");
    }
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn changed_native_field_exclusion_cannot_become_a_repair_candidate() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("fields.xml");
    let original_source = "public class Foo {\n public int value;\n}\n";
    let original_config = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocVariable\"><property name=\"scope\" value=\"public\"/></module></module></module>";
    fs::write(&source, original_source).unwrap();
    fs::write(&config, original_config).unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            source.to_str().unwrap(),
            "--checker=checkstyle",
            "--workspace",
            p.0.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--java-tool",
            &java,
            "--checkstyle-jar",
            &jar,
            "--format=json",
        ])
        .output()
        .unwrap();
    let scan: Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = scan["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::write(
        &config,
        original_config.replace(
            "<property name=\"scope\"",
            "<property name=\"ignoreNamePattern\" value=\"value\"/><property name=\"scope\"",
        ),
    )
    .unwrap();
    for expected in [
        "rule_coverage_requires_review",
        "rule_coverage_requires_review",
        "candidate_absent_unverified_policy",
    ] {
        if expected == "candidate_absent_unverified_policy" {
            fs::write(&config, original_config).unwrap();
            fs::write(
                &source,
                original_source.replace(
                    " public int value;",
                    " /** Stored value. */\n public int value;",
                ),
            )
            .unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                &id,
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["observation"], expected, "{result}");
        assert_eq!(result["event_persisted"], true);
    }
    let fact: Value = serde_json::from_slice(
        &fs::read(
            p.0.join(".codeguard/findings")
                .join(id)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn original_method_tag_options_control_native_diagnostics_and_repair() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("tags.xml");
    let original_source = "public class Foo {\n /** Computes. */\n public int compute(int value) {\n  return value;\n }\n /** Helper. */\n private int helper(int value) {\n  return value;\n }\n /** Throws failure. */\n public void fail() throws IllegalStateException {\n  throw new IllegalStateException();\n }\n /** Skipped API. */\n @SkipDocs\n public int skipped(int value) {\n  return value;\n }\n}\n";
    fs::write(&source, original_source).unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    let prefix = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocMethod\"><property name=\"id\" value=\"methodTags\"/><property name=\"accessModifiers\" value=\"public\"/>";
    let suffix = "</module></module></module>";
    let mut task = None;
    for (properties, expected) in [
        ("", 4),
        (
            "<property name=\"allowMissingParamTags\" value=\"true\"/>",
            2,
        ),
        (
            "<property name=\"allowMissingReturnTag\" value=\"true\"/>",
            2,
        ),
        (
            "<property name=\"allowMissingParamTags\" value=\"true\"/><property name=\"allowMissingReturnTag\" value=\"true\"/>",
            0,
        ),
        (
            "<property name=\"allowedAnnotations\" value=\"SkipDocs\"/>",
            2,
        ),
        ("<property name=\"validateThrows\" value=\"true\"/>", 5),
    ] {
        fs::write(&config, format!("{prefix}{properties}{suffix}")).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "observed", "{scan}");
        let findings = scan["findings"].as_array().unwrap();
        assert_eq!(findings.len(), expected, "{properties}: {scan}");
        assert!(findings.iter().all(|f| f["rule_id"] == "methodTags"));
        assert_eq!(scan["workbench"]["new_blockers"], 0);
        assert_eq!(scan["coverage_proven"], false);
        if task.is_none() {
            task = Some(
                scan["workbench"]["next"]["repair_brief"]["task_id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
    }
    fs::write(&config, format!("{prefix}{suffix}")).unwrap();
    let task = task.unwrap();
    for (fixed, expected) in [
        (false, "still_present"),
        (true, "candidate_absent_unverified_policy"),
    ] {
        if fixed {
            fs::write(
                &source,
                original_source
                    .replace(
                        "/** Computes. */",
                        "/** Computes.\n  * @param value input\n  * @return input\n  */",
                    )
                    .replace(
                        "/** Skipped API. */",
                        "/** Skipped API.\n  * @param value input\n  * @return input\n  */",
                    ),
            )
            .unwrap();
        }
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                &task,
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        let verified: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(verified["observation"], expected, "{verified}");
        assert_eq!(verified["event_persisted"], true);
    }
    for entry in fs::read_dir(p.0.join(".codeguard/findings")).unwrap() {
        let fact: Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path().join("finding.json")).unwrap())
                .unwrap();
        assert_eq!(fact["state"], "open");
    }
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn original_type_options_select_native_documentation_tasks() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("types.xml");
    let missing_source = "public class Foo {\n public class PublicNested {}\n private class PrivateNested {}\n @SkipDocs\n public class Marked {}\n public record Data(int value) {}\n public interface Contract {}\n}\n";
    let tags_source = "public class Foo {\n /** Data. */\n @SkipDocs\n public record Data(int value) {}\n /** Tagged.\n  * @custom context\n  */\n public class Tagged {}\n}\n";
    let prefix = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\">";
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    for (module, rule, properties, text, expected) in [
        (
            "MissingJavadocType",
            "missingTypes",
            "<property name=\"scope\" value=\"public\"/>",
            missing_source,
            5,
        ),
        (
            "MissingJavadocType",
            "missingTypes",
            "<property name=\"scope\" value=\"public\"/><property name=\"skipAnnotations\" value=\"SkipDocs\"/>",
            missing_source,
            4,
        ),
        (
            "MissingJavadocType",
            "missingTypes",
            "<property name=\"scope\" value=\"private\"/><property name=\"excludeScope\" value=\"public\"/>",
            missing_source,
            1,
        ),
        (
            "MissingJavadocType",
            "missingTypes",
            "<property name=\"tokens\" value=\"RECORD_DEF\"/>",
            missing_source,
            1,
        ),
        (
            "JavadocType",
            "typeTags",
            "<property name=\"scope\" value=\"public\"/>",
            tags_source,
            2,
        ),
        (
            "JavadocType",
            "typeTags",
            "<property name=\"scope\" value=\"public\"/><property name=\"allowMissingParamTags\" value=\"true\"/>",
            tags_source,
            1,
        ),
        (
            "JavadocType",
            "typeTags",
            "<property name=\"scope\" value=\"public\"/><property name=\"allowUnknownTags\" value=\"true\"/>",
            tags_source,
            1,
        ),
        (
            "JavadocType",
            "typeTags",
            "<property name=\"scope\" value=\"public\"/><property name=\"allowMissingParamTags\" value=\"true\"/><property name=\"allowUnknownTags\" value=\"true\"/>",
            tags_source,
            0,
        ),
        (
            "JavadocType",
            "typeTags",
            "<property name=\"scope\" value=\"public\"/><property name=\"allowedAnnotations\" value=\"SkipDocs\"/>",
            tags_source,
            1,
        ),
        (
            "JavadocType",
            "typeTags",
            "<property name=\"tokens\" value=\"RECORD_DEF\"/>",
            tags_source,
            1,
        ),
    ] {
        fs::write(&source, text).unwrap();
        fs::write(&config, format!("{prefix}<module name=\"{module}\"><property name=\"id\" value=\"{rule}\"/>{properties}</module></module></module>")).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "observed", "{scan}");
        let findings = scan["findings"].as_array().unwrap();
        assert_eq!(findings.len(), expected, "{module} {properties}: {scan}");
        assert!(findings.iter().all(|f| f["rule_id"] == rule));
        assert_eq!(scan["workbench"]["new_blockers"], 0);
        assert_eq!(scan["coverage_proven"], false);
    }
    let mut target = None;
    for entry in fs::read_dir(p.0.join(".codeguard/findings")).unwrap() {
        let entry = entry.unwrap();
        let fact: Value =
            serde_json::from_slice(&fs::read(entry.path().join("finding.json")).unwrap()).unwrap();
        if fact["native_rule_id"] == "typeTags" {
            target = Some(entry.file_name().to_str().unwrap().to_owned());
        }
    }
    let target = target.unwrap();
    // 恢复标签检查的首次配置，避免 token 切换冒充文档修复。
    fs::write(&config, format!("{prefix}<module name=\"JavadocType\"><property name=\"id\" value=\"typeTags\"/><property name=\"scope\" value=\"public\"/></module></module></module>")).unwrap();
    // 一起修正 record 参数和未知标签，不能保留同规则其它诊断后称已完成。
    fs::write(
        &source,
        tags_source
            .replace("/** Data. */", "/** Data.\n  * @param value input\n  */")
            .replace("  * @custom context\n", ""),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &target,
            p.0.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--java-tool",
            &java,
            "--checkstyle-jar",
            &jar,
            "--format=json",
        ])
        .output()
        .unwrap();
    let verified: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        verified["observation"], "candidate_absent_unverified_policy",
        "{verified}"
    );
    assert_eq!(verified["event_persisted"], true);
    for entry in fs::read_dir(p.0.join(".codeguard/findings")).unwrap() {
        let fact: Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path().join("finding.json")).unwrap())
                .unwrap();
        assert_eq!(fact["state"], "open");
    }
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 jar"]
fn original_type_author_version_formats_preserve_native_failures_and_repair() {
    let p = project();
    let source = p.0.join("Foo.java");
    let config = p.0.join("formats.xml");
    let configuration = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocType\"><property name=\"id\" value=\"typeFormats\"/><property name=\"authorFormat\" value=\"team-[a-z]+\"/><property name=\"versionFormat\" value=\"v[0-9]+\"/></module></module></module>";
    fs::write(&config, configuration).unwrap();
    let java = std::env::var("CODEGUARD_JAVA_BIN").unwrap();
    let jar = std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap();
    let mut task = None;
    for (text, expected) in [
        ("/** Model. */\npublic class Foo {}\n", 2),
        (
            "/** Model.\n * @author wrong\n * @version wrong\n */\npublic class Foo {}\n",
            2,
        ),
        (
            "/** Model.\n * @author team-api\n */\npublic class Foo {}\n",
            1,
        ),
        (
            "/** Model.\n * @author team-api\n * @version v1\n */\npublic class Foo {}\n",
            0,
        ),
    ] {
        fs::write(&source, text).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "observed", "{scan}");
        let findings = scan["findings"].as_array().unwrap();
        assert_eq!(findings.len(), expected, "{scan}");
        assert!(findings.iter().all(|f| f["rule_id"] == "typeFormats"));
        assert_eq!(scan["workbench"]["new_blockers"], 0);
        if task.is_none() {
            task = Some(
                scan["workbench"]["next"]["repair_brief"]["task_id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
            let step =
                scan["workbench"]["next"]["repair_brief"]["checkstyle_guidance"]["repair_steps"][0]
                    .as_str()
                    .unwrap();
            assert!(
                step.contains("@author") && step.contains("@version") && step.contains("不编造")
            );
        }
    }
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            task.as_deref().unwrap(),
            p.0.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--java-tool",
            &java,
            "--checkstyle-jar",
            &jar,
            "--format=json",
        ])
        .output()
        .unwrap();
    let verified: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        verified["observation"], "candidate_absent_unverified_policy",
        "{verified}"
    );
    assert_eq!(verified["event_persisted"], true);
    // 无效正则必须由原生配置失败反馈，不能产生源码违规或假干净结果。
    fs::write(&config, configuration.replace("team-[a-z]+", "[")).unwrap();
    for expected_blockers in [1, 0] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--workspace",
                p.0.to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &java,
                "--checkstyle-jar",
                &jar,
                "--format=json",
            ])
            .output()
            .unwrap();
        let scan: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(scan["local_status"], "incomplete", "{scan}");
        assert!(scan["findings"].as_array().unwrap().is_empty());
        assert_eq!(scan["reason"], "checkstyle_configuration_regex_invalid");
        assert_eq!(
            scan["next_actions"][0],
            "repair_original_checkstyle_regex_configuration"
        );
        let brief = &scan["workbench"]["next"]["repair_brief"];
        assert_eq!(brief["checker_id"], "java.checkstyle.preparation");
        assert!(brief["step"].as_str().unwrap().contains("正则"));
        assert_eq!(scan["workbench"]["new_findings"], 0);
        assert_eq!(scan["workbench"]["new_blockers"], expected_blockers);
    }
    for entry in fs::read_dir(p.0.join(".codeguard/findings")).unwrap() {
        let fact: Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path().join("finding.json")).unwrap())
                .unwrap();
        assert_eq!(fact["state"], "open");
    }
}
