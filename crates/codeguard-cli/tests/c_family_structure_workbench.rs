use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires installed fixed Apple Clang21; set CODEGUARD_CLANG_BIN"]
fn structural_deficits_share_stable_file_tasks_and_original_rescan_does_not_close_them() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("existing tool"));
    let mut evidence = Vec::new();
    for (language, standard, ext) in [("c", "c11", "c"), ("cpp", "c++17", "cpp")] {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-structure-work-{language}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let _fixture = Fixture(root.clone());
        let source = root.join(format!("api.{ext}"));
        let initial = if language == "c" {
            "int f(int x) { return x; }\nint g(int x) { return x+1; }\n"
        } else {
            "int f(int x) { return x; }\nint f(double x) { return (int)x; }\n"
        };
        fs::write(&source, initial).unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        let scan = || {
            let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["comments", language])
                .arg(&source)
                .arg("--clang-tool")
                .arg(&tool)
                .args(["--standard", standard, "--format=json"])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(3));
            serde_json::from_slice::<Value>(&output.stdout).unwrap()
        };
        let first = scan();
        let ids = first["structural_workbench"]["task_ids"]
            .as_array()
            .expect("structural stable tasks");
        assert_eq!(ids.len(), 1);
        let id = ids[0].as_str().unwrap().to_owned();
        assert_eq!(
            first["structural_next"]["repair_brief"]["structural_positions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            first["structural_next"]["repair_brief"]["rule_source"],
            "codeguard_structural_policy"
        );
        assert_eq!(
            first["structural_next"]["repair_brief"]["allowed_paths"],
            json!([format!("api.{ext}")])
        );
        let second = scan();
        assert_eq!(second["structural_workbench"]["task_ids"], json!([id]));
        let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify"])
            .arg(&id)
            .arg(&root)
            .arg("--clang-tool")
            .arg(&tool)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(verify.status.code(), Some(3));
        assert!(
            String::from_utf8_lossy(&verify.stdout)
                .contains("clang_structure_task_verify_not_integrated")
        );
        let attempt = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "attempt", "start"])
            .arg(&id)
            .arg(&root)
            .args([
                "--owner",
                "agent",
                "--lease-token",
                &"a".repeat(64),
                "--action-id",
                "repair-source",
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(attempt.status.code(), Some(3));
        assert!(
            String::from_utf8_lossy(&attempt.stdout)
                .contains("clang_structure_attempt_journal_not_integrated")
        );
        fs::write(&source, format!("\n\n{initial}")).unwrap();
        let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("task")
            .arg("show")
            .arg(&id)
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(next.status.code(), Some(0));
        let stale: Value = serde_json::from_slice(&next.stdout).unwrap();
        assert_eq!(stale["task"]["allowed_paths"], json!([]));
        let moved = scan();
        assert_eq!(moved["structural_workbench"]["task_ids"], json!([id]));
        assert_eq!(
            moved["structural_next"]["repair_brief"]["structural_positions"][0]["line"],
            3
        );
        let text = fs::read_to_string(root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        for heading in [
            "问题证据",
            "规则依据",
            "允许修改",
            "修复步骤",
            "复检",
            "历史尝试",
            "关闭条件",
        ] {
            assert!(text.contains(heading), "{heading}");
        }
        fs::remove_file(root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        let restored = scan();
        assert!(root.join(format!(".codeguard/tasks/{id}.md")).exists());
        assert_eq!(restored["structural_workbench"]["task_ids"], json!([id]));
        assert!(
            fs::read_to_string(root.join(format!(".codeguard/tasks/{id}.md")))
                .unwrap()
                .contains("codeguard.documentation.function_structure_required")
        );
        let fixed = if language == "c" {
            "/** Read value.\n * @param x input value\n * @return same value\n */\nint f(int x) { return x; }\n/** Increment.\n * @param x input value\n * @return incremented value\n */\nint g(int x) { return x+1; }\n"
        } else {
            "/** Read integer.\n * @param x input value\n * @return same value\n */\nint f(int x) { return x; }\n/** Convert value.\n * @param x floating input\n * @return converted integer\n */\nint f(double x) { return (int)x; }\n"
        };
        fs::write(&source, fixed).unwrap();
        let clean = scan();
        assert_eq!(clean["structural_workbench"]["task_ids"], json!([]));
        assert_eq!(
            clean["structural_next"]["repair_brief"]["observation_status"],
            "candidate_absent_unverified_policy"
        );
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        let origin = root.join(format!(
            ".codeguard/reports/{}.json",
            first["structural_workbench"]["run_id"].as_str().unwrap()
        ));
        let bytes = fs::read(&origin).unwrap();
        let mut altered = bytes.clone();
        altered.push(b'\n');
        fs::write(&origin, altered).unwrap();
        let shown = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "show"])
            .arg(&id)
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(shown.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&shown.stdout).contains("clang_structure_origin_changed"));
        fs::write(origin, bytes).unwrap();
        let packets = fs::read_dir(root.join(".codeguard/reports"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("clangdocstruct-"))
            })
            .map(|p| serde_json::from_slice::<Value>(&fs::read(p).unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(packets.len(), 5);
        evidence.push(json!({"language":language,"task_id":id,"first":first,"repeated":second,"stale":stale,"moved":moved,"restored":restored,"clean":clean,"packets":packets}));
    }
    if let Some(path) = std::env::var_os("CODEGUARD_STRUCTURE_WORKBENCH_EVIDENCE") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"development_native_structure_workbench","qualification":"not_granted","test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_structure_workbench.rs"))),"codeguard_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),"cases":evidence})).unwrap()).unwrap();
    }
}
