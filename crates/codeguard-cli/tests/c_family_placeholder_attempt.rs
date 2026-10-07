use serde_json::{Value, json};
use std::{fs, process::Command};

#[test]
#[ignore = "requires explicit existing Clang via CODEGUARD_CLANG_BIN"]
fn placeholder_attempts_require_original_verification_and_preserve_failure_budget() {
    let tool = std::env::var("CODEGUARD_CLANG_BIN").unwrap();
    for (language, standard, extension) in [("c", "c11", "c"), ("cpp", "c++17", "cpp")] {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-placeholder-attempt-{language}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join(format!("api.{extension}"));
        fs::write(
            &source,
            "/// TODO\n/// @param x TBD\n/// @return FIXME\nint f(int x) { return x; }\n",
        )
        .unwrap();
        let invoke = |args: &[&str]| {
            let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(args)
                .arg("--format=json")
                .output()
                .unwrap();
            (
                out.status.code(),
                serde_json::from_slice::<Value>(&out.stdout).unwrap(),
            )
        };
        let path = root.to_str().unwrap();
        assert_eq!(invoke(&["init", path, "--apply"]).0, Some(3));
        let scan = || {
            invoke(&[
                "comments",
                language,
                source.to_str().unwrap(),
                "--clang-tool",
                &tool,
                "--standard",
                standard,
            ])
        };
        let (_, observed) = scan();
        let id = observed["placeholder_workbench"]["task_ids"][0]
            .as_str()
            .unwrap();
        let task = |words: &[&str], options: &[&str]| {
            let mut args = vec!["task"];
            args.extend_from_slice(words);
            args.extend([id, path]);
            args.extend_from_slice(options);
            invoke(&args)
        };
        let show = || task(&["show"], &[]).1["task"].clone();
        let initial_show = task(&["show"], &[]).1;
        let initial = initial_show["task"].clone();
        assert_eq!(initial_show["schema_version"], "0.9.0");
        assert_eq!(initial_show["next_actions"][0], initial["recheck_argv"]);
        assert_eq!(initial["disposition"], "actionable");
        assert_eq!(
            initial["allowed_paths"],
            json!([format!("api.{extension}")])
        );
        assert_eq!(initial["history"]["no_progress_count"], 0);
        let (exit, lease) = task(&["claim"], &["--owner", "placeholder-agent"]);
        assert_eq!(exit, Some(0), "{lease}");
        let token = lease["lease_token"].as_str().unwrap();
        let start_options = [
            "--owner",
            "placeholder-agent",
            "--lease-token",
            token,
            "--action-id",
            "repair-source",
        ];
        let mut reports = Vec::new();
        for count in 1..=2 {
            let (exit, start) = task(&["attempt", "start"], &start_options);
            assert_eq!(exit, Some(0), "{start}");
            assert_eq!(show()["disposition"], "waiting");
            let (exit, finish) = task(
                &["attempt", "finish"],
                &[
                    "--owner",
                    "placeholder-agent",
                    "--lease-token",
                    token,
                    "--attempt-id",
                    start["attempt_id"].as_str().unwrap(),
                    "--outcome",
                    "ready-to-verify",
                    "--note-code",
                    "source_edit",
                ],
            );
            assert_eq!(exit, Some(0), "{finish}");
            assert_eq!(show()["disposition"], "verification_required");
            let (exit, early) = task(&["attempt", "start"], &start_options);
            assert_eq!(exit, Some(3));
            assert_eq!(early["reason"], "verification_required_before_retry");
            let (exit, verified) = task(
                &["verify"],
                &["--owner", "placeholder-agent", "--lease-token", token],
            );
            assert_eq!(exit, Some(3));
            assert_eq!(verified["observation"], "still_present");
            assert_eq!(verified["event_persisted"], true);
            assert_eq!(show()["history"]["no_progress_count"], count);
            reports.push(verified);
        }
        assert_eq!(show()["reason_code"], "no_progress_budget_exhausted");
        assert_eq!(show()["allowed_paths"], json!([]));
        assert_eq!(
            task(&["attempt", "start"], &start_options).1["reason"],
            "no_progress_budget_exhausted"
        );
        scan();
        assert_eq!(show()["history"]["no_progress_count"], 2);
        fs::remove_file(root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        scan();
        assert_eq!(show()["history"]["no_progress_count"], 2);
        let report_path = root.join(format!(
            ".codeguard/reports/{}.json",
            reports[0]["native_scan"]["run_id"].as_str().unwrap()
        ));
        let bytes = fs::read(&report_path).unwrap();
        fs::remove_file(&report_path).unwrap();
        assert_eq!(
            show()["reason_code"],
            "historical_verification_evidence_unavailable"
        );
        assert_eq!(show()["allowed_paths"], json!([]));
        fs::write(&report_path, &bytes).unwrap();
        let mut forged: Value = serde_json::from_slice(&bytes).unwrap();
        forged["task_rule"] = json!("codeguard.documentation.function_structure_required");
        fs::write(&report_path, serde_json::to_vec_pretty(&forged).unwrap()).unwrap();
        assert!(show().is_null(), "篡改复检不能产生有效修复指引");
        fs::write(&report_path, bytes).unwrap();
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        if let Ok(prefix) = std::env::var("CODEGUARD_PLACEHOLDER_ATTEMPT_EVIDENCE") {
            assert!(std::path::Path::new(&prefix).is_absolute());
            fs::write(
                format!("{prefix}.{language}.json"),
                serde_json::to_vec_pretty(
                    &json!({"initial":initial,"initial_show":initial_show,"rechecks":reports,"exhausted":show(),"exhausted_show":task(&["show"], &[]).1}),
                )
                .unwrap(),
            )
            .unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}
