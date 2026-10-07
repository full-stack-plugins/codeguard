use serde_json::Value;
use std::process::Command;

fn query(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn all_registered_languages_keep_four_blocked_core_obligations() {
    let out = query(&["capabilities", "--acceptance-plan", "--format=json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["report_type"], "production_acceptance_plan_view");
    assert_eq!(report["full_requirement_count"], 228);
    assert_eq!(report["selected_language_count"], 57);
    assert!(
        ["not_granted", "v1_granted"].contains(&report["qualification"].as_str().unwrap_or("")),
        "顶层 qualification 应为 not_granted 或 v1_granted"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["plan"]["platform_targets"].as_array().unwrap().len(),
        5
    );
    for row in report["plan"]["languages"].as_array().unwrap() {
        assert_eq!(row["capabilities"].as_object().unwrap().len(), 4);
        assert!(
            ["unqualified", "v1_qualified"].contains(&row["version_scope"]["qualification"].as_str().unwrap_or("")),
            "version_scope qualification 应为 unqualified 或 v1_qualified"
        );
        for cell in row["capabilities"].as_object().unwrap().values() {
            assert!(
                ["blocked", "v1_qualified"].contains(&cell["qualification"].as_str().unwrap_or("")),
                "capability qualification 应为 blocked 或 v1_qualified"
            );
            assert!(!cell["task_refs"].as_array().unwrap().is_empty());
            assert_eq!(
                cell["build_paths"].as_array().unwrap().len(),
                row["build_targets"].as_array().unwrap().len()
            );
        }
    }
    if let Ok(path) = std::env::var("CODEGUARD_TEST_ACCEPTANCE_PLAN_EVIDENCE") {
        std::fs::write(path, out.stdout).unwrap();
    }
}

#[test]
fn filtered_java_plan_preserves_gradle_cve_gap_and_full_registry_obligations() {
    let out = query(&["capabilities", "java", "--acceptance-plan", "--format=json"]);
    assert!(out.status.success());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["full_requirement_count"], 228);
    assert_eq!(report["selected_language_count"], 1);
    assert_eq!(report["selection"], "java");
    let java = &report["plan"]["languages"][0];
    let paths = java["capabilities"]["vulnerabilities"]["build_paths"]
        .as_array()
        .unwrap();
    assert_eq!(paths.len(), 2);
    assert!(
        paths
            .iter()
            .any(|path| path["ecosystem"] == "maven" && path["implementation_status"] == "partial")
    );
    assert!(paths.iter().any(|path| path["ecosystem"] == "gradle"
        && path["implementation_status"] == "partial"));
}

#[test]
fn legacy_inventory_stays_separate_and_ambiguous_plan_arguments_are_rejected() {
    let out = query(&["capabilities", "--format=json"]);
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["report_type"], "capability_inventory");
    for args in [
        vec!["--acceptance-plan", "--category=cve"],
        vec!["--acceptance-plan", "--acceptance-plan"],
        vec!["--acceptance-plan", "unknown"],
        vec!["--acceptance-plan", "java", "python"],
    ] {
        let mut argv = vec!["capabilities"];
        argv.extend(args);
        assert_eq!(query(&argv).status.code(), Some(2));
    }
}

#[test]
fn partial_gradle_cve_never_grants_native_scan_or_repair_qualification() {
    let out = query(&["capabilities", "java", "--acceptance-plan", "--format=json"]);
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let cell = &report["plan"]["languages"][0]["capabilities"]["vulnerabilities"];
    assert!(
        ["blocked", "v1_qualified"].contains(&cell["qualification"].as_str().unwrap_or("")),
        "vulnerabilities qualification 应为 blocked 或 v1_qualified"
    );
    let gradle = cell["build_paths"]
        .as_array()
        .unwrap()
        .iter()
        .find(|path| path["ecosystem"] == "gradle")
        .unwrap();
    assert_eq!(gradle["implementation_status"], "partial");
    assert!(
        gradle["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str().unwrap().contains("真实OWASP"))
    );
}

#[test]
fn cpp_standalone_replay_is_traceable_without_project_qualification() {
    let out = query(&["capabilities", "cpp", "--acceptance-plan", "--format=json"]);
    assert!(out.status.success());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let plan = &report["plan"];
    let capability = &plan["languages"][0]["capabilities"]["syntax"];
    assert!(
        ["blocked", "v1_qualified"].contains(&capability["qualification"].as_str().unwrap_or("")),
        "syntax qualification 应为 blocked 或 v1_qualified"
    );
    let evidence = "tests/acceptance/evidence/cpp17-native-wasm-differential.json";
    for path in capability["build_paths"].as_array().unwrap() {
        let included = path["evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry == evidence);
        assert_eq!(included, path["ecosystem"] == "standalone");
    }
    assert!(plan["source_hashes"][evidence].is_string());
}

#[test]
fn standalone_clang_evidence_does_not_imply_build_ecosystem_integration() {
    for language in ["c", "cpp"] {
        let out = query(&[
            "capabilities",
            language,
            "--acceptance-plan",
            "--format=json",
        ]);
        assert!(out.status.success());
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        for path in report["plan"]["languages"][0]["capabilities"]["syntax"]["build_paths"]
            .as_array()
            .unwrap()
        {
            if path["ecosystem"] != "standalone" {
                assert_eq!(
                    path["implementation_status"], "not_integrated",
                    "{language}: {path}"
                );
            }
        }
    }
}
