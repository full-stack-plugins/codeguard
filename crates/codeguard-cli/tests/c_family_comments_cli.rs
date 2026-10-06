#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-c-doc-{name}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn source(&self, language: &str, text: &str) -> PathBuf {
        let path = self
            .0
            .join(if language == "c" { "api.c" } else { "api.cpp" });
        fs::write(&path, text).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn sarif(rule: &str) -> Value {
    json!({"version":"2.1.0","runs":[{"columnKind":"unicodeCodePoints","invocations":[{"executionSuccessful":true}],"artifacts":[{"location":{"uri":"file://","index":0}}],"tool":{"driver":{"name":"clang","version":"Apple clang version 21.0.0 (clang-2100.3.34.2)","rules":[{"id":rule}]}},"results":[{"level":"warning","ruleId":rule,"ruleIndex":0,"message":{"text":"UNTRUSTED_DOC_PAYLOAD"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"file://","index":0},"region":{"startLine":2,"startColumn":4}}}]}]}]})
}
fn tool(fixture: &Fixture, report: &Value) -> PathBuf {
    let path = fixture.0.join("clang");
    fs::write(&path, format!("#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'Apple clang version 21.0.0 (clang-2100.3.34.2)'; exit 0; fi\nprintf '%s\\n' \"$@\" > '{}'/argv\ncat >/dev/null\ncat <<'CG_REPORT' >&2\n{}\nCG_REPORT\n", fixture.0.display(), report)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
fn invoke(
    source: &Path,
    language: &str,
    tool: &Path,
    standard: &str,
    json: bool,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", language])
        .arg(source)
        .arg("--clang-tool")
        .arg(tool)
        .args([
            "--standard",
            standard,
            if json {
                "--format=json"
            } else {
                "--format=human"
            },
        ])
        .output()
        .unwrap()
}
fn read_report(output: &std::process::Output) -> Value {
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["detailed_contract_qualification"], "not_granted");
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["workspace_binding"], "not_bound");
    assert_eq!(report["next"], Value::Null);
    if let Some(directory) = std::env::var_os("CODEGUARD_C_FAMILY_COMMENTS_REPORT_DIR") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!(
                "report-{}-{}.json",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            )),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    report
}
#[test]
fn native_documentation_profile_is_distinct_source_bound_and_has_repair_guidance() {
    let f = Fixture::new("profile");
    let source = f.source(
        "c",
        "/** Adds one.\n * @param value\n */\nint add(int value) { return value + 1; }\n",
    );
    let selected = tool(&f, &sarif("warn_doc_block_command_empty_paragraph"));
    let r = read_report(&invoke(&source, "c", &selected, "c11", true));
    assert_eq!(r["report_type"], "c_family_comments_feedback");
    assert_eq!(r["local_scan_complete"], true);
    assert_eq!(r["documentation_findings"].as_array().unwrap().len(), 1);
    let finding = &r["documentation_findings"][0];
    assert_eq!(
        finding["rule_id"],
        "clang.warn_doc_block_command_empty_paragraph"
    );
    assert_eq!(finding["line"], 2);
    assert_eq!(finding["column_byte"], 4);
    assert!(finding["repair_steps"].as_array().unwrap().len() >= 2);
    assert_eq!(r["verification_command"][1], "comments");
    assert_eq!(
        r["documentation_configuration"]["origin"],
        "explicit_probe_profile"
    );
    assert_eq!(
        r["documentation_configuration"]["project_configuration"],
        "unknown"
    );
    let argv = fs::read_to_string(f.0.join("argv")).unwrap();
    assert!(argv.contains("-Wdocumentation\n"));
    assert!(argv.contains("-Wdocumentation-pedantic\n"));
    assert!(!argv.contains("-Wall\n"));
    assert!(!r.to_string().contains("UNTRUSTED_DOC_PAYLOAD"));
    assert!(!f.0.join(".codeguard").exists());
    let output = invoke(&source, "c", &selected, "c11", false);
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("clang.warn_doc_block_command_empty_paragraph"));
    assert!(text.contains("详细文档"));
    assert!(!text.contains("UNTRUSTED_DOC_PAYLOAD"));
}
#[test]
fn unknown_or_non_documentation_native_rules_are_not_promoted_by_prefix() {
    for rule in ["warn_unused_variable", "warn_doc_new_unadapted_rule"] {
        let f = Fixture::new("unknown");
        let source=f.source("cpp", "/** Adds one.\n * @param value The input.\n */\nint add(int value) { return value + 1; }\n");
        let selected = tool(&f, &sarif(rule));
        let r = read_report(&invoke(&source, "cpp", &selected, "c++17", true));
        assert_eq!(r["native"]["diagnostics"].as_array().unwrap().len(), 1);
        assert_eq!(r["documentation_findings"], json!([]));
        assert_eq!(
            r["unclassified_native_diagnostics"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}
#[test]
fn invalid_report_and_preprocessor_scope_never_become_documentation_findings() {
    let f = Fixture::new("invalid");
    let mut payload = sarif("warn_doc_block_command_empty_paragraph");
    payload["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] =
        json!("file:///foreign.c");
    let selected = tool(&f, &payload);
    let source = f.source(
        "c",
        "/** Adds one.\n * @param value\n */\nint add(int value) { return value + 1; }\n",
    );
    let r = read_report(&invoke(&source, "c", &selected, "c11", true));
    assert_eq!(r["local_scan_complete"], false);
    assert_eq!(r["native"]["reason"], "clang_report_invalid");
    assert_eq!(r["documentation_findings"], json!([]));
    fs::remove_file(f.0.join("argv")).unwrap();
    fs::write(&source, "#include \"missing.h\"\nint x;\n").unwrap();
    let r = read_report(&invoke(&source, "c", &selected, "c11", true));
    assert_eq!(
        r["native"]["reason"],
        "clang_preprocessor_context_unresolved"
    );
    assert_eq!(r["documentation_findings"], json!([]));
    assert!(!f.0.join("argv").exists());
}
#[test]
fn unsupported_or_unpaired_context_is_rejected_before_tool_execution() {
    let f = Fixture::new("args");
    let source = f.source("c", "int x;\n");
    let selected = tool(&f, &sarif("warn_doc_block_command_empty_paragraph"));
    for args in [
        vec!["--standard=c++17"],
        vec!["--standard=c11", "--standard=c11"],
        vec![],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "c"])
            .arg(&source)
            .arg("--clang-tool")
            .arg(&selected)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(!f.0.join("argv").exists());
    }
}
#[test]
fn native_syntax_errors_do_not_establish_complete_documentation_scanning() {
    let f = Fixture::new("syntax-errors");
    let source = f.source("c", "/** Adds one.\n * @param value\n */\nint x = ;\n");
    let mut payload = sarif("err_expected_expression");
    payload["runs"][0]["invocations"][0]["executionSuccessful"] = json!(false);
    payload["runs"][0]["results"][0]["level"] = json!("error");
    let selected = tool(&f, &payload);
    let script = fs::read_to_string(&selected).unwrap();
    fs::write(&selected, format!("{script}exit 1\n")).unwrap();
    let r = read_report(&invoke(&source, "c", &selected, "c11", true));
    assert_eq!(r["native"]["status"], "diagnostics_observed");
    assert_eq!(r["local_scan_complete"], false);
    assert_eq!(r["reason"], "native_syntax_errors_limit_documentation");
    assert_eq!(r["documentation_findings"], json!([]));
    assert_eq!(
        r["unclassified_native_diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn changed_source_or_tool_and_wrong_version_withdraw_documentation_diagnostics() {
    for modification in ["source", "tool", "version"] {
        let f = Fixture::new(modification);
        let source = f.source(
            "c",
            "/** Adds one.\n * @param value\n */\nint add(int value) { return value + 1; }\n",
        );
        let selected = tool(&f, &sarif("warn_doc_block_command_empty_paragraph"));
        let mut script = fs::read_to_string(&selected).unwrap();
        if modification == "version" {
            script = script.replace(
                "Apple clang version 21.0.0 (clang-2100.3.34.2)",
                "Apple clang version 20.0.0 (unsupported)",
            );
        } else {
            let action = if modification == "source" {
                format!("printf 'int changed;\\n' > '{}'", source.display())
            } else {
                "printf '\\n# altered\\n' >> \"$0\"".into()
            };
            script = script.replace("cat >/dev/null", &format!("cat >/dev/null\n{action}"));
        }
        fs::write(&selected, script).unwrap();
        let r = read_report(&invoke(&source, "c", &selected, "c11", true));
        assert_eq!(r["local_scan_complete"], false);
        assert_eq!(r["documentation_findings"], json!([]));
        assert_eq!(
            r["native"]["reason"],
            match modification {
                "source" => "clang_source_changed",
                "tool" => "clang_tool_changed",
                _ => "clang_version_unverified",
            }
        );
    }
}
#[test]
fn unavailable_tool_linked_source_and_deadline_have_no_documentation_finding() {
    let f = Fixture::new("environment");
    let source = f.source(
        "c",
        "/** Adds one.\n * @param value\n */\nint add(int value) { return value + 1; }\n",
    );
    let r = read_report(&invoke(
        &source,
        "c",
        &f.0.join("missing-clang"),
        "c11",
        true,
    ));
    assert_eq!(r["native"]["reason"], "clang_tool_unavailable");
    assert_eq!(r["documentation_findings"], json!([]));
    let selected = tool(&f, &sarif("warn_doc_block_command_empty_paragraph"));
    let linked = f.0.join("link.c");
    std::os::unix::fs::symlink(&source, &linked).unwrap();
    let r = read_report(&invoke(&linked, "c", &selected, "c11", true));
    assert_eq!(r["native"]["reason"], "source_path_symlink_disallowed");
    assert!(!f.0.join("argv").exists());
    let script = fs::read_to_string(&selected)
        .unwrap()
        .replace("cat >/dev/null", "cat >/dev/null\nsleep 5");
    fs::write(&selected, script).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "c"])
        .arg(&source)
        .arg("--clang-tool")
        .arg(&selected)
        .args(["--standard=c11", "--timeout=2s", "--format=json"])
        .output()
        .unwrap();
    let r = read_report(&output);
    assert_eq!(r["local_scan_complete"], false);
    assert_eq!(r["native"]["reason"], "clang_execution_incomplete");
    assert_eq!(r["documentation_findings"], json!([]));
}
#[test]
fn sigint_cancels_the_native_documentation_process_group_and_returns_130() {
    let f = Fixture::new("cancel");
    let source = f.source(
        "c",
        "/** Adds one.\n * @param value\n */\nint add(int value) { return value + 1; }\n",
    );
    let selected = tool(&f, &sarif("warn_doc_block_command_empty_paragraph"));
    let marker = f.0.join("scanning");
    let ghost = f.0.join("ghost");
    let script = fs::read_to_string(&selected).unwrap().replace(
        "cat >/dev/null",
        &format!(
            "cat >/dev/null\n: > '{}'\n(sleep 2; printf ghost > '{}') &\nsleep 10",
            marker.display(),
            ghost.display()
        ),
    );
    fs::write(&selected, script).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "c"])
        .arg(&source)
        .arg("--clang-tool")
        .arg(&selected)
        .args(["--standard=c11", "--timeout=20s", "--format=json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while !marker.exists() {
        assert!(child.try_wait().unwrap().is_none(), "原生阶段必须先启动");
        assert!(std::time::Instant::now() < deadline, "没有进入原生文档检查");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130), "{output:?}");
    let r: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(r["command_status"], "cancelled");
    assert_eq!(r["local_scan_complete"], false);
    assert_eq!(r["native"]["reason"], "request_cancelled");
    assert_eq!(r["documentation_findings"], json!([]));
    std::thread::sleep(std::time::Duration::from_millis(2200));
    assert!(!ghost.exists(), "取消后子孙进程不得延迟写入");
    if let Some(directory) = std::env::var_os("CODEGUARD_C_FAMILY_COMMENTS_REPORT_DIR") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!("cancelled-{}.json", std::process::id())),
            serde_json::to_vec_pretty(&r).unwrap(),
        )
        .unwrap();
    }
}
#[test]
#[ignore = "requires existing Apple Clang 21 via CODEGUARD_CLANG_BIN"]
fn actual_clang_documentation_cases_preserve_native_rules_and_known_gaps() {
    use sha2::{Digest, Sha256};
    let executable_sha256 = format!(
        "{:x}",
        Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
    );
    let mut cases = Vec::new();
    let selected = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").unwrap());
    for (language, standard) in [("c", "c11"), ("cpp", "c++17")] {
        let f = Fixture::new(language);
        for (name, source, rule, count) in [
            (
                "param",
                "/** Adds one.\n * @param value\n * @return The input plus one.\n */\nint add(int value) { return value + 1; }\n",
                "clang.warn_doc_block_command_empty_paragraph",
                1,
            ),
            (
                "name",
                "/** Adds one.\n * @param other The input.\n * @return The input plus one.\n */\nint add(int value) { return value + 1; }\n",
                "clang.warn_doc_param_not_found",
                1,
            ),
            (
                "return",
                "/** Adds one.\n * @param value The input.\n * @return\n */\nint add(int value) { return value + 1; }\n",
                "clang.warn_doc_block_command_empty_paragraph",
                1,
            ),
            (
                "void",
                "/** Performs no work.\n * @return The input.\n */\nvoid noop(void) {}\n",
                "clang.warn_doc_returns_attached_to_a_void_function",
                1,
            ),
            (
                "bare",
                "/** @param value\n * @return\n */\nint add(int value) { return value + 1; }\n",
                "clang.warn_doc_block_command_empty_paragraph",
                2,
            ),
            (
                "unicode",
                "/** é说明。\n * @param value\n * @return The input plus one.\n */\nint add(int value) { return value + 1; }\n",
                "clang.warn_doc_block_command_empty_paragraph",
                1,
            ),
            (
                "good",
                "/** Adds one.\n * @param value The input.\n * @return The input plus one.\n */\nint add(int value) { return value + 1; }\n",
                "",
                0,
            ),
            (
                "missing",
                "int add(int value) { return value + 1; }\n",
                "",
                0,
            ),
        ] {
            let path = f.source(language, source);
            let r = read_report(&invoke(&path, language, &selected, standard, true));
            assert_eq!(r["local_scan_complete"], true, "{name}: {r}");
            assert_eq!(
                r["documentation_findings"].as_array().unwrap().len(),
                count,
                "{name}: {r}"
            );
            if count > 0 {
                assert_eq!(
                    r["documentation_findings"][0]["rule_id"], rule,
                    "{name}: {r}"
                );
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), source);
            assert!(!f.0.join(".codeguard").exists());
            assert_eq!(
                r["source_sha256"],
                format!("{:x}", Sha256::digest(source.as_bytes()))
            );
            cases.push(json!({"case_id":format!("{language}-{name}"),"expected_documentation_diagnostics":count,"report":r}));
        }
    }
    assert_eq!(
        executable_sha256,
        format!(
            "{:x}",
            Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
        )
    );
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_C_FAMILY_COMMENTS_NATIVE_EVIDENCE") {
        let report = json!({"evidence_kind":"local_native_development_regression","independent_holdout":false,"qualification":"not_granted","codeguard_sha256":executable_sha256,"test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_comments_cli.rs"))),"cases":cases});
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
