//! Swift 原生单文件 parse；不执行项目脚本、类型检查、宏插件或源码。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

/// 核对调用方编译器并解析冻结的 UTF-8 stdin；返回有界错误位置或具体未完成原因。
/// 参数为绝对工具路径、源字节和共同截止时间；返回值不证明项目覆盖或交付许可。
pub(crate) fn observe(tool: &Path, source: &[u8], deadline: Instant) -> Value {
    observe_with_cancellation(tool, source, deadline, &AtomicBool::new(false))
}

/// 使用调用方取消令牌观察同一冻结输入；版本探测与源码检查共用令牌和截止时间。
pub(crate) fn observe_with_cancellation(
    tool: &Path,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({"status":"incomplete","reason":"swift_tool_unavailable_or_untrusted","version":null,"tool_sha256":null,"diagnostics":[]});
    if !tool.is_absolute() || std::str::from_utf8(source).is_err() {
        return report;
    }
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, 64 * 1024 * 1024) else {
        return report;
    };
    let sha = digest(&bytes);
    report["tool_sha256"] = json!(sha);
    // 不仅复核字节，还要保留调用方别名的物理入口绑定。
    let tool_current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, 64 * 1024 * 1024)
                .is_ok_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == sha)
    };
    let invoke = |args: &[&str], stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args: args.iter().map(OsString::from).collect(),
                cwd: Path::new("/").to_path_buf(),
                env: BTreeMap::new(),
                stdin,
                deadline,
                output_limit_bytes: 64 * 1024,
            },
            cancelled,
        )
    };
    let version = invoke(&["--version"], None);
    if !tool_current() {
        report["reason"] = json!("swift_tool_changed_during_check");
        return report;
    }
    if version.termination != Termination::Exited(0) {
        report["reason"] = json!(execution_reason(version.termination));
        return report;
    }
    let banner = std::str::from_utf8(&version.stdout).ok();
    // Apple swiftc 的启动器把独立 driver 版本写到 stderr；只接受该固定格式。
    let driver_banner = version.stderr.is_empty()
        || std::str::from_utf8(&version.stderr).ok().is_some_and(|s| {
            s.trim()
                .strip_prefix("swift-driver version: ")
                .is_some_and(|v| {
                    v.len() <= 32
                        && v.split('.').count() == 3
                        && v.split('.').all(|part| {
                            !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())
                        })
                })
        });
    if !driver_banner
        || !banner.is_some_and(|text| {
            text.lines().any(|line| {
                line.strip_prefix("Apple Swift version 6.4")
                    .is_some_and(|tail| tail.starts_with(" ("))
            })
        })
    {
        report["reason"] = json!("swift_version_unverified_or_unsupported");
        return report;
    }
    report["version"] = json!("Apple Swift 6.4");
    // frontend parse 仅读取冻结 stdin；不接受项目 response file、搜索路径或插件参数。
    let outcome = invoke(
        &[
            "-frontend",
            "-parse",
            "-diagnostic-style",
            "llvm",
            "-no-color-diagnostics",
            "-",
        ],
        Some(source.to_vec()),
    );
    if !tool_current() {
        report["reason"] = json!("swift_tool_changed_during_check");
        return report;
    }
    if !matches!(outcome.termination, Termination::Exited(0 | 1)) {
        report["reason"] = json!(execution_reason(outcome.termination));
        return report;
    }
    if !outcome.stdout.is_empty() {
        report["reason"] = json!("swift_syntax_report_invalid");
        return report;
    }
    let Some(rows) = diagnostics(&outcome.stderr, source) else {
        report["reason"] = json!("swift_syntax_report_invalid");
        return report;
    };
    let success = outcome.termination == Termination::Exited(0);
    if success != rows.is_empty() || (success && !outcome.stderr.is_empty()) {
        report["reason"] = json!("swift_native_exit_diagnostics_inconsistent");
        return report;
    }
    report["diagnostics"] = json!(rows);
    report["status"] = json!(if success {
        "completed"
    } else {
        "diagnostics_observed"
    });
    report["reason"] = json!(if success {
        "swift_native_parse_no_diagnostics"
    } else {
        "swift_native_parse_diagnostics"
    });
    report
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn position(source: &[u8], line: u64, column: u64) -> bool {
    line > 0
        && column > 0
        && line <= u32::MAX as u64
        && column <= 1024 * 1024 + 1
        && std::str::from_utf8(source)
            .ok()
            .and_then(|s| s.split('\n').nth((line - 1) as usize))
            .is_some_and(|s| {
                usize::try_from(column - 1)
                    .ok()
                    .is_some_and(|n| n <= s.len() && s.is_char_boundary(n))
            })
}
// 只消费 LLVM 的 stdin 诊断头；上下文必须是对应源码、空行或 caret/fix-it。
// 未知路径、warning、无先行错误的 note 或无法识别输出不猜成代码违规；原始文案不进入公开指引。
fn diagnostics(stderr: &[u8], source: &[u8]) -> Option<Vec<Value>> {
    let text = std::str::from_utf8(stderr).ok()?;
    let source_text = std::str::from_utf8(source).ok()?;
    let mut rows = Vec::new();
    let mut context_line = None;
    for line in text.lines() {
        if let Some(index) = context_line {
            if source_text.split('\n').nth(index) == Some(line) {
                continue;
            }
        }
        if let Some(tail) = line.strip_prefix("<stdin>:") {
            let (l, tail) = tail.split_once(':')?;
            let (c, message) = tail.split_once(':')?;
            let line_number = l.parse::<u64>().ok()?;
            let column = c.parse::<u64>().ok()?;
            if !position(source, line_number, column) {
                return None;
            }
            if message.starts_with(" error: ") {
                if rows.len() >= 32 {
                    return None;
                }
                rows.push(
                    json!({"line":line_number,"column":column,"rule_id":"swift.parse.error"}),
                );
            } else if !message.starts_with(" note: ") || rows.is_empty() {
                return None;
            }
            context_line = Some((line_number - 1) as usize);
        } else if !line.trim().is_empty()
            && !(context_line.is_some()
                && (line.trim_start().starts_with('^') || line.trim_start().starts_with("<#")))
        {
            return None;
        }
    }
    Some(rows)
}

/// 校验历史原生观察的严格字段、版本、原因和坐标；当前字节相同时同时验证 UTF-8 边界。
/// 历史源码变化时仅保留记录，不作为当前修复和任务关闭依据。
pub(crate) fn valid_native_observation(native: &Value, current: Option<&[u8]>) -> bool {
    let keys = ["status", "reason", "version", "tool_sha256", "diagnostics"];
    if !native
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || !matches!(
            native["status"].as_str(),
            Some("not_run" | "incomplete" | "completed" | "diagnostics_observed")
        )
        || !native["reason"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        || !(native["version"].is_null() || native["version"] == "Apple Swift 6.4")
        || !(native["tool_sha256"].is_null()
            || native["tool_sha256"].as_str().is_some_and(valid_sha))
    {
        return false;
    }
    let Some(rows) = native["diagnostics"].as_array().filter(|r| r.len() <= 32) else {
        return false;
    };
    match native["status"].as_str() {
        Some("completed" | "diagnostics_observed") => {
            if native["version"] != "Apple Swift 6.4"
                || native["tool_sha256"].is_null()
                || (native["status"] == "completed"
                    && (!rows.is_empty()
                        || native["reason"] != "swift_native_parse_no_diagnostics"))
                || (native["status"] == "diagnostics_observed"
                    && (rows.is_empty() || native["reason"] != "swift_native_parse_diagnostics"))
            {
                return false;
            }
        }
        Some("not_run") => {
            if !rows.is_empty() || !native["version"].is_null() || !native["tool_sha256"].is_null()
            {
                return false;
            }
        }
        _ => {
            if !rows.is_empty() {
                return false;
            }
        }
    }
    rows.iter().all(|r| {
        r.as_object().is_some_and(|o| {
            o.len() == 3
                && ["line", "column", "rule_id"]
                    .iter()
                    .all(|k| o.contains_key(*k))
        }) && r["rule_id"] == "swift.parse.error"
            && r["line"]
                .as_u64()
                .is_some_and(|l| l > 0 && l <= u32::MAX as u64)
            && r["column"]
                .as_u64()
                .is_some_and(|c| c > 0 && c <= 1024 * 1024 + 1)
            && current.is_none_or(|b| {
                position(
                    b,
                    r["line"].as_u64().unwrap(),
                    r["column"].as_u64().unwrap(),
                )
            })
    })
}
fn execution_reason(t: Termination) -> &'static str {
    match t {
        Termination::TimedOut | Termination::DeadlineBeforeStart => "request_deadline_exceeded",
        Termination::Cancelled => "request_cancelled",
        Termination::OutputLimit => "swift_native_output_budget_exhausted",
        Termination::SpawnFailure => "swift_native_process_unavailable",
        Termination::CleanupFailure => "swift_native_cleanup_incomplete",
        _ => "swift_native_execution_incomplete",
    }
}

#[cfg(test)]
mod tests {
    use super::diagnostics;
    use serde_json::json;

    #[test]
    fn actual_swift_note_does_not_discard_the_eof_error() {
        let stderr = b"<stdin>:2:1: error: expected '}' at end of brace statement\n\n^\n<stdin>:1:10: note: to match this opening '{'\nfunc f() {\n         ^\n";
        assert_eq!(
            diagnostics(stderr, b"func f() {\n"),
            Some(vec![
                json!({"line":2,"column":1,"rule_id":"swift.parse.error"})
            ])
        );
    }

    #[test]
    fn note_only_or_unrecognized_output_cannot_become_clean_or_source_error() {
        for stderr in [
            b"<stdin>:1:10: note: no error\n".as_slice(),
            b"driver failed\n".as_slice(),
        ] {
            assert!(diagnostics(stderr, b"func f() {\n").is_none());
        }
    }
}
