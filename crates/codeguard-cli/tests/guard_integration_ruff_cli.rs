#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Case {
    root: PathBuf,
}
impl Case {
    fn new(name: &str, mode: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-ruff-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/guard-integration/ruff-f401");
        let manifest: Value =
            serde_json::from_slice(&fs::read(fixtures.join("capture.json")).unwrap()).unwrap();
        let native = fs::read(fixtures.join(format!("{name}.json"))).unwrap();
        let doc: Value = serde_json::from_slice(&native).unwrap();
        let argv = manifest["cases"][name]["argv"].as_array().unwrap();
        let case = Self { root };
        case.put("invocation",json!({"version":"codeguard.ruff-invocation/v1alpha1","packageVersion":"0.1.4","nativeSchema":"0.13.0","executable":argv[0],"argv":&argv[1..],"runId":doc["run_id"],"processExit":3,"producerSha256":manifest["codeguardSha256"]}));
        case.put("context",json!({"version":"codeguard.ruff-context/v1alpha1","runId":doc["run_id"],"binding":{"repoId":"repo","taskId":"task","worktreeId":"worktree","requirementIds":["ruff-f401"],"candidateOid":"a".repeat(40),"baseOid":"b".repeat(40),"mergeGroupId":null,"baselineDigest":null,"sourceSnapshotDigest":format!("sha256:{}",manifest["cases"][name]["sourceSha256"].as_str().unwrap())},"target":"app.py","producerSha256":manifest["codeguardSha256"],"startedAt":"2026-10-09T00:00:00Z","finishedAt":"2026-10-09T00:01:00Z"}));
        case.put("mapping",json!({"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"f401"},{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}));
        case.put("contract",json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"ruff-f401","revision":"1"},"spec":{"rules":[{"id":"f401","enforcement":mode,"assertion":{"type":"forbid_relation","subject":"python","predicate":"has","object":"unused-import"}}]}}));
        fs::write(case.path("native"), native).unwrap();
        case
    }
    fn path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.json"))
    }
    fn put(&self, name: &str, value: Value) {
        fs::write(self.path(name), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn get(&self, name: &str) -> Value {
        serde_json::from_slice(&fs::read(self.path(name)).unwrap()).unwrap()
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command.arg("guard-project-ruff");
        for (flag, file) in [
            ("--invocation", "invocation"),
            ("--native-report", "native"),
            ("--context", "context"),
            ("--mapping", "mapping"),
            ("--contract", "contract"),
        ] {
            command.arg(flag).arg(self.path(file));
        }
        command
    }
    fn run(&self) -> Output {
        self.command().output().unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn bundle(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn actual_cli_common_zero_two_three_preserve_captured_native_three() {
    for (name, mode, exit, decision) in [
        ("clean", "enforce", 0, "ALLOW"),
        ("fixed", "enforce", 0, "ALLOW"),
        ("bad", "enforce", 2, "BLOCK"),
        ("bad", "review", 3, "REQUIRE_APPROVAL"),
        ("bad", "advise", 0, "ALLOW"),
        ("noqa", "review", 2, "BLOCK"),
        ("missing-tool", "advise", 2, "BLOCK"),
    ] {
        let case = Case::new(name, mode);
        let out = case.run();
        assert_eq!(
            out.status.code(),
            Some(exit),
            "{name}/{mode}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stderr.is_empty());
        let bundle = bundle(&out);
        assert_eq!(bundle["nativeExit"], 3);
        assert_eq!(bundle["envelope"]["decision"], decision);
        assert_eq!(
            bundle["artifacts"]["domain"].as_str().unwrap().as_bytes(),
            fs::read(case.path("native")).unwrap()
        );
        let envelope = serde_json::from_value(bundle["envelope"].clone()).unwrap();
        guardengine::integration::verify_engine_artifacts(
            &envelope,
            bundle["artifacts"]["contract"].as_str().unwrap().as_bytes(),
            bundle["artifacts"]["facts"].as_str().unwrap().as_bytes(),
            bundle["artifacts"]["report"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
    }
}
#[test]
fn prebinding_conflict_has_no_envelope_and_structured_stderr_four() {
    let case = Case::new("clean", "enforce");
    let mut inv = case.get("invocation");
    inv["producerSha256"] = "e".repeat(64).into();
    case.put("invocation", inv);
    let out = case.run();
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let diagnostic: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostic["phase"], "unbound");
}
#[test]
fn missing_or_malformed_native_after_binding_is_error_null_four() {
    for missing in [true, false] {
        let case = Case::new("clean", "enforce");
        if missing {
            fs::remove_file(case.path("native")).unwrap();
        } else {
            fs::write(case.path("native"), b"{broken").unwrap();
        }
        let out = case.run();
        assert_eq!(out.status.code(), Some(4));
        let result = bundle(&out);
        assert_eq!(result["envelope"]["runStatus"], "error");
        assert!(result["envelope"]["decision"].is_null());
        assert!(result["artifacts"]["report"].is_null());
        assert_eq!(
            serde_json::from_slice::<Value>(&out.stderr).unwrap()["phase"],
            "bound"
        );
    }
}
#[test]
fn captured_root_mismatch_is_bound_error_without_foreign_domain() {
    let case = Case::new("clean", "enforce");
    let mut inv = case.get("invocation");
    inv["argv"][2] = "/different-root".into();
    case.put("invocation", inv);
    let out = case.run();
    assert_eq!(out.status.code(), Some(4));
    let result = bundle(&out);
    assert!(result["envelope"]["decision"].is_null());
    assert!(result["artifacts"]["domain"].is_null());
}

#[test]
fn unsupported_inputs_are_unbound_and_private_paths_are_not_executed() {
    for mutation in 0..7 {
        let case = Case::new("clean", "enforce");
        match mutation {
            0 => {
                let mut v = case.get("context");
                v["qualified"] = true.into();
                case.put("context", v);
            }
            1 => {
                let mut v = case.get("invocation");
                v["version"] = "future".into();
                case.put("invocation", v);
            }
            2 => {
                let mut v = case.get("invocation");
                v["argv"][0] = "repair".into();
                case.put("invocation", v);
            }
            3 => {
                let mut v = case.get("invocation");
                v["processExit"] = 0.into();
                case.put("invocation", v);
            }
            4 => {
                let raw = fs::read_to_string(case.path("context")).unwrap();
                fs::write(
                    case.path("context"),
                    raw.replacen('{', "{\"target\":\"app.py\",", 1),
                )
                .unwrap();
            }
            5 => {
                fs::write(case.path("invocation"), vec![b' '; 16_385]).unwrap();
            }
            _ => {
                let original = case.path("context");
                let target = case.root.join("real-context.json");
                fs::rename(&original, &target).unwrap();
                std::os::unix::fs::symlink(&target, &original).unwrap();
            }
        }
        let out = case.run();
        assert_eq!(out.status.code(), Some(4), "{mutation}");
        assert!(out.stdout.is_empty(), "{mutation}");
        assert_eq!(
            serde_json::from_slice::<Value>(&out.stderr).unwrap()["phase"],
            "unbound"
        );
    }
}
#[test]
fn native_oversize_symlink_and_mismatched_run_are_bound_errors() {
    for mutation in 0..3 {
        let case = Case::new("clean", "enforce");
        match mutation {
            0 => fs::write(case.path("native"), vec![b' '; 1_048_577]).unwrap(),
            1 => {
                let original = case.path("native");
                let target = case.root.join("real-native.json");
                fs::rename(&original, &target).unwrap();
                std::os::unix::fs::symlink(&target, &original).unwrap();
            }
            _ => {
                let mut v = case.get("native");
                v["run_id"] = "foreign-run".into();
                case.put("native", v);
            }
        }
        let out = case.run();
        assert_eq!(out.status.code(), Some(4));
        let b = bundle(&out);
        assert!(b["envelope"]["decision"].is_null());
        assert!(b["artifacts"]["domain"].is_null());
    }
}
#[test]
fn help_adds_the_new_entry_and_old_guard_project_stays_distinct() {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["help", "guard-project-ruff"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("guard-project-ruff"));
    let case = Case::new("clean", "enforce");
    let old = case.command();
    // Old entry uses different context/schema and must not silently accept this new profile.
    let args: Vec<_> = old.get_args().skip(1).map(|s| s.to_owned()).collect();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("guard-project")
        .args(args)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
}
#[cfg(target_os = "linux")]
#[test]
fn stdout_write_failure_is_bound_diagnostic_four() {
    use std::process::Stdio;
    let case = Case::new("clean", "enforce");
    let output = case
        .command()
        .stdout(Stdio::from(
            fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .unwrap(),
        ))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    let d: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(d["phase"], "bound");
    assert_eq!(d["code"], "stdout_write_failed");
}
#[test]
fn broader_contract_and_wildcard_are_rejected_before_binding() {
    for broad in [true, false] {
        let case = Case::new("clean", "enforce");
        let mut c = case.get("contract");
        if broad {
            let mut extra = c["spec"]["rules"][0].clone();
            extra["id"] = "unrelated".into();
            c["spec"]["rules"].as_array_mut().unwrap().push(extra);
        } else {
            c["spec"]["rules"][0]["assertion"]["subject"] = "*".into();
        }
        case.put("contract", c);
        let out = case.run();
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&out.stderr).unwrap()["phase"],
            "unbound"
        );
    }
}
#[test]
fn bound_projection_budget_failure_preserves_valid_matching_native_bytes() {
    let case = Case::new("bad", "enforce");
    let mut native = case.get("native");
    let finding = native["files"][0]["findings"][0].clone();
    native["files"][0]["findings"] = Value::Array(
        (0..120)
            .map(|i| {
                let mut row = finding.clone();
                row["finding_id"] = format!("finding-{i}").into();
                row
            })
            .collect(),
    );
    case.put("native", native);
    let mut contract = case.get("contract");
    for field in ["subject", "predicate", "object"] {
        contract["spec"]["rules"][0]["assertion"][field] = "\\".repeat(30_000).into();
    }
    case.put("contract", contract);
    let output = case.run();
    assert_eq!(output.status.code(), Some(4));
    let result = bundle(&output);
    assert_eq!(result["envelope"]["runStatus"], "error");
    assert!(result["envelope"]["decision"].is_null());
    assert!(result["artifacts"]["report"].is_null());
    assert_eq!(
        result["artifacts"]["domain"].as_str().unwrap().as_bytes(),
        fs::read(case.path("native")).unwrap()
    );
}
