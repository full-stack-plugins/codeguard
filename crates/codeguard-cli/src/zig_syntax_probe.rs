//! Zig 原生 AST 探针供 lint 与任务复检共用；不授予完整 lint 或关闭权威。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

/// 用已提供工具检查指定原字节；参数含受控 cwd 与共享 deadline，返回有界原生观察。
pub(crate) fn observe(tool: &Path, source: &[u8], cwd: &Path, deadline: Instant) -> Option<Value> {
    if !tool.is_absolute() {
        return None;
    }
    let executable = tool.canonicalize().ok()?;
    if !executable.is_file() {
        return None;
    }
    let tool_sha256 = format!(
        "{:x}",
        Sha256::digest(read_bounded_regular_file(&executable, 64 * 1024 * 1024).ok()?)
    );
    let cancelled = AtomicBool::new(false);
    let version = run_process(
        &ProcessSpec {
            executable: executable.clone(),
            args: vec![OsString::from("version")],
            cwd: cwd.to_path_buf(),
            env: BTreeMap::new(),
            stdin: None,
            deadline,
            output_limit_bytes: 1024,
        },
        &cancelled,
    );
    if version.termination != Termination::Exited(0)
        || std::str::from_utf8(&version.stdout).ok()?.trim() != "0.16.0"
    {
        return Some(
            json!({"status":"incomplete","reason":"zig_version_unverified_or_unsupported","version":null,"tool_sha256":tool_sha256,"diagnostics":[]}),
        );
    }
    let outcome = run_process(
        &ProcessSpec {
            executable: executable.clone(),
            args: vec![
                OsString::from("ast-check"),
                OsString::from("--color"),
                OsString::from("off"),
            ],
            cwd: cwd.to_path_buf(),
            env: BTreeMap::new(),
            stdin: Some(source.to_vec()),
            deadline,
            output_limit_bytes: 64 * 1024,
        },
        &cancelled,
    );
    let current_tool_sha256 = read_bounded_regular_file(&executable, 64 * 1024 * 1024)
        .ok()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    if current_tool_sha256.as_deref() != Some(tool_sha256.as_str()) {
        return Some(
            json!({"status":"incomplete","reason":"zig_tool_changed_during_check","version":"0.16.0","tool_sha256":tool_sha256,"diagnostics":[]}),
        );
    }
    let mut diagnostics = parse_diagnostic_positions(&outcome.stderr);
    // 原生列是 UTF-8 字节偏移；越界位置或非诊断 stdout 不得变为可修复证据。
    let positions_valid = diagnostics.iter().all(|position| {
        let row = position["line"].as_u64().unwrap_or(0) as usize;
        let column = position["column"].as_u64().unwrap_or(0) as usize;
        row > 0
            && column > 0
            && source
                .split(|b| *b == b'\n')
                .nth(row - 1)
                .is_some_and(|line| column <= line.len() + 1)
    });
    if !positions_valid || !outcome.stdout.is_empty() {
        diagnostics.clear();
    }
    Some(json!({
        "status":match outcome.termination {
            Termination::Exited(0) if outcome.stderr.is_empty() && outcome.stdout.is_empty() => "completed",
            Termination::Exited(1) if !diagnostics.is_empty() => "diagnostics_observed",
            _ => "incomplete",
        },
        "reason":match outcome.termination {
            Termination::Exited(0) if outcome.stderr.is_empty() && outcome.stdout.is_empty() => "ast_check_no_diagnostics",
            Termination::Exited(1) if !diagnostics.is_empty() => "ast_check_diagnostics",
            _ => "zig_ast_check_incomplete",
        },
        "version":"0.16.0",
        "tool_sha256":tool_sha256,
        "diagnostic_count":diagnostics.len(),
        "diagnostics":diagnostics
    }))
}

fn parse_diagnostic_positions(raw: &[u8]) -> Vec<Value> {
    let Ok(text) = std::str::from_utf8(raw) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("<stdin>:")?;
            let (row, rest) = rest.split_once(':')?;
            let (column, rest) = rest.split_once(':')?;
            if !rest.trim_start().starts_with("error:") {
                return None;
            }
            let row = row.parse::<u32>().ok()?;
            let column = column.parse::<u32>().ok()?;
            if row == 0 || column == 0 {
                return None;
            }
            Some(json!({"line":row,"column":column,"rule_id":"zig.ast_check.error"}))
        })
        .take(32)
        .collect()
}
