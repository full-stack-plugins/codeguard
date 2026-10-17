//! Zig 原生 AST 探针供 lint 与任务复检共用；不授予完整 lint 或关闭权威。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

/// 用已提供工具检查指定原字节；参数含受控 cwd 与共享 deadline，返回有界原生观察。
pub(crate) fn observe(tool: &Path, source: &[u8], cwd: &Path, deadline: Instant) -> Option<Value> {
    observe_with_cancellation(tool, source, cwd, deadline, &AtomicBool::new(false))
}

/// 使用调用方取消令牌观察同一冻结输入；版本探测与源码检查共用令牌和截止时间。
pub(crate) fn observe_with_cancellation(
    tool: &Path,
    source: &[u8],
    cwd: &Path,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Option<Value> {
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
    // 版本调用也可能改写制品或请求别名；源码调用前后都必须核对冻结入口。
    let tool_current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, 64 * 1024 * 1024)
                .is_ok_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == tool_sha256)
    };
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
        cancelled,
    );
    if !tool_current() {
        return Some(
            json!({"status":"incomplete","reason":"zig_tool_changed_during_check","version":null,"tool_sha256":tool_sha256,"diagnostics":[]}),
        );
    }
    // 请求级终止（取消/预算耗尽）不是工具缺陷；先于版本判定保留原始原因。
    if let Some(reason) = request_reason(&version.termination) {
        return Some(
            json!({"status":"incomplete","reason":reason,"version":null,"tool_sha256":tool_sha256,"diagnostics":[]}),
        );
    }
    let version_verified = version.termination == Termination::Exited(0)
        && version.stderr.is_empty()
        && std::str::from_utf8(&version.stdout).is_ok_and(|text| text.trim() == "0.16.0");
    if !version_verified {
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
        cancelled,
    );
    if !tool_current() {
        return Some(
            json!({"status":"incomplete","reason":"zig_tool_changed_during_check","version":"0.16.0","tool_sha256":tool_sha256,"diagnostics":[]}),
        );
    }
    if let Some(reason) = request_reason(&outcome.termination) {
        return Some(
            json!({"status":"incomplete","reason":reason,"version":"0.16.0","tool_sha256":tool_sha256,"diagnostics":[]}),
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

/// 请求级终止原因；取消/预算耗尽不能改写为工具版本或执行缺陷。
fn request_reason(termination: &Termination) -> Option<&'static str> {
    match termination {
        Termination::Cancelled => Some("request_cancelled"),
        Termination::TimedOut | Termination::DeadlineBeforeStart => {
            Some("request_deadline_exceeded")
        }
        _ => None,
    }
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

#[cfg(test)]
mod request_tests {
    use super::observe_with_cancellation;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::{Duration, Instant},
    };
    fn root(name: &str) -> PathBuf {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-zig-probe-{name}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        root
    }
    fn executable(path: &std::path::Path, script: &str) {
        fs::write(path, script).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    #[test]
    fn request_level_termination_is_not_relabelled_as_tool_defects() {
        for phase in ["version", "ast_check"] {
            let root = root(&format!("cancel-{phase}"));
            let tool = root.join("zig");
            let block = "printf started > \"$0.started\"; exec /bin/sleep 30";
            let script = if phase == "version" {
                format!("#!/bin/sh\n{block}\n")
            } else {
                "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\n"
                    .to_owned()
                    + &format!("{block}\n")
            };
            executable(&tool, &script);
            let marker = tool.with_extension("started");
            let cancelled = Arc::new(AtomicBool::new(false));
            let trigger = Arc::clone(&cancelled);
            let watcher = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(4);
                while !marker.exists() && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(5));
                }
                let started = marker.exists();
                trigger.store(true, Ordering::Relaxed);
                started
            });
            let started = Instant::now();
            let report = observe_with_cancellation(
                &tool,
                b"const Empty = struct {};\n",
                &root,
                started + Duration::from_secs(10),
                &cancelled,
            )
            .expect("取消仍应返回结构化观察");
            assert!(watcher.join().unwrap(), "{phase}: 原生阶段必须先启动");
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "{phase}: 取消未终止活动进程"
            );
            assert_eq!(report["status"], "incomplete", "{phase}: {report}");
            assert_eq!(report["reason"], "request_cancelled", "{phase}: {report}");
            assert_eq!(report["diagnostics"], serde_json::json!([]));
            fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn expired_deadline_reports_request_budget_not_version_mismatch() {
        let root = root("deadline");
        let tool = root.join("zig");
        executable(&tool, "#!/bin/sh\nprintf '0.16.0\\n'\n");
        let report = super::observe(
            &tool,
            b"const Empty = struct {};\n",
            &root,
            Instant::now() - Duration::from_millis(1),
        )
        .expect("预算耗尽仍应返回结构化观察");
        assert_eq!(report["status"], "incomplete", "{report}");
        assert_eq!(report["reason"], "request_deadline_exceeded", "{report}");
        assert_eq!(report["diagnostics"], serde_json::json!([]));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn non_utf8_version_output_is_a_version_failure_not_a_missing_tool() {
        let root = root("version-bytes");
        let tool = root.join("zig");
        executable(&tool, "#!/bin/sh\nprintf '\\377\\n'\n");
        let report = super::observe(
            &tool,
            b"const Empty = struct {};\n",
            &root,
            Instant::now() + Duration::from_secs(5),
        )
        .expect("工具可用但版本输出非法仍应返回结构化观察");
        assert_eq!(report["status"], "incomplete", "{report}");
        assert_eq!(
            report["reason"], "zig_version_unverified_or_unsupported",
            "{report}"
        );
        assert_eq!(report["version"], serde_json::Value::Null);
        fs::remove_dir_all(root).unwrap();
    }
}
