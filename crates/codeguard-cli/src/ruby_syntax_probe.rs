//! 显式 Ruby 的隔离语法观察；用于有界产品反馈与开发对照，不执行源码或代替项目 RuboCop。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

/// 对冻结 UTF-8 stdin 执行固定 Ruby 2.6.10p210 的语法检查。
/// 参数为显式绝对工具路径、原始源码和共同截止时间；返回仅具局部未验证权威的有界观察。
pub(crate) fn observe(
    tool: &Path,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({"status":"incomplete","reason":"ruby_syntax_input_invalid","version":null,"tool_sha256":null,"diagnostics":[]});
    if !tool.is_absolute() || source.len() > 1024 * 1024 || std::str::from_utf8(source).is_err() {
        return report;
    }
    report["reason"] = json!("ruby_syntax_tool_unavailable");
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, 64 * 1024 * 1024) else {
        return report;
    };
    let sha = digest(&bytes);
    report["tool_sha256"] = json!(sha);
    let tool_current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, 64 * 1024 * 1024)
                .is_ok_and(|bytes| digest(&bytes) == sha)
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
    // 版本探测本身可改写制品或请求别名；变化时不得再启动第二次调用。
    if !tool_current() {
        report["reason"] = json!("ruby_syntax_tool_changed");
        return report;
    }
    if version.termination != Termination::Exited(0)
        || !verified_version(&version.stdout)
        || !version.stderr.is_empty()
    {
        report["reason"] = json!("ruby_syntax_version_unverified");
        return report;
    }
    report["version"] = json!("ruby 2.6.10p210");
    let outcome = invoke(
        &["--disable=gems", "-EUTF-8:UTF-8", "-W0", "-c", "-"],
        Some(source.to_vec()),
    );
    if !tool_current() {
        report["reason"] = json!("ruby_syntax_tool_changed");
        return report;
    }
    if !matches!(outcome.termination, Termination::Exited(0 | 1)) {
        report["reason"] = json!("ruby_syntax_execution_incomplete");
        return report;
    }
    let success = outcome.termination == Termination::Exited(0);
    let diagnostics = if success && outcome.stdout == b"Syntax OK\n" && outcome.stderr.is_empty() {
        Some(vec![])
    } else if !success && outcome.stdout.is_empty() {
        diagnostic_lines(&outcome.stderr, source)
    } else {
        None
    };
    let Some(diagnostics) = diagnostics else {
        report["reason"] = json!("ruby_syntax_report_invalid");
        return report;
    };
    report["diagnostics"] = json!(diagnostics);
    report["status"] = json!(if success {
        "completed"
    } else {
        "diagnostics_observed"
    });
    report["reason"] = json!(if success {
        "ruby_native_syntax_no_diagnostics"
    } else {
        "ruby_native_syntax_diagnostics"
    });
    report
}
/// 校验已保存的固定 Ruby 观察；参数为报告和可选当前源码，返回形状及真实行范围是否有效。
/// 不猜测列号；历史源码已改变时仅核验有界行号，当前源码可用时核验行范围。
pub(crate) fn valid_observation(native: &Value, source: Option<&[u8]>) -> bool {
    let keys = ["status", "reason", "version", "tool_sha256", "diagnostics"];
    let valid_sha = |v: &Value| {
        v.as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    };
    if !native
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || !native["reason"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        || !(native["version"].is_null() || native["version"] == "ruby 2.6.10p210")
        || !(native["tool_sha256"].is_null() || valid_sha(&native["tool_sha256"]))
    {
        return false;
    }
    let Some(rows) = native["diagnostics"].as_array().filter(|r| r.len() <= 32) else {
        return false;
    };
    match native["status"].as_str() {
        Some("incomplete") => return rows.is_empty(),
        Some("completed" | "diagnostics_observed") => {}
        _ => return false,
    }
    if native["version"] != "ruby 2.6.10p210"
        || !valid_sha(&native["tool_sha256"])
        || (native["status"] == "completed"
            && (!rows.is_empty() || native["reason"] != "ruby_native_syntax_no_diagnostics"))
        || (native["status"] == "diagnostics_observed"
            && (rows.is_empty() || native["reason"] != "ruby_native_syntax_diagnostics"))
    {
        return false;
    }
    let mut lines = std::collections::BTreeSet::new();
    rows.iter().all(|row| {
        row.as_object()
            .is_some_and(|o| o.len() == 2 && o.contains_key("line") && o.contains_key("rule_id"))
            && row["rule_id"] == "ruby.syntax"
            && row["line"].as_u64().is_some_and(|line| {
                line > 0
                    && line <= u32::MAX as u64
                    && lines.insert(line)
                    && source.is_none_or(|bytes| {
                        std::str::from_utf8(bytes)
                            .is_ok_and(|text| line as usize <= text.lines().count().max(1))
                    })
            })
    })
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// 固定语法版本，平台说明仅作私有运行输出，不扩大语法版本范围。
fn verified_version(stdout: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(stdout) else {
        return false;
    };
    text.ends_with('\n')
        && text.lines().count() == 1
        && text
            .strip_suffix('\n')
            .and_then(|line| line.strip_prefix("ruby 2.6.10p210 "))
            .is_some_and(|rest| rest.starts_with('(') && rest.ends_with(']'))
}

// 消费完整有界报告；不从 caret 显示宽度猜列，不丢弃未知输出后伪造完整分类。
fn diagnostic_lines(stderr: &[u8], source: &[u8]) -> Option<Vec<Value>> {
    let text = std::str::from_utf8(stderr).ok()?;
    let source = std::str::from_utf8(source).ok()?;
    let source_lines: Vec<&str> = source.lines().collect();
    let mut lines = std::collections::BTreeSet::new();
    let mut current = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("-:") {
            let (row, message) = rest.split_once(": ")?;
            let row = row.parse::<usize>().ok()?;
            if row == 0
                || row > source_lines.len().max(1)
                || !(message.starts_with("syntax error, ")
                    || message == "unterminated string meets end of file")
            {
                return None;
            }
            lines.insert(row);
            if lines.len() > 32 {
                return None;
            }
            current = Some(row);
        } else {
            let row = current?;
            let is_source = source_lines
                .get(row - 1)
                .is_some_and(|source| *source == line);
            let is_caret = line.trim() == "^";
            if !(is_source || is_caret || line.is_empty()) {
                return None;
            }
        }
    }
    if lines.is_empty() {
        return None;
    }
    Some(
        lines
            .into_iter()
            .map(|line| json!({"line":line,"rule_id":"ruby.syntax"}))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{diagnostic_lines, valid_observation, verified_version};
    #[test]
    fn history_rejects_fabricated_columns_versions_lines_and_duplicate_diagnostics() {
        let good = serde_json::json!({"status":"diagnostics_observed","reason":"ruby_native_syntax_diagnostics","version":"ruby 2.6.10p210","tool_sha256":"a".repeat(64),"diagnostics":[{"line":1,"rule_id":"ruby.syntax"}]});
        assert!(valid_observation(&good, Some(b"def f(\n")));
        for (key, value) in [
            ("column", serde_json::json!(1)),
            ("rule_id", serde_json::json!("rubocop.Style")),
            ("line", serde_json::json!(2)),
        ] {
            let mut bad = good.clone();
            bad["diagnostics"][0][key] = value;
            assert!(!valid_observation(&bad, Some(b"def f(\n")));
        }
        let mut bad = good.clone();
        bad["version"] = serde_json::json!("ruby 3.4.0");
        assert!(!valid_observation(&bad, None));
        let mut bad = good.clone();
        bad["diagnostics"]
            .as_array_mut()
            .unwrap()
            .push(good["diagnostics"][0].clone());
        assert!(!valid_observation(&bad, None));
    }
    #[test]
    fn inconsistent_exits_and_unexpected_output_never_confirm_syntax() {
        use std::{
            fs,
            os::unix::fs::PermissionsExt,
            sync::atomic::AtomicBool,
            time::{Duration, Instant},
        };
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-ruby-output-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let tool = root.join("ruby");
        for (body, expected) in [
            ("printf 'Syntax OK\\n'; exit 0", "completed"),
            (
                "printf -- '-:1: syntax error, unexpected end-of-input\\n' >&2; exit 1",
                "diagnostics_observed",
            ),
            ("printf 'Syntax OK\\n'; exit 1", "incomplete"),
            ("exit 0", "incomplete"),
            (
                "printf 'Syntax OK\\n'; printf warning >&2; exit 0",
                "incomplete",
            ),
            (
                "printf -- 'other.rb:1: syntax error, unexpected end-of-input\\n' >&2; exit 1",
                "incomplete",
            ),
            (
                "printf -- '-:1: syntax error, unexpected end-of-input\\nfatal runtime\\n' >&2; exit 1",
                "incomplete",
            ),
            ("exit 2", "incomplete"),
        ] {
            fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruby 2.6.10p210 (fixture) [fixture]\\n'; exit 0; fi\n/bin/cat >/dev/null\n{body}\n")).unwrap();
            fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
            let report = super::observe(
                &tool,
                b"x =\n",
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            );
            assert_eq!(report["status"], expected, "{body}: {report}");
            if expected == "incomplete" {
                assert_eq!(report["diagnostics"], serde_json::json!([]));
            }
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn only_exact_version_and_complete_original_stdin_diagnostics_are_classified() {
        assert!(verified_version(b"ruby 2.6.10p210 (fixture) [fixture]\n"));
        for version in [
            b"ruby 2.6.10p211 (fixture) [fixture]\n".as_slice(),
            b"ruby 3.0.0 (fixture) [fixture]\n",
            b"ruby 2.6.10p210 (fixture) [fixture]\nnoise\n",
        ] {
            assert!(!verified_version(version));
        }
        let source = b"s = \"oops\n";
        let valid = b"-:1: unterminated string meets end of file\ns = \"oops\n          ^\n";
        assert_eq!(diagnostic_lines(valid, source).unwrap()[0]["line"], 1);
        for stderr in [
            b"other.rb:1: syntax error, unexpected end-of-input\n".as_slice(),
            b"-:0: syntax error, unexpected end-of-input\n",
            b"-:2: syntax error, unexpected end-of-input\n",
            b"-:1: warning: unused literal\n",
            b"-:1: syntax error, unexpected end-of-input\nfatal: unavailable runtime\n",
            b"-:1: syntax error, unexpected end-of-input\nother source\n",
            b"\xff",
        ] {
            assert!(diagnostic_lines(stderr, source).is_none(), "{stderr:?}");
        }
        let source = "x\n".repeat(33);
        let stderr = (1..=33)
            .map(|line| format!("-:{line}: syntax error, unexpected end-of-input\n"))
            .collect::<String>();
        assert!(diagnostic_lines(stderr.as_bytes(), source.as_bytes()).is_none());
    }
}
