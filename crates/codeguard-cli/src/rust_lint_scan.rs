//! Rust 项目的局部原生 Clippy 观察；默认 features 与 all-targets 不代表全部构建组合。

use codeguard_adapters::parse_cargo_clippy_json;
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::workspace_refresh::read_workspace_baseline;

static NEXT_RUN: AtomicU64 = AtomicU64::new(0);
static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 调用明确选择的 Cargo 原生 Clippy；返回局部诊断，不授予规则包或交付权威。
pub fn observe_cargo_clippy(
    root: &Path,
    source_files: &BTreeSet<String>,
    cargo_tool: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    observe_cargo_clippy_inner(root, source_files, cargo_tool, deadline, cancelled, None)
}

/// 使用原生 `--force-warn` 对单条 Clippy 规则作抑制对照；结果只供本地复检裁定。
pub fn observe_cargo_clippy_force_warn(
    root: &Path,
    source_files: &BTreeSet<String>,
    cargo_tool: Option<&Path>,
    rule_id: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    observe_cargo_clippy_inner(
        root,
        source_files,
        cargo_tool,
        deadline,
        cancelled,
        Some(rule_id),
    )
}

fn observe_cargo_clippy_inner(
    root: &Path,
    source_files: &BTreeSet<String>,
    cargo_tool: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
    force_warn_rule: Option<&str>,
) -> Value {
    let (workspace_binding, workspace_id) = match read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => ("bound", json!(id)),
            None => ("legacy_unbound", Value::Null),
        },
        Ok(None) => ("uninitialized", Value::Null),
        Err(_) => ("invalid", Value::Null),
    };
    let mut report = json!({
        "schema_version":"0.2.0", "report_type":"rust_clippy_local_observation",
        "operation":"lint", "language":"rust", "command_status":"incomplete",
        "delivery_decision":"not_evaluated", "exit_code":3,
        "workspace_binding":workspace_binding, "workspace_id":workspace_id,
        "run_id":run_id(), "tool_approval":"unverified", "rulepack_approval":"unverified",
        "checker_id":"rust.cargo_clippy", "configuration":"unknown", "configuration_ref":null,
        "local_scan_complete":false, "reason":"cargo_tool_not_selected",
        "tool_sha256":null, "manifest_sha256":null, "findings":[], "scope":"default_features_all_targets",
        "coverage_proven":false, "authority":"local_unverified",
        "recheck_command":"cargo clippy --locked --offline --all-targets --message-format=json"
    });
    if let Some(rule) = force_warn_rule {
        if !rule.strip_prefix("clippy::").is_some_and(|name| {
            !name.is_empty()
                && name.len() <= 72
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        }) {
            report["reason"] = json!("force_warn_rule_invalid");
            return report;
        }
        report["recheck_command"] = json!(format!(
            "cargo clippy --locked --offline --all-targets --message-format=json -- --force-warn {rule}"
        ));
    }
    let manifest = root.join("Cargo.toml");
    let Ok(before_manifest) = read_bounded_regular_file(&manifest, 256 * 1024) else {
        report["reason"] = json!("root_cargo_manifest_unavailable");
        return report;
    };
    report["manifest_sha256"] = json!(format!("{:x}", Sha256::digest(&before_manifest)));
    for name in ["clippy.toml", ".clippy.toml"] {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                if read_bounded_regular_file(&path, 256 * 1024).is_err() {
                    report["reason"] = json!("clippy_config_unreadable");
                    return report;
                }
                report["configuration"] = json!("configured");
                report["configuration_ref"] = json!(name);
                break;
            }
            Ok(_) => {
                report["reason"] = json!("clippy_config_not_regular");
                return report;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                report["reason"] = json!("clippy_config_unreadable");
                return report;
            }
        }
    }
    let Some(tool) = cargo_tool else {
        return report;
    };
    let Ok(resolved_tool) = tool.canonicalize() else {
        report["reason"] = json!("cargo_tool_unavailable");
        return report;
    };
    let Ok(tool_bytes) = read_bounded_regular_file(&resolved_tool, 128 * 1024 * 1024) else {
        report["reason"] = json!("cargo_tool_unavailable");
        return report;
    };
    report["tool_sha256"] = json!(format!("{:x}", Sha256::digest(&tool_bytes)));
    let Ok(inputs) = crate::rust_lint_inputs::RustLintInputs::capture(root, source_files) else {
        report["reason"] = json!("rust_inputs_unavailable");
        return report;
    };
    if inputs.source("Cargo.toml") != Some(before_manifest.as_slice()) {
        report["reason"] = json!("manifest_changed_during_scan");
        return report;
    }
    if inputs.source("Cargo.lock").is_none() {
        // 检查不得隐式生成锁文件；项目需先明确准备依赖，再按原锁复检。
        report["reason"] = json!("cargo_lock_unavailable");
        return report;
    }
    let Some(scratch) = private_scratch() else {
        report["reason"] = json!("private_workspace_unavailable");
        return report;
    };
    let mut environment = BTreeMap::new();
    for name in ["PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME"] {
        if let Some(value) = std::env::var_os(name) {
            environment.insert(OsString::from(name), value);
        }
    }
    environment.insert(
        OsString::from("CARGO_TARGET_DIR"),
        scratch.0.as_os_str().to_os_string(),
    );
    environment.insert(OsString::from("CARGO_NET_OFFLINE"), OsString::from("true"));
    let mut args: Vec<OsString> = [
        "clippy",
        "--locked",
        "--offline",
        "--all-targets",
        "--message-format=json",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    if let Some(rule) = force_warn_rule {
        args.extend([
            OsString::from("--"),
            OsString::from("--force-warn"),
            OsString::from(rule),
        ]);
    }
    let outcome = run_process(
        &ProcessSpec {
            executable: tool.to_path_buf(),
            args,
            cwd: root.to_path_buf(),
            env: environment,
            stdin: None,
            deadline,
            output_limit_bytes: 16 * 1024 * 1024,
        },
        cancelled,
    );
    let parsed = parse_cargo_clippy_json(&outcome.stdout);
    let mut finding_paths_valid = true;
    let mut source_identity_valid = true;
    let mut occurrences = BTreeMap::<String, u64>::new();
    let mut native_findings = parsed.findings.clone();
    native_findings.sort_by(|left, right| {
        (&left.path, left.line, left.column, &left.rule_id).cmp(&(
            &right.path,
            right.line,
            right.column,
            &right.rule_id,
        ))
    });
    let findings: Vec<Value> = native_findings
        .iter()
        .filter_map(|finding| {
            if !source_files.contains(&finding.path) {
                finding_paths_valid = false;
                return None;
            }
            let Some(bytes) = inputs.source(&finding.path) else {
                source_identity_valid = false;
                return None;
            };
            let Some(anchor) = bytes
                .split(|byte| *byte == b'\n')
                .nth(finding.line.saturating_sub(1) as usize)
                .map(|line| line.trim_ascii())
                .filter(|line| !line.is_empty())
            else {
                source_identity_valid = false;
                return None;
            };
            let mut hash = Sha256::new();
            for part in [
                b"codeguard-clippy-finding-v1".as_slice(),
                finding.path.as_bytes(),
                finding.rule_id.as_bytes(),
                anchor,
            ] {
                hash.update((part.len() as u64).to_be_bytes());
                hash.update(part);
            }
            let base = format!("{:x}", hash.finalize());
            let ordinal = occurrences.entry(base.clone()).or_default();
            let mut final_hash = Sha256::new();
            final_hash.update(base.as_bytes());
            final_hash.update(ordinal.to_be_bytes());
            *ordinal += 1;
            let fingerprint = format!("{:x}", final_hash.finalize());
            Some(json!({
                "finding_id":format!("CG-{}", &fingerprint[..32]),
                "finding_fingerprint":fingerprint,
                "source_sha256":format!("{:x}", Sha256::digest(bytes)),
                "rule_id":finding.rule_id,
                "path":finding.path,
                "line":finding.line,
                "column":finding.column,
                "level":finding.level
            }))
        })
        .collect();
    report["findings"] = json!(findings);
    let manifest_unchanged = read_bounded_regular_file(&manifest, 256 * 1024)
        .is_ok_and(|after| after == before_manifest);
    let inputs_unchanged = inputs.unchanged(root);
    let tool_unchanged = tool
        .canonicalize()
        .is_ok_and(|after| after == resolved_tool)
        && read_bounded_regular_file(&resolved_tool, 128 * 1024 * 1024)
            .is_ok_and(|after| after == tool_bytes);
    if !inputs_unchanged || !tool_unchanged {
        // 陈旧诊断不能绑定到新源码，也不能产生新的源码修复任务；局部阻塞仍保留。
        report["findings"] = json!([]);
    }
    let reason = if outcome.termination == Termination::Cancelled {
        "request_cancelled"
    } else if !manifest_unchanged {
        "manifest_changed_during_scan"
    } else if !inputs_unchanged {
        "rust_inputs_changed_during_scan"
    } else if !tool_unchanged {
        "cargo_tool_changed_during_scan"
    } else if !finding_paths_valid {
        "native_finding_outside_observed_scope"
    } else if !source_identity_valid {
        "source_identity_unavailable"
    } else if matches!(outcome.termination, Termination::Exited(_)) {
        parsed
            .issue
            .unwrap_or(if outcome.termination == Termination::Exited(0) {
                "native_observed_unverified"
            } else {
                "native_process_failed"
            })
    } else {
        termination_reason(outcome.termination)
    };
    report["reason"] = json!(reason);
    report["local_scan_complete"] = json!(reason == "native_observed_unverified");
    report
}

fn run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "rust-lint-{}-{nanos}-{}",
        std::process::id(),
        NEXT_RUN.fetch_add(1, Ordering::Relaxed)
    )
}

fn termination_reason(termination: Termination) -> &'static str {
    match termination {
        Termination::TimedOut | Termination::DeadlineBeforeStart => "request_deadline_exceeded",
        Termination::Cancelled => "request_cancelled",
        Termination::OutputLimit => "native_output_limit_exceeded",
        Termination::Exited(_) | Termination::Signaled => "native_process_failed",
        _ => "native_execution_failed",
    }
}

fn private_scratch() -> Option<Scratch> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    private_scratch_with_nonce(nonce)
}

fn private_scratch_with_nonce(nonce: u128) -> Option<Scratch> {
    let path = std::env::temp_dir().join(format!(
        "codeguard-rust-clippy-{}-{nonce}-{}",
        std::process::id(),
        NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&path).ok()?;
    Some(Scratch(path))
}

#[cfg(test)]
mod scratch_contract {
    #[test]
    fn identical_clock_values_do_not_collide_between_native_tasks() {
        let first = super::private_scratch_with_nonce(0).unwrap();
        let second =
            super::private_scratch_with_nonce(0).expect("同进程相同时间戳必须仍分配独立私有目录");
        assert_ne!(first.0, second.0);
    }
}
