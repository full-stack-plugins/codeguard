use std::{fs, process::Command};

#[test]
fn aliases_select_canonical_languages_in_read_only_plans() {
    let root = std::env::temp_dir().join(format!("cg-language-alias-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (alias, canonical) in [
        ("py", "python"),
        ("rs", "rust"),
        ("ts", "typescript"),
        ("rb", "ruby"),
        ("kt", "kotlin"),
        ("erl", "erlang"),
        ("golang", "go"),
        ("c++", "cpp"),
        ("c#", "csharp"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["plan", "lint", alias])
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{alias}: {out:?}");
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["selection"]["language"], canonical);
    }
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn aliases_reach_the_same_native_entry_without_running_missing_tools() {
    let root = std::env::temp_dir().join(format!("cg-native-alias-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (category, alias, canonical) in [
        ("lint", "py", "python"),
        ("check", "py", "python"),
        ("cve", "py", "python"),
        ("comments", "rs", "rust"),
        ("build", "rs", "rust"),
        ("lint", "ts", "typescript"),
    ] {
        let mut reports = Vec::new();
        for language in [alias, canonical] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
            command
                .args([category, language])
                .arg(&root)
                .arg("--format=json")
                .env("PATH", &root);
            if category == "cve" {
                command
                    .arg("--pip-audit-tool")
                    .arg(root.join("missing-pip-audit"))
                    .args(["--pip-audit-version", "2.10.0"]);
            }
            let out = command.output().unwrap();
            assert_eq!(out.status.code(), Some(3), "{category} {language}: {out:?}");
            let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            reports.push(report);
        }
        assert_eq!(reports[0]["report_type"], reports[1]["report_type"]);
        assert_eq!(reports[0]["schema_version"], reports[1]["schema_version"]);
        assert_eq!(
            reports[0]["delivery_decision"],
            reports[1]["delivery_decision"]
        );
    }
    fs::remove_dir_all(root).unwrap();
}
