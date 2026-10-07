#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture(name: &str) -> Fixture {
    let p = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-clang-lint-{name}-{}", std::process::id()));
    fs::create_dir(&p).unwrap();
    Fixture(p)
}
fn tool(f: &Fixture, report: &Value) -> PathBuf {
    let p = f.0.join("clang");
    let payload = serde_json::to_string(report).unwrap();
    let exit = if report["runs"][0]["invocations"][0]["executionSuccessful"] == true {
        0
    } else {
        1
    };
    fs::write(&p,format!("#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'Apple clang version 21.0.0 (clang-2100.3.34.2)'; exit 0; fi\ncat >/dev/null\ncat <<'CG_REPORT' >&2\n{payload}\nCG_REPORT\nexit {exit}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    p
}
fn sarif() -> Value {
    json!({"version":"2.1.0","runs":[{"columnKind":"unicodeCodePoints","invocations":[{"executionSuccessful":false}],"artifacts":[{"location":{"uri":"file://","index":0}}],"tool":{"driver":{"name":"clang","version":"Apple clang version 21.0.0 (clang-2100.3.34.2)","rules":[{"id":"err_expected_expression"}]}},"results":[{"level":"error","ruleId":"err_expected_expression","ruleIndex":0,"message":{"text":"UNTRUSTED_DIAGNOSTIC_TEXT"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"file://","index":0},"region":{"startLine":1,"startColumn":9}}}]}]}]})
}
fn run(f: &Fixture, tool: &Path, source: &str, language: &str, standard: &str) -> Value {
    let p = f.0.join(if language == "c" { "a.c" } else { "a.cpp" });
    fs::write(&p, source).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", language])
        .arg(&p)
        .args([
            "--clang-tool",
            tool.to_str().unwrap(),
            "--standard",
            standard,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3), "{o:?}");
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    if let Some(directory) = std::env::var_os("CODEGUARD_CLANG_REPORT_DIR") {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!(
                "{}-{}.json",
                language,
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            )),
            serde_json::to_vec_pretty(&r).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(fs::read_to_string(p).unwrap(), source);
    r
}
#[test]
fn explicit_clang_reports_rule_and_location_without_raw_diagnostic_or_wasm() {
    let f = fixture("native");
    let t = tool(&f, &sarif());
    let r = run(&f, &t, "int x = ;\n", "c", "c11");
    assert_eq!(r["native"]["status"], "diagnostics_observed");
    assert_eq!(
        r["native"]["diagnostics"][0]["rule_id"],
        "clang.err_expected_expression"
    );
    assert_eq!(r["native"]["diagnostics"][0]["line"], 1);
    assert_eq!(r["native"]["diagnostics"][0]["column_byte"], 9);
    assert_eq!(r["verification_command"][0], "codeguard");
    assert_eq!(r["verification_command"][2], "c");
    assert_eq!(r["verification_command"][7], "c11");
    assert_eq!(r["syntax_candidates"], Value::Null);
    assert_eq!(r["delivery_decision"], "not_evaluated");
    assert!(!r.to_string().contains("UNTRUSTED_DIAGNOSTIC_TEXT"));
}
#[test]
fn malformed_native_report_never_falls_back_or_becomes_a_finding() {
    let f = fixture("badreport");
    let mut value = sarif();
    value["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] =
        json!("file:///foreign.c");
    let t = tool(&f, &value);
    let r = run(&f, &t, "int x = ;\n", "c", "c11");
    assert_eq!(r["native"]["status"], "incomplete");
    assert_eq!(r["native"]["reason"], "clang_report_invalid");
    assert_eq!(r["findings"], json!([]));
    assert_eq!(r["syntax_candidates"], Value::Null);
}
#[test]
fn preprocessor_context_and_inappropriate_standard_fail_before_compiling() {
    let f = fixture("context");
    let t = tool(&f, &sarif());
    let r = run(&f, &t, "#include \"missing.h\"\nint x;\n", "c", "c11");
    assert_eq!(
        r["native"]["reason"],
        "clang_preprocessor_context_unresolved"
    );
    assert_eq!(r["native"]["diagnostics"], json!([]));
    let p = f.0.join("a.c");
    for tail in [
        vec!["--standard=c++17"],
        vec!["--standard=c11", "--standard", "c11"],
        vec!["--clang-tool", t.to_str().unwrap()],
    ] {
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "c"])
            .arg(&p)
            .args(tail)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2), "{o:?}");
    }
}
#[test]
#[ignore = "requires explicit existing Apple Clang 21 via CODEGUARD_CLANG_BIN"]
fn actual_clang_c_and_cpp_report_error_then_repair_without_source_side_effects() {
    let f = fixture("actual");
    let t = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").unwrap());
    for (language, standard) in [("c", "c11"), ("cpp", "c++17")] {
        let bad = run(&f, &t, "int x = ;\n", language, standard);
        assert_eq!(bad["native"]["status"], "diagnostics_observed", "{bad}");
        let unicode = "const char *s=\"é\"; int x = ;\n";
        let unicode_report = run(&f, &t, unicode, language, standard);
        assert_eq!(
            unicode_report["native"]["status"], "diagnostics_observed",
            "{unicode_report}"
        );
        assert_eq!(
            unicode_report["native"]["diagnostics"][0]["column_byte"]
                .as_u64()
                .unwrap() as usize,
            unicode.as_bytes().iter().rposition(|b| *b == b';').unwrap() + 1
        );
        let warning = run(
            &f,
            &t,
            "int f(void) { int unused = 1; return 0; }\n",
            language,
            standard,
        );
        assert_eq!(
            warning["native"]["status"], "diagnostics_observed",
            "{warning}"
        );
        assert_eq!(warning["native"]["diagnostics"][0]["level"], "warning");
        let good = run(&f, &t, "int x = 1;\n", language, standard);
        assert_eq!(good["native"]["status"], "completed", "{good}");
        assert_eq!(good["coverage_proven"], false);
        assert_eq!(good["setup"]["requirement"], "required");
    }
}

#[test]
fn human_native_feedback_includes_safe_rule_position_and_correct_adapter_status() {
    let f = fixture("human");
    let t = tool(&f, &sarif());
    let p = f.0.join("a.c");
    fs::write(&p, "int x = ;\n").unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "c"])
        .arg(&p)
        .args(["--clang-tool", t.to_str().unwrap(), "--standard=c11"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("clang.err_expected_expression"), "{text}");
    assert!(!text.contains("UNTRUSTED_DIAGNOSTIC_TEXT"));
    assert!(!text.contains("原生适配器尚未接入"));
}

#[test]
fn native_warnings_are_retained_even_when_clang_exits_zero() {
    let f = fixture("warning");
    let mut value = sarif();
    value["runs"][0]["invocations"][0]["executionSuccessful"] = json!(true);
    value["runs"][0]["results"][0]["level"] = json!("warning");
    value["runs"][0]["results"][0]["ruleId"] = json!("warn_unused_variable");
    value["runs"][0]["tool"]["driver"]["rules"][0]["id"] = json!("warn_unused_variable");
    let t = tool(&f, &value);
    let r = run(
        &f,
        &t,
        "int f(void) { int unused = 1; return 0; }\n",
        "c",
        "c11",
    );
    assert_eq!(r["native"]["status"], "diagnostics_observed", "{r}");
    assert_eq!(r["native"]["diagnostics"][0]["level"], "warning");
}

fn run_named(
    f: &Fixture,
    tool: &Path,
    source: &str,
    language: &str,
    standard: &str,
    name: &str,
) -> Value {
    let p = f.0.join(name);
    fs::write(&p, source).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", language])
        .arg(&p)
        .args([
            "--clang-tool",
            tool.to_str().unwrap(),
            "--standard",
            standard,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3), "{o:?}");
    serde_json::from_slice(&o.stdout).unwrap()
}

#[test]
fn explicit_header_files_run_native_clang_and_keep_ambiguity_honest() {
    let f = fixture("header");
    let t = tool(&f, &sarif());
    for (language, name, standard) in [
        ("c", "a.h", "c11"),
        ("cpp", "a.hpp", "c++17"),
        ("cpp", "b.hh", "c++17"),
        ("cpp", "c.hxx", "c++17"),
    ] {
        let r = run_named(&f, &t, "int x = ;\n", language, standard, name);
        assert_eq!(r["native"]["status"], "diagnostics_observed", "{name}: {r}");
        assert_eq!(
            r["native"]["diagnostics"][0]["rule_id"], "clang.err_expected_expression",
            "{name}"
        );
        assert!(r["source_sha256"].is_string(), "{name}: {r}");
    }
    // 预处理守卫对头文件同样生效：include 指令保持上下文未解析，不制造源码违规。
    let r = run_named(
        &f,
        &t,
        "#include \"missing.h\"\nint x = ;\n",
        "cpp",
        "c++17",
        "d.hpp",
    );
    assert_eq!(
        r["native"]["reason"], "clang_preprocessor_context_unresolved",
        "{r}"
    );
    assert_eq!(r["native"]["diagnostics"], json!([]));
    // `.h` 在 cpp 语言下仍有 C/C++ 双重身份歧义；显式拒绝而不是猜成 C++。
    let r = run_named(&f, &t, "int x = ;\n", "cpp", "c++17", "e.h");
    assert_eq!(r["native"]["status"], "incomplete", "{r}");
    assert_eq!(r["native"]["reason"], "clang_source_scope_unavailable");
    assert_eq!(r["source_sha256"], Value::Null);
}

#[test]
#[ignore = "requires explicit existing Apple Clang21 via CODEGUARD_CLANG_BIN; never installs"]
fn actual_clang_headers_report_syntax_then_clean_repair() {
    let f = fixture("actual-header");
    let t = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").unwrap());
    let mut evidence = Vec::new();
    for (language, name, standard) in [
        ("c", "a.h", "c11"),
        ("cpp", "a.hpp", "c++17"),
        ("cpp", "b.hh", "c++17"),
        ("cpp", "c.hxx", "c++17"),
    ] {
        let bad = run_named(&f, &t, "int x = ;\n", language, standard, name);
        assert_eq!(
            bad["native"]["status"], "diagnostics_observed",
            "{name}: {bad}"
        );
        assert_eq!(
            bad["native"]["diagnostics"][0]["rule_id"], "clang.err_expected_expression",
            "{name}"
        );
        let good = run_named(&f, &t, "int x = 1;\n", language, standard, name);
        assert_eq!(good["native"]["status"], "completed", "{name}: {good}");
        assert_eq!(good["native"]["reason"], "clang_native_no_diagnostics");
        assert_eq!(good["coverage_proven"], false);
        evidence.push(json!({"name":name,"bad":bad,"good":good}));
    }
    let ambiguous = run_named(&f, &t, "int x = ;\n", "cpp", "c++17", "d.h");
    assert_eq!(
        ambiguous["native"]["reason"],
        "clang_source_scope_unavailable"
    );
    evidence.push(json!({"name":"d.h","ambiguous_cpp_h":ambiguous}));
    if let Some(path) = std::env::var_os("CODEGUARD_CLANG_HEADER_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
fn hash_literals_and_comments_do_not_require_preprocessor_context() {
    let f = fixture("hash-literals");
    let mut report = sarif();
    report["runs"][0]["invocations"][0]["executionSuccessful"] = json!(true);
    report["runs"][0]["results"] = json!([]);
    let t = tool(&f, &report);
    for (language, standard, source) in [
        ("c", "c11", "const char *s = \"#text\";\n"),
        ("c", "c11", "int x = '#'; /* #include \"no.h\" */\n"),
        (
            "cpp",
            "c++17",
            "const char *s=R\"cg(\n#include \"no.h\"\n)cg\";\n",
        ),
    ] {
        let r = run(&f, &t, source, language, standard);
        assert_eq!(r["native"]["status"], "completed", "{r}");
        assert_eq!(r["coverage_proven"], false);
        assert_eq!(r["syntax_candidates"], Value::Null);
    }
}

#[test]
fn alternate_preprocessing_directives_stop_before_native_tool_identity() {
    let f = fixture("alternate-pp");
    let t = tool(&f, &sarif());
    for (language, standard, source) in [
        ("c", "c11", "\u{feff}#error CODEGUARD_PP_PROBE\n"),
        ("c", "c11", "\u{a0}#error CODEGUARD_PP_PROBE\n"),
        ("cpp", "c++17", "%\\ \n:error CODEGUARD_PP_PROBE\n"),
        ("c", "c11", "%:include \"missing.h\"\n"),
        ("c", "c11", "??=include \"missing.h\"\n"),
        ("cpp", "c++17", "%\\\n:include \"missing.h\"\n"),
        (
            "cpp",
            "c++17",
            "const char *s=R\"cg(#literal)cg\";\n#include \"missing.h\"\n",
        ),
    ] {
        let r = run(&f, &t, source, language, standard);
        assert_eq!(
            r["native"]["reason"], "clang_preprocessor_context_unresolved",
            "{r}"
        );
        assert_eq!(r["native"]["tool_sha256"], Value::Null);
        assert_eq!(r["native"]["diagnostics"], json!([]));
        assert_eq!(r["syntax_candidates"], Value::Null);
    }
}

#[test]
#[ignore = "requires explicit existing Apple Clang21 via CODEGUARD_CLANG_BIN; never installs"]
fn actual_clang_literals_raw_translation_boundaries_and_directives_are_distinct() {
    let f = fixture("actual-pp");
    let t = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").unwrap());
    let mut evidence = Vec::new();
    for (language, standard) in [("c", "c11"), ("cpp", "c++17")] {
        for source in [
            "const char *s=\"#text\";\n",
            "int x='#'; /* #include \"missing.h\" */\n",
            "/*\n#include \"missing.h\"\n*/ int x=1;\n",
        ] {
            let r = run(&f, &t, source, language, standard);
            assert_eq!(r["native"]["status"], "completed", "{r}");
            assert_eq!(r["native"]["reason"], "clang_native_no_diagnostics");
            assert!(r["native"]["tool_sha256"].is_string());
            evidence.push(json!({"source":source,"report":r}));
        }
        let bad = "const char *s=\"#text\"; int x = ;\n";
        let r = run(&f, &t, bad, language, standard);
        assert_eq!(r["native"]["status"], "diagnostics_observed", "{r}");
        assert_eq!(
            r["native"]["diagnostics"][0]["rule_id"],
            "clang.err_expected_expression"
        );
        evidence.push(json!({"source":bad,"report":r}));
        for source in [
            "%:include \"missing.h\"\n",
            "/* context */ #include \"missing.h\"\n",
        ] {
            let r = run(&f, &t, source, language, standard);
            assert_eq!(
                r["native"]["reason"],
                "clang_preprocessor_context_unresolved"
            );
            assert_eq!(r["native"]["tool_sha256"], Value::Null);
            assert_eq!(r["native"]["diagnostics"], json!([]));
            evidence.push(json!({"source":source,"report":r}));
        }
    }
    for source in [
        "const char *s=R\"cg(\n#include \"missing.h\"\n)cg\";\n",
        "const char *s=u8R\"cg(\n)cg\\\n\"\n#include \"missing.h\"\n)cg\";\n",
        "const char *s=R\\\n\"cg(\n#include \"missing.h\"\n)cg\";\n",
    ] {
        let r = run(&f, &t, source, "cpp", "c++17");
        assert_eq!(r["native"]["status"], "completed", "{r}");
        assert_eq!(r["coverage_proven"], false);
        assert_eq!(r["delivery_decision"], "not_evaluated");
        evidence.push(json!({"source":source,"report":r}));
    }
    if let Some(path) = std::env::var_os("CODEGUARD_CLANG_CONTEXT_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
