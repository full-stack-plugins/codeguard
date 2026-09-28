//! 同轮ESLint原生有效规则观察；只接收显式调用刚产生的冻结输入，不授予覆盖批准。
use crate::doctor_scratch::DoctorScratch;
use codeguard_adapters::{EslintCommand, parse_eslint_effective_rule};
use codeguard_runtime::{
    ProcessSpec, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) fn unavailable(rule: Option<&str>) -> Value {
    json!({"rule_id":rule,"status":"incomplete","reason":"eslint_effective_context_missing","severity":null,"input_stable":false,"stdout_sha256":null})
}
pub(crate) fn observe(rule: &str, scan: &Value, deadline: Instant) -> Value {
    let mut observation = unavailable(Some(rule));
    if let Some(reason) = interrupted(deadline) {
        observation["reason"] = json!(reason);
        return observation;
    }
    if !inputs_current(scan) {
        observation["reason"] = json!("eslint_effective_input_changed");
        return observation;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |v| v.as_nanos());
    let id = format!("eslint-settings-{}-{nanos}", std::process::id());
    let Some(scratch) = DoctorScratch::create(&id) else {
        observation["reason"] = json!("eslint_effective_workspace_unavailable");
        return observation;
    };
    let path = |key: &str| PathBuf::from(scan["inputs"][key]["path"].as_str().unwrap_or(""));
    let command = EslintCommand {
        node: path("node"),
        entry: path("eslint"),
        config: path("config"),
        sources: vec![path("source")],
        report: scratch.path().join("unused.json"),
        max_warnings: None,
    };
    let Ok(args) = command.effective_config_args() else {
        observation["reason"] = json!("eslint_effective_command_invalid");
        return observation;
    };
    let cwd = PathBuf::from(scan["cwd"].as_str().unwrap_or(""));
    if !cwd.is_absolute() || !cwd.is_dir() || cwd.canonicalize().ok().as_ref() != Some(&cwd) {
        observation["reason"] = json!("eslint_effective_context_missing");
        return observation;
    }
    let spec = ProcessSpec {
        executable: command.node,
        args,
        cwd,
        env: BTreeMap::new(),
        stdin: None,
        deadline,
        output_limit_bytes: 1024 * 1024,
    };
    let outcome = run_process_recorded(
        &spec,
        &AtomicBool::new(false),
        scratch.path(),
        "effective.log",
    );
    if let Some(reason) = interrupted(deadline) {
        observation["reason"] = json!(reason);
        return observation;
    }
    let stable = inputs_current(scan);
    observation["input_stable"] = json!(stable);
    if !stable {
        observation["reason"] = json!("eslint_effective_input_changed");
        return observation;
    }
    let Ok(outcome) = outcome else {
        observation["reason"] = json!("eslint_effective_evidence_failed");
        return observation;
    };
    if matches!(
        outcome.termination,
        Termination::Cancelled | Termination::TimedOut | Termination::DeadlineBeforeStart
    ) {
        observation["input_stable"] = json!(false);
        observation["reason"] = json!(if outcome.termination == Termination::Cancelled {
            "request_cancelled"
        } else {
            "request_deadline_exceeded"
        });
        return observation;
    }
    observation["stdout_sha256"] = json!(format!("{:x}", Sha256::digest(&outcome.stdout)));
    if outcome.termination != Termination::Exited(0) || !outcome.stderr.is_empty() {
        observation["reason"] = json!("eslint_effective_execution_incomplete");
        return observation;
    }
    match parse_eslint_effective_rule(&outcome.stdout, rule) {
        Ok(severity) => {
            observation["status"] = json!("observed");
            observation["severity"] = json!(severity);
            observation["reason"] = json!(if severity.is_none() {
                "eslint_effective_rule_not_present"
            } else {
                "eslint_effective_rule_observed"
            });
        }
        Err(reason) => observation["reason"] = json!(reason),
    }
    observation
}
fn interrupted(deadline: Instant) -> Option<&'static str> {
    if codeguard_runtime::sigint_cancellation_requested() {
        Some("request_cancelled")
    } else if Instant::now() >= deadline {
        Some("request_deadline_exceeded")
    } else {
        None
    }
}
fn inputs_current(scan: &Value) -> bool {
    [
        ("source", 16 * 1024 * 1024),
        ("config", 1024 * 1024),
        ("node", 128 * 1024 * 1024),
        ("eslint", 16 * 1024 * 1024),
    ]
    .iter()
    .all(|(key, limit)| {
        let input = &scan["inputs"][*key];
        input["path"].as_str().is_some_and(|p| {
            let path = Path::new(p);
            path.is_absolute()
                && path.canonicalize().ok().as_deref() == Some(path)
                && read_bounded_regular_file(path, *limit)
                    .ok()
                    .is_some_and(|bytes| input["sha256"] == format!("{:x}", Sha256::digest(bytes)))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::observe;
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::{
        path::PathBuf,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    #[test]
    #[ignore = "requires explicit existing Node and ESLint 10.11.0; native effective configuration"]
    fn native_print_config_observes_enabled_off_and_missing_rules() {
        use std::os::unix::fs::DirBuilderExt;
        let node = PathBuf::from(std::env::var_os("CODEGUARD_NODE_BIN").unwrap())
            .canonicalize()
            .unwrap();
        let entry = PathBuf::from(std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap())
            .canonicalize()
            .unwrap();
        let version = std::process::Command::new(&node)
            .args(["--", entry.to_str().unwrap(), "--version"])
            .output()
            .unwrap();
        assert!(version.status.success());
        assert_eq!(version.stdout, b"v10.11.0\n");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-eslint-effective-{}-{nanos}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        let source = root.join("app.js");
        std::fs::write(&source, "debugger;").unwrap();
        let config = root.join("eslint.config.cjs");
        for (body, severity) in [
            ("module.exports=[{rules:{'no-debugger':'error'}}]", Some(2)),
            ("module.exports=[{rules:{'no-debugger':'off'}}]", Some(0)),
            ("module.exports=[{rules:{}}]", None),
        ] {
            std::fs::write(&config, body).unwrap();
            let mut inputs = json!({});
            for (key, path) in [
                ("node", &node),
                ("eslint", &entry),
                ("source", &source),
                ("config", &config),
            ] {
                inputs[key] = json!({"path":path,"sha256":format!("{:x}",Sha256::digest(std::fs::read(path).unwrap()))});
            }
            let scan = json!({"inputs":inputs,"cwd":root});
            let result = observe(
                "no-debugger",
                &scan,
                Instant::now() + Duration::from_secs(90),
            );
            assert_eq!(result["status"], "observed", "{result}");
            assert_eq!(result["severity"], json!(severity), "{result}");
            assert_eq!(result["input_stable"], true);
            assert_eq!(result["stdout_sha256"].as_str().unwrap().len(), 64);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
