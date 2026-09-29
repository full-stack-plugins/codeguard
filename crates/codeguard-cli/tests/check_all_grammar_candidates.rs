#![cfg(all(feature = "wasm-precheck", unix))]

use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

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
        root.join("component.cfs"),
        "component { function f() { return 1; } }\n",
    )
    .unwrap();
    fs::write(
        root.join("page.cfm"),
        "<cfquery name=\"q\">SELECT #x# FROM users</cfquery>\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args(["--format=json", "--timeout", "90s"])
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
    assert_eq!(report["schema_version"], "0.33.0");
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
fn check_all_invokes_all_32_pinned_candidates_across_bounded_projects() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-check-all-grammars-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for (index, (name, source)) in [
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
    .into_iter()
    .enumerate()
    {
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
