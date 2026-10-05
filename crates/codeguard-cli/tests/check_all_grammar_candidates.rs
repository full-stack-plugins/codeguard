#![cfg(all(feature = "wasm-precheck", unix))]

use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

#[test]
fn hidden_kotlin_recovery_is_visible_in_project_feedback() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-kotlin-hidden-project-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Main.kt"), "fun f(x: ) = x\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        // 固定缺原生工具分支，避免宿主 kotlinc 抢占候选观察。
        .env("PATH", &root)
        .args(["check", "all"])
        .arg(&root)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    let kotlin = observations
        .iter()
        .find(|item| item["language"] == "kotlin")
        .expect("Kotlin candidate observation");
    assert_eq!(kotlin["recovery_count"], 0);
    assert_eq!(kotlin["reason"], "syntax_recovery_incomplete");
    assert_eq!(report["delivery_decision"], "incomplete");
    let text_output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        // 固定缺原生工具分支，避免宿主 kotlinc 抢占候选观察。
        .env("PATH", &root)
        .args(["check", "all"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(text_output.status.code(), Some(3));
    assert!(
        String::from_utf8_lossy(&text_output.stdout).contains("grammar 报告错误但恢复位置不完整"),
        "{}",
        String::from_utf8_lossy(&text_output.stdout)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn completed_native_ruff_preempts_only_its_matching_python_file() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-native-preferred-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("app.py"), "import os\n").unwrap();
    fs::write(root.join("main.zig"), "const Empty = struct {};\n").unwrap();
    fs::write(root.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = root.join("fake-ruff");
    fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'ruff 0.16.8'; exit 0; fi\nif [ \"$2\" = '--show-files' ]; then echo \"$3\"; exit 0; fi\nif [ \"$2\" = '--show-settings' ]; then printf 'linter.rules.enabled = [\\n\\tunused-import (F401),\\n]\\nlinter.per_file_ignores = {}\\n'; exit 0; fi\nif [ \"$2\" = '--no-cache' ]; then if [ \"$3\" = '--ignore-noqa' ]; then source=$6; else source=$5; fi; printf '[{\"code\":\"F401\",\"message\":\"unused\",\"filename\":\"%s\",\"location\":{\"row\":1,\"column\":1},\"severity\":\"error\"}]\\n' \"$source\"; exit 1; fi\nexit 2\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        // 显式 Ruff 仍可执行；隔离宿主 Zig 等工具，固定相邻文件的 WASM 分支。
        .env("PATH", &root)
        .args(["check", "all"])
        .arg(&root)
        .args([
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
            "--timeout",
            "40s",
        ])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["native_results"]["python_lint"]["files"][0]["run_status"], "findings",
        "{report}"
    );
    assert_eq!(
        report["native_results"]["python_lint"]["files"][0]["findings"][0]["rule_id"],
        "F401"
    );
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 1);
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    assert!(!observations.iter().any(|row| row["path"] == "app.py"));
    assert!(
        observations
            .iter()
            .any(|row| row["path"] == "main.zig" && row["status"] == "candidate_observed")
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn check_all_routes_distinct_dialects_after_native_without_claiming_clean() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-check-grammar-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("component.tsx"), "const C = () => <div />;\n").unwrap();
    fs::write(root.join("script.js"), "const x = 1;\n").unwrap();
    fs::write(
        root.join("plot.m"),
        "title('@interface Foo');\n%{\n#import <NotObjectiveC.h>\n@interface Fake\n%}\nplot(1:3);\n",
    )
    .unwrap();
    fs::write(root.join("synth.sc"), "{ SinOsc.ar(440) }.play;\n").unwrap();
    fs::write(
        root.join("component.cfs"),
        "component { function f() { return 1; } }\n",
    )
    .unwrap();
    fs::write(
        root.join("page.cfm"),
        "<!-- <cfquery>SELECT #html# FROM users</cfquery> -->\n<!--- <cfquery>SELECT #broken</cfquery> --->\n<cfquery <!--- note > ---> name=\"q\">SELECT #x# FROM users</cfquery>\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "90s", "--jobs=1"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "0.38.0");
    assert_eq!(report["execution_budget"]["jobs_limit"], 1);
    assert_eq!(report["delivery_decision"], "incomplete");
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    for language in ["tsx", "javascript", "cfml", "cfscript", "cfquery"] {
        assert!(
            observations.iter().any(|item| item["language"] == language),
            "{language}: {observations:?}"
        );
    }
    assert!(!observations.iter().any(|item| item["path"] == "plot.m"));
    assert!(!observations.iter().any(|item| item["path"] == "synth.sc"));
    assert_eq!(report["syntax_candidates"]["unrouted_count"], 2);
    let unknown = report["discovery"]["unknown_conditions"]
        .as_array()
        .unwrap();
    for path in ["plot.m", "synth.sc"] {
        assert!(
            unknown
                .iter()
                .any(|item| item == &format!("ambiguous_language_suffix:{path}")),
            "{path}: {unknown:?}"
        );
    }
    assert_eq!(
        observations
            .iter()
            .filter(|item| item["language"] == "cfquery")
            .count(),
        2,
        "HTML-comment CFML executes; only CFML-comment query is excluded"
    );
    assert!(
        observations
            .iter()
            .all(|item| item["grammar_qualified"] == false)
    );
    assert!(observations.iter().all(|item| item["status"] != "clean"));
    assert_eq!(
        report["syntax_candidates"]["execution_phase"],
        "after_native"
    );
}

#[test]
fn ambiguous_only_project_remains_an_explicit_unrouted_scope() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ambiguous-only-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("plot.m"), "plot(1:3);\n").unwrap();
    fs::write(root.join("synth.sc"), "{ SinOsc.ar(440) }.play;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "30s"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["syntax_candidates"]["source_file_count"], 2);
    assert_eq!(report["syntax_candidates"]["unrouted_count"], 2);
    assert_eq!(report["syntax_candidates"]["status"], "not_run");
    assert_eq!(report["syntax_candidates"]["reason"], "unrouted_source");
    assert_eq!(report["delivery_decision"], "incomplete");
    assert!(
        report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn known_grammar_precision_limits_reach_project_feedback() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-known-grammar-limits-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("class.vb"),
        "Public Class C\nPublic Function F() As Integer\nReturn 1\nEnd Function\nEnd Class\n",
    )
    .unwrap();
    fs::write(
        root.join("query.cfm"),
        "<cfquery name=\"q\">SELECT FROM users</cfquery>\n",
    )
    .unwrap();
    fs::write(root.join("broken.kt"), "fun f(x: ) = x\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        // 固定缺原生工具分支，避免宿主 kotlinc 抢占候选观察。
        .env("PATH", &root)
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "45s"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    for (language, expected) in [
        ("vbnet", "known grammar false positive"),
        ("cfquery", "does not validate full SQL semantics"),
        ("kotlin", "native compiler rejects missing parameter type"),
    ] {
        let observation = observations
            .iter()
            .find(|item| item["language"] == language)
            .unwrap();
        assert!(
            observation["known_limitations"][0]
                .as_str()
                .unwrap()
                .contains(expected),
            "{language}: {observation}"
        );
        assert_eq!(observation["grammar_qualified"], false);
        if language == "kotlin" {
            assert_eq!(observation["recovery_count"], 0);
        }
    }
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn human_feedback_names_known_candidate_false_positive() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-known-limit-human-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("class.vb"),
        "Public Class C\nPublic Function F() As Integer\nReturn 1\nEnd Function\nEnd Class\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args(["--timeout", "45s"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("已知 grammar 限制"), "{text}");
    assert!(text.contains("known grammar false positive"), "{text}");
    assert!(!text.contains("Return 1"), "{text}");
}

fn grammar_samples() -> [(&'static str, &'static str); 31] {
    [
        ("a.ets", "@Component struct C { build() { Text('hi') } }"),
        ("a.c", "int main(void) { return 0; }"),
        (
            "a.cfm",
            "<cfquery name=\"q\">SELECT #x# FROM users</cfquery>",
        ),
        ("a.cfs", "component { function f() { return 1; } }"),
        (
            "a.cbl",
            "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO.\n",
        ),
        ("a.cpp", "int main() { return 0; }"),
        ("a.cs", "class C { static int F() { return 1; } }"),
        ("a.dart", "String greet(String name) => 'Hello $name';"),
        ("a.erl", "-module(hello).\nhello() -> ok.\n"),
        ("a.go", "package main\nfunc main() {}\n"),
        ("a.java", "class A {}"),
        ("a.js", "const value = 1;"),
        ("a.kt", "fun main() { val x = 1 }"),
        ("a.lua", "local x = 1\n"),
        ("a.luau", "local x: number = 1\n"),
        ("a.nix", "let x = 1; in x"),
        ("a.m", "@interface Foo : NSObject\n@end\n"),
        ("a.pas", "program Hello;\nbegin\n  writeln('Hi');\nend.\n"),
        ("a.php", "<?php function f() { return 1; }"),
        ("a.py", "x = 1\n"),
        ("a.r", "x <- 1\n"),
        ("a.rb", "def f; 1; end\n"),
        ("a.rs", "fn main() { let x = 1; }"),
        ("a.scala", "object Main { def f(x: Int): Int = x + 1 }"),
        ("a.sol", "pragma solidity ^0.8.20;\ncontract Vault {}\n"),
        ("a.swift", "func f() -> Int { return 1 }"),
        ("a.tf", "resource \"x\" \"y\" { foo = \"bar\" }"),
        ("a.tsx", "const C = () => <div />;"),
        ("a.ts", "const value: number = 1;"),
        (
            "a.vb",
            "Public Class C\n    Public Function F() As Integer\n        Return 1\n    End Function\nEnd Class\n",
        ),
        ("a.zig", "const Empty = struct {};\n"),
    ]
}

#[test]
fn check_all_invokes_all_32_pinned_candidates_across_bounded_projects() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-check-all-grammars-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for (index, (name, source)) in grammar_samples().into_iter().enumerate() {
        let group = root.join(format!("group-{}", index / 8));
        fs::create_dir_all(&group).unwrap();
        fs::write(group.join(name), source).unwrap();
    }
    let mut languages = BTreeSet::new();
    for group_index in 0..4 {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "all"])
            .arg(root.join(format!("group-{group_index}")))
            .args(["--format=json", "--timeout", "120s"])
            .env("PATH", "") // 固定无原生工具，验收全部候选资产路由。
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report["syntax_candidates"]["skipped_count"], 0,
            "group {group_index}"
        );
        assert_eq!(
            report["syntax_candidates"]["delivery_decision"],
            "incomplete"
        );
        let observations = report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap();
        for observation in observations {
            assert_eq!(
                observation["recovery_count"], 0,
                "group {group_index}: unexpected recovery in valid sample: {observation}"
            );
            assert_eq!(
                observation["reason"],
                serde_json::Value::Null,
                "group {group_index}: valid sample has an incomplete parse: {observation}"
            );
            assert!(
                !observation["known_limitations"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                "group {group_index}: candidate lacks its fixed known limits: {observation}"
            );
        }
        for language in observations
            .iter()
            .filter(|item| item["status"] == "candidate_observed")
            .filter_map(|item| item["language"].as_str())
        {
            languages.insert(language.to_owned());
        }
    }
    fs::remove_dir_all(&root).unwrap();
    let expected =
        serde_json::from_str::<serde_json::Value>(include_str!("../../../grammars/manifest.json"))
            .unwrap()["assets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|asset| asset["language"].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>();
    assert_eq!(languages, expected);
}

#[test]
fn one_mixed_project_observes_all_32_candidates_within_the_existing_budget() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-single-project-grammars-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for (name, source) in grammar_samples() {
        fs::write(root.join(name), source).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "120s"])
        .env("PATH", "") // 原生优先另有验收，此处要求实际调用全部 WASM。
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["syntax_candidates"]["skipped_count"], 0);
    assert_eq!(report["syntax_candidates"]["source_file_count"], 31);
    assert_eq!(report["delivery_decision"], "incomplete");
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    assert_eq!(observations.len(), 32);
    assert!(
        observations
            .windows(2)
            .all(|pair| { pair[0]["path"].as_str().unwrap() <= pair[1]["path"].as_str().unwrap() })
    );
    let observed = observations
        .iter()
        .filter(|item| item["status"] == "candidate_observed")
        .filter_map(|item| item["language"].as_str())
        .collect::<BTreeSet<_>>();
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../grammars/manifest.json")).unwrap();
    let expected = manifest["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|asset| asset["language"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(observed, expected, "{observations:?}");
    assert!(
        observations
            .iter()
            .all(|item| item["grammar_qualified"] == false)
    );
    assert!(
        observations.iter().all(|item| item["reason"].is_null()),
        "a valid sample was not fully parsed: {observations:?}"
    );
}

#[test]
fn oversized_source_is_an_explicit_candidate_gap() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-check-large-grammar-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("large.zig"), vec![b'a'; 1024 * 1024 + 1]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "30s"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["syntax_candidates"]["status"],
        "attempted_incomplete"
    );
    assert_eq!(
        report["syntax_candidates"]["reason"],
        "candidate_unavailable"
    );
    assert_eq!(
        report["syntax_candidates"]["observations"][0]["status"],
        "candidate_unavailable"
    );
    assert_eq!(
        report["syntax_candidates"]["observations"][0]["grammar_sha256"],
        serde_json::Value::Null
    );
}

#[test]
fn edited_file_hooks_invoke_all_32_candidates_without_scanning_untouched_files() {
    use std::io::Write;
    use std::process::Stdio;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-hook-32-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let mut languages = BTreeSet::new();
    for (index, group) in grammar_samples().chunks(8).enumerate() {
        let project = root.join(format!("group-{index}"));
        fs::create_dir(&project).unwrap();
        fs::write(project.join("untouched.js"), "const = ;").unwrap();
        let paths: Vec<&str> = group
            .iter()
            .map(|(name, source)| {
                fs::write(project.join(name), source).unwrap();
                *name
            })
            .collect();
        let payload = serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request",
            "input":{"event":"file_changed","changed_paths":paths,"task_id":null,
                "write_outcome":"confirmed","host_claims_blocking":false}});
        let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["hook", "execute"])
            .arg(&project)
            .args(["--format=json", "--timeout=120s"])
            .env("PATH", &project)
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
        let result = child.wait_with_output().unwrap();
        assert_eq!(
            result.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let feedback = &report["local_feedback"];
        assert_eq!(feedback["requested_paths"], serde_json::json!(paths));
        assert_eq!(
            feedback["syntax_candidates"]["skipped_count"], 0,
            "{feedback}"
        );
        assert_eq!(feedback["syntax_candidates"]["unrouted_count"], 0);
        for row in feedback["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
        {
            assert_ne!(row["path"], "untouched.js");
            assert_eq!(row["status"], "candidate_observed", "{row}");
            assert_eq!(row["recovery_count"], 0, "{row}");
            assert!(row["reason"].is_null(), "{row}");
            assert_eq!(row["grammar_qualified"], false);
            languages.insert(row["language"].as_str().unwrap().to_owned());
        }
        assert_eq!(feedback["delivery_decision"], "not_evaluated");
    }
    fs::remove_dir_all(root).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../grammars/manifest.json")).unwrap();
    let expected: BTreeSet<String> = manifest["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["language"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(languages, expected);
    assert_eq!(languages.len(), 32);
}

#[test]
fn project_check_observes_typescript_module_sources_with_pinned_grammar() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-module-extensions-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for name in ["module.mts", "module.cts"] {
        fs::write(root.join(name), "export const value: number = ;\n").unwrap();
    }
    for name in ["types.d.mts", "types.d.cts"] {
        fs::write(root.join(name), "export declare const value: number;\n").unwrap();
    }
    fs::write(root.join("module.mtsx"), "export default <div />;").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .env("PATH", &root)
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "60s", "--jobs=2"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["syntax_candidates"]["source_file_count"], 4,
        "{report}"
    );
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    assert_eq!(observations.len(), 4, "{report}");
    for observation in observations {
        assert_eq!(observation["language"], "typescript", "{observation}");
        assert_eq!(observation["grammar_qualified"], false);
        assert_eq!(observation["status"], "candidate_observed", "{observation}");
        let name = observation["path"].as_str().unwrap();
        if name.starts_with("types.") {
            assert_eq!(observation["recovery_count"], 0, "{observation}");
        } else {
            assert!(
                observation["recovery_count"].as_u64().unwrap() > 0,
                "{observation}"
            );
        }
    }
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn typescript_module_tasks_are_stable_and_edit_hook_keeps_changed_scope() {
    use std::io::Write;
    use std::process::Stdio;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-module-workflow-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for name in ["module.mts", "module.cts"] {
        fs::write(root.join(name), "export const value: number = ;\n").unwrap();
    }
    fs::write(
        root.join("types.d.mts"),
        "export declare const value: number;\n",
    )
    .unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init"])
        .arg(&root)
        .args(["--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(
        init.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let mut first_ids = BTreeSet::new();
    for round in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .env("PATH", &root)
            .args(["check", "typescript"])
            .arg(&root)
            .args(["--format=json", "--timeout", "60s"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report["syntax_candidates"]["observations"]
                .as_array()
                .unwrap()
                .len(),
            3,
            "{report}"
        );
        assert_eq!(
            report["syntax_tasks"]["status"], "synced_partial",
            "{report}"
        );
        let tasks = report["syntax_tasks"]["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2, "{report}");
        let ids: BTreeSet<String> = tasks
            .iter()
            .map(|t| t["task_id"].as_str().unwrap().to_owned())
            .collect();
        if round == 0 {
            first_ids = ids;
        } else {
            assert_eq!(ids, first_ids);
        }
    }
    let payload = serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request",
        "input":{"event":"file_changed","changed_paths":["module.mts"],"task_id":null,
            "write_outcome":"confirmed","host_claims_blocking":false}});
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .env("PATH", &root)
        .args(["hook", "execute"])
        .arg(&root)
        .args(["--format=json", "--timeout", "60s"])
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
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let feedback = &report["local_feedback"];
    assert_eq!(
        feedback["requested_paths"],
        serde_json::json!(["module.mts"])
    );
    let observations = feedback["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    assert_eq!(observations.len(), 1, "{report}");
    assert_eq!(observations[0]["path"], "module.mts");
    assert_eq!(observations[0]["language"], "typescript");
    let tasks = feedback["syntax_tasks"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1, "{report}");
    assert!(first_ids.contains(tasks[0]["task_id"].as_str().unwrap()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_check_observes_r_and_cpp_explicit_suffixes() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-r-cpp-suffixes-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let cases = [
        ("upper.R", "r", "x <- (\n", true),
        ("lower.r", "r", "x <- 1\n", false),
        ("upper.C", "cpp", "int value = ;\n", true),
        ("short.cp", "cpp", "int value = ;\n", true),
        ("upper.CPP", "cpp", "int value = ;\n", true),
        ("plus.c++", "cpp", "int value = ;\n", true),
        ("source.cxx", "cpp", "int value = ;\n", true),
        ("header.hxx", "cpp", "struct Value {};\n", false),
    ];
    for (name, _, source, _) in cases {
        fs::write(root.join(name), source).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        // 隔离原生工具，实际执行随包 grammar 的候选分支。
        .env("PATH", &root)
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "60s"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    assert_eq!(observations.len(), cases.len(), "{report}");
    for (name, language, _, invalid) in cases {
        let row = observations.iter().find(|row| row["path"] == name).unwrap();
        assert_eq!(row["language"], language, "{row}");
        assert_eq!(row["status"], "candidate_observed", "{row}");
        assert_eq!(
            row["recovery_count"].as_u64().unwrap() > 0,
            invalid,
            "{row}"
        );
        assert_eq!(row["grammar_qualified"], false, "{row}");
    }
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn human_feedback_retains_specific_python_version_limitation() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-python-specific-limit-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("app.py"),
        "message = t\"private-secret-content\"\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .env("PATH", &root)
        .args(["check", "python"])
        .arg(&root)
        .args(["--timeout", "30s"])
        .output()
        .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Python 3.14 template strings"), "{text}");
    assert!(text.contains("target-bound native confirmation"), "{text}");
    assert!(!text.contains("private-secret-content"), "{text}");
}
