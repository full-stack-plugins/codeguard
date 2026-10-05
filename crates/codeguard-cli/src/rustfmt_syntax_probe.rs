use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::Path,
    sync::atomic::AtomicBool,
    time::Instant,
};

/// 对固定 edition2024 的冻结 stdin 作 Rustfmt 解析；参数是显式入口、源码及共同预算。
/// 返回脱敏语法观察，不检查格式差异、不执行 Cargo/源码，也不授予 lint 或项目覆盖。
#[cfg(any(feature = "wasm-precheck", test))]
pub(crate) fn observe(
    tool: &Path,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    observe_for_edition(tool, source, "2024", deadline, cancelled)
}

/// 使用已解析Cargo edition的同字节原生观察；返回脱敏解析协议而非lint结论。
#[cfg(any(feature = "wasm-precheck", test))]
pub(crate) fn observe_for_edition(
    tool: &Path,
    source: &[u8],
    edition: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    observe_with_context_guard(tool, source, edition, deadline, cancelled, &|| true)
}

/// 在版本调用与解析之间核对项目声明/源码上下文；参数含只读连续性检查。
pub(crate) fn observe_with_context_guard(
    tool: &Path,
    source: &[u8],
    edition: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
    context_current: &dyn Fn() -> bool,
) -> Value {
    let mut report = json!({"status":"incomplete","reason":"rustfmt_syntax_input_invalid","version":null,"tool_sha256":null,"edition":"2024","observation_kind":"formatter_parser","diagnostics":[]});
    if !matches!(edition, "2015" | "2018" | "2021" | "2024") {
        return report;
    }
    report["edition"] = json!(edition);
    if !tool.is_absolute() || source.len() > 1024 * 1024 || std::str::from_utf8(source).is_err() {
        return report;
    }
    report["reason"] = json!("rustfmt_syntax_tool_unavailable");
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, 64 * 1024 * 1024) else {
        return report;
    };
    let sha = format!("{:x}", Sha256::digest(&bytes));
    report["tool_sha256"] = json!(sha);
    let Some(scratch) = crate::rustfmt_scratch::RustfmtScratch::create_for_edition(edition) else {
        report["reason"] = json!("rustfmt_private_config_unavailable");
        return report;
    };
    let tool_current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, 64 * 1024 * 1024)
                .is_ok_and(|b| format!("{:x}", Sha256::digest(&b)) == sha)
    };
    let invoke = |args: &[&str], stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args: args.iter().map(OsString::from).collect(),
                cwd: scratch.root.clone(),
                env: BTreeMap::new(),
                stdin,
                deadline,
                output_limit_bytes: 2 * 1024 * 1024,
            },
            cancelled,
        )
    };
    let version = invoke(&["--version"], None);
    if !tool_current() {
        report["reason"] = json!("rustfmt_syntax_tool_changed");
        return report;
    }
    if !scratch.current() {
        report["reason"] = json!("rustfmt_private_config_changed");
        return report;
    }
    if version.termination != Termination::Exited(0)
        || !verified_version(&version.stdout)
        || !version.stderr.is_empty()
    {
        report["reason"] = json!("rustfmt_syntax_version_unverified");
        return report;
    }
    report["version"] = json!("rustfmt 1.9.0-stable");
    if !context_current() {
        report["reason"] = json!("rustfmt_project_context_changed");
        return report;
    }
    // 不用 --check：格式差异不是语法错误；stdin 不向原文件写入格式结果。
    let outcome = invoke(
        &[
            "--config-path",
            "fixed.toml",
            "--emit",
            "stdout",
            "--color",
            "never",
        ],
        Some(source.to_vec()),
    );
    if !tool_current() {
        report["reason"] = json!("rustfmt_syntax_tool_changed");
        return report;
    }
    if !scratch.current() {
        report["reason"] = json!("rustfmt_private_config_changed");
        return report;
    }
    let (status, reason, diagnostics) = match outcome.termination {
        Termination::Exited(0)
            if outcome.stderr.is_empty()
                && outcome.stdout.ends_with(b"\n")
                && std::str::from_utf8(&outcome.stdout).is_ok() =>
        {
            (
                "completed",
                "rustfmt_native_parse_no_diagnostics",
                Some(Vec::new()),
            )
        }
        Termination::Exited(1 | 101) if outcome.stdout.is_empty() => (
            "diagnostics_observed",
            "rustfmt_native_parse_diagnostics",
            diagnostic_lines(&outcome.stderr, source),
        ),
        Termination::Exited(0 | 1 | 101) => ("incomplete", "rustfmt_syntax_report_invalid", None),
        _ => ("incomplete", "rustfmt_syntax_execution_incomplete", None),
    };
    let Some(diagnostics) = diagnostics else {
        report["reason"] = json!(if status == "diagnostics_observed" {
            "rustfmt_syntax_report_invalid"
        } else {
            reason
        });
        return report;
    };
    if !context_current() {
        report["reason"] = json!("rustfmt_project_context_changed");
        return report;
    }
    report["status"] = json!(status);
    report["reason"] = json!(reason);
    report["diagnostics"] = json!(diagnostics);
    report
}

fn verified_version(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| {
            s.strip_prefix("rustfmt 1.9.0-stable (")?
                .strip_suffix(")\n")
        })
        .is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 96
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b' ')
        })
}
// 原生列涉及显示/字符坐标；只保留核对到冻结输入的行号，拒绝外部文件和自由文本。
fn diagnostic_lines(stderr: &[u8], source: &[u8]) -> Option<Vec<Value>> {
    let text = std::str::from_utf8(stderr).ok()?;
    let header = text.lines().next()?;
    let coded_error = header
        .strip_prefix("error[E")
        .and_then(|tail| tail.split_once("]: "))
        .is_some_and(|(code, _)| code.len() == 4 && code.bytes().all(|b| b.is_ascii_digit()));
    if !(header.starts_with("error: ") || coded_error)
        || header.starts_with("error: internal compiler error")
        || text
            .lines()
            .any(|line| line.starts_with("thread ") && line.contains("panicked at"))
    {
        return None;
    }
    let source_lines: Vec<&str> = std::str::from_utf8(source).ok()?.split('\n').collect();
    let max_line = source_lines.len();
    let mut rows = BTreeSet::new();
    for location in text
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("--> "))
    {
        let location = location.strip_prefix("<stdin>:")?;
        let (line, column) = location.split_once(':')?;
        let line = line.parse::<usize>().ok()?;
        let column = column.parse::<usize>().ok()?;
        if line == 0
            || line > max_line
            || column == 0
            || rows.len() >= 32
            || column
                > source_lines[line - 1].len()
                    + 1
                    + usize::from(
                        header == "error: this file contains an unclosed delimiter"
                            && source.ends_with(b"\n")
                            && line + 1 == max_line,
                    )
        {
            return None;
        }
        rows.insert(line);
    }
    if rows.is_empty() {
        return None;
    }
    Some(
        rows.into_iter()
            .map(|line| json!({"line":line,"rule_id":"rust.syntax"}))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_inputs_do_not_start_tools_or_create_configuration() {
        let report = super::observe(
            std::path::Path::new("relative-rustfmt"),
            &[0xff],
            std::time::Instant::now(),
            &std::sync::atomic::AtomicBool::new(false),
        );
        assert_eq!(report["reason"], "rustfmt_syntax_input_invalid");
        assert!(report["tool_sha256"].is_null());
        assert!(report["diagnostics"].as_array().unwrap().is_empty());
    }
    #[test]
    fn foreign_unlocated_or_out_of_range_errors_are_not_source_diagnostics() {
        for error in [
            "error: bad\n --> /project/file.rs:1:1\n",
            "error: bad\n --> <stdin>:0:1\n",
            "error: bad\n --> <stdin>:3:1\n",
            "error: bad\n --> <stdin>:1:999\n",
            "error: bad\n",
            "Warning: config\nerror: bad\n --> <stdin>:1:1\n",
            "error: bad\n --> <stdin>:1:1\nthread 'main' panicked at internal.rs:1\n",
        ] {
            assert!(super::diagnostic_lines(error.as_bytes(), b"fn f() {}\n").is_none());
        }
        assert!(
            super::diagnostic_lines(b"error: bad\n --> <stdin>:1:1\n", b"fn f() {}\n").is_some()
        );
    }
    #[test]
    fn known_lexer_error_and_bounded_eof_anchor_are_accepted() {
        assert!(
            super::diagnostic_lines(
                b"error[E0765]: unterminated double quote string\n --> <stdin>:1:17\n",
                b"const S: &str = \"oops;\n"
            )
            .is_some()
        );
        assert!(
            super::diagnostic_lines(
                b"error: this file contains an unclosed delimiter\n --> <stdin>:1:13\n",
                b"fn main() {\n"
            )
            .is_some()
        );
    }
    #[test]
    fn private_config_changes_are_visible() {
        let scratch = crate::rustfmt_scratch::RustfmtScratch::create().unwrap();
        assert!(scratch.current());
        std::fs::write(scratch.config_path(), "edition=\"2015\"\n").unwrap();
        assert!(!scratch.current());
    }
}
