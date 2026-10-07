use serde_json::{Value, json};
use std::{fs, process::Command};

#[test]
#[ignore = "requires explicit existing Clang via CODEGUARD_CLANG_BIN"]
fn failed_placeholder_rechecks_persist_without_reusing_old_positions_or_closing_tasks() {
    let tool = std::env::var("CODEGUARD_CLANG_BIN").unwrap();
    for (language, standard, extension) in [("c", "c11", "c"), ("cpp", "c++17", "cpp")] {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-placeholder-failure-{language}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join(format!("api.{extension}"));
        let initial = "/// TODO\n/// @param x TBD\n/// @return FIXME\nint f(int x) { return x; }\n";
        fs::write(&source, initial).unwrap();
        let invoke = |args: &[&str]| {
            let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(args)
                .arg("--format=json")
                .output()
                .unwrap();
            (
                out.status.code(),
                serde_json::from_slice::<Value>(&out.stdout).unwrap_or_else(|e| {
                    panic!(
                        "{args:?}: {e}; stderr={}",
                        String::from_utf8_lossy(&out.stderr)
                    )
                }),
            )
        };
        let path = root.to_str().unwrap();
        assert_eq!(invoke(&["init", path, "--apply"]).0, Some(3));
        let observed = invoke(&[
            "comments",
            language,
            source.to_str().unwrap(),
            "--clang-tool",
            &tool,
            "--standard",
            standard,
        ])
        .1;
        let id = observed["placeholder_workbench"]["task_ids"][0]
            .as_str()
            .unwrap();
        fs::write(&source, "int f(int x) { return ;\n").unwrap();
        let (exit, failed) = invoke(&["task", "verify", id, path]);
        assert_eq!(exit, Some(3));
        assert_eq!(failed["observation"], "incomplete");
        assert_eq!(failed["event_persisted"], true, "{failed}");
        assert_eq!(
            failed["native_scan"]["placeholder_observation_status"],
            "incomplete"
        );
        assert!(failed["native_scan"]["placeholders"].is_null());
        assert!(
            failed["native_scan"]["native"]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["level"] == "error")
        );
        let (_, show) = invoke(&["task", "show", id, path]);
        assert_eq!(show["task"]["reason_code"], "native_placeholder_incomplete");
        assert_eq!(show["task"]["allowed_paths"], json!([]));
        assert_eq!(show["task"]["placeholder_positions"], json!([]));
        let failed_path = root.join(format!(
            ".codeguard/reports/{}.json",
            failed["native_scan"]["run_id"].as_str().unwrap()
        ));
        let failed_bytes = fs::read(&failed_path).unwrap();
        let mut forged: Value = serde_json::from_slice(&failed_bytes).unwrap();
        forged["placeholder_observation_status"] = json!("observed");
        forged["placeholder_observation_reason"] = json!("clang_placeholder_observed");
        fs::write(&failed_path, serde_json::to_vec_pretty(&forged).unwrap()).unwrap();
        let rejected = invoke(&["task", "show", id, path]);
        assert_eq!(rejected.0, Some(3));
        assert!(rejected.1["task"].is_null());
        fs::write(&failed_path, failed_bytes).unwrap();
        fs::write(&source, initial).unwrap();
        let (exit, timed) = invoke(&["task", "verify", id, path, "--timeout", "1ms"]);
        assert_eq!(exit, Some(3));
        assert_eq!(timed["observation"], "incomplete");
        assert_eq!(timed["event_persisted"], true, "{timed}");
        assert_eq!(timed["native_scan"]["native"]["status"], "incomplete");
        let (_, recovered) = invoke(&["task", "verify", id, path]);
        assert_eq!(recovered["observation"], "still_present");
        assert_eq!(recovered["event_persisted"], true);
        for report in [&failed, &timed] {
            let run = report["native_scan"]["run_id"].as_str().unwrap();
            assert!(
                root.join(format!(".codeguard/reports/{run}.json"))
                    .is_file()
            );
            assert!(
                root.join(format!(".codeguard/state/consumed/{run}.json"))
                    .is_file()
            );
            assert!(
                root.join(format!(".codeguard/findings/{id}/events/verify-{run}.json"))
                    .is_file()
            );
        }
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        if let Ok(prefix) = std::env::var("CODEGUARD_PLACEHOLDER_FAILURE_EVIDENCE") {
            assert!(std::path::Path::new(&prefix).is_absolute());
            fs::write(
                format!("{prefix}.{language}.json"),
                serde_json::to_vec_pretty(
                    &json!({"failed":failed,"show":show,"timed":timed,"recovered":recovered}),
                )
                .unwrap(),
            )
            .unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}
