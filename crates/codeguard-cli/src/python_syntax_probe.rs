//! 使用已安装 Ruff 对冻结 stdin 做隔离语法检查，不加载项目配置或执行用户代码。
use codeguard_adapters::{RuffParseState, parse_ruff_json};
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

/// 返回固定 Ruff 0.16.8 / Python 3.12 的局部语法观察。
/// 参数为绝对工具路径、源码字节和总截止时间；没有项目、规则或交付授权。
pub(crate) fn observe(tool: &Path, source: &[u8], deadline: Instant) -> Value {
    let mut report = json!({"status":"incomplete","reason":"python_syntax_tool_unavailable","version":null,"tool_sha256":null,"target_version":"py312","diagnostics":[]});
    if !tool.is_absolute() || source.len() > 1024 * 1024 || std::str::from_utf8(source).is_err() {
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
            &AtomicBool::new(false),
        )
    };
    let version = invoke(&["--version"], None);
    // 版本探测本身可改变入口；再次执行前核对请求路径与冻结制品，不能等执行后才撤回结果。
    let tool_current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, 64 * 1024 * 1024)
                .ok()
                .is_some_and(|bytes| digest(&bytes) == sha)
    };
    if !tool_current() {
        report["reason"] = json!("python_syntax_tool_changed");
        return report;
    }
    if version.termination != Termination::Exited(0)
        || version.stdout != b"ruff 0.16.8\n"
        || !version.stderr.is_empty()
    {
        report["reason"] = json!("python_syntax_version_unverified");
        return report;
    }
    report["version"] = json!("ruff 0.16.8");
    // 选择 E9 仅触发解析/I/O；原生 invalid-syntax 不受 noqa 抑制。其它规则不能当语法错误。
    let outcome = invoke(
        &[
            "check",
            "--isolated",
            "--no-cache",
            "--ignore-noqa",
            "--select",
            "E9",
            "--target-version",
            "py312",
            "--output-format",
            "json",
            "--stdin-filename",
            "codeguard_input.py",
            "-",
        ],
        Some(source.to_vec()),
    );
    if !tool_current() {
        report["reason"] = json!("python_syntax_tool_changed");
        return report;
    }
    let code = match outcome.termination {
        Termination::Exited(code @ (0 | 1)) => code,
        _ => {
            report["reason"] = json!("python_syntax_execution_incomplete");
            return report;
        }
    };
    if !outcome.stderr.is_empty() {
        report["reason"] = json!("python_syntax_unexpected_stderr");
        return report;
    }
    let Some(rows) = parse_syntax(code, &outcome.stdout, source) else {
        report["reason"] = json!("python_syntax_report_invalid");
        return report;
    };
    report["status"] = json!(if rows.is_empty() {
        "completed"
    } else {
        "diagnostics_observed"
    });
    report["reason"] = json!(if rows.is_empty() {
        "python_native_syntax_no_diagnostics"
    } else {
        "python_native_syntax_diagnostics"
    });
    report["diagnostics"] = json!(rows);
    report
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn parse_syntax(code: i32, stdout: &[u8], source: &[u8]) -> Option<Vec<Value>> {
    let parsed = parse_ruff_json(code, stdout);
    if parsed.state != RuffParseState::Valid {
        return None;
    }
    let text = std::str::from_utf8(source).ok()?;
    parsed.diagnostics.into_iter().map(|diagnostic| {
        let line = diagnostic.location.row as usize;
        let column = diagnostic.location.column as usize;
        // 保留原生列，不将它未经证明地转换为编辑坐标；这里只核验行/列的保守上界。
        if diagnostic.code != "invalid-syntax" || diagnostic.severity != "error"
            || diagnostic.filename != "/codeguard_input.py"
            || !text.split('\n').nth(line.checked_sub(1)?).is_some_and(|s| column > 0 && column <= s.len() + 1) {
            return None;
        }
        Some(json!({"line":line,"column":column,"column_unit":"ruff_reported","rule_id":"invalid-syntax"}))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::parse_syntax;
    use serde_json::json;
    #[test]
    #[ignore = "requires installed Ruff0.16.8 via CODEGUARD_RUFF_SYNTAX_BIN"]
    fn actual_ruff_stable_tool_preserves_valid_and_invalid_syntax_results() {
        use std::{
            fs,
            path::PathBuf,
            time::{Duration, Instant},
        };
        let tool = PathBuf::from(std::env::var("CODEGUARD_RUFF_SYNTAX_BIN").unwrap())
            .canonicalize()
            .unwrap();
        let before = super::digest(&fs::read(&tool).unwrap());
        let deadline = Instant::now() + Duration::from_secs(20);
        for (source, expected) in [
            (b"x = 1\n".as_slice(), "completed"),
            (b"def run():\n".as_slice(), "diagnostics_observed"),
            (b"if True:\npass\n".as_slice(), "diagnostics_observed"),
        ] {
            let report = super::observe(&tool, source, deadline);
            assert_eq!(report["status"], expected, "{report}");
            assert_eq!(report["version"], "ruff 0.16.8");
            assert_eq!(report["target_version"], "py312");
            assert_eq!(report["tool_sha256"], before);
            assert_eq!(
                report["diagnostics"].as_array().unwrap().is_empty(),
                expected == "completed"
            );
        }
        assert_eq!(super::digest(&fs::read(tool).unwrap()), before);
    }
    #[test]
    fn version_probe_cannot_redirect_alias_before_syntax_invocation() {
        use std::{
            fs,
            os::unix::fs::{PermissionsExt, symlink},
            time::{Duration, Instant},
        };
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-python-probe-alias-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let original = root.join("ruff.original");
        fs::write(&original, "#!/bin/sh\nif [ \"$1\" = --version ]; then\n/bin/ln -sf \"$0.next\" \"$0.alias\"\nprintf 'ruff 0.16.8\\n'\nelse\nprintf executed > \"$0.executed\"\nprintf '[]'\nfi\n").unwrap();
        fs::write(root.join("ruff.original.next"), "#!/bin/sh\nprintf '[]'\n").unwrap();
        fs::set_permissions(&original, fs::Permissions::from_mode(0o700)).unwrap();
        let alias = root.join("ruff.original.alias");
        symlink(&original, &alias).unwrap();
        let report = super::observe(&alias, b"x = 1\n", Instant::now() + Duration::from_secs(5));
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["reason"], "python_syntax_tool_changed");
        assert!(!root.join("ruff.original.executed").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn tool_replaced_by_version_probe_is_not_executed_again() {
        use std::{
            fs,
            os::unix::fs::PermissionsExt,
            time::{Duration, Instant},
        };
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-python-probe-replacement-{}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let tool = root.join("ruff");
        let replacement = root.join("ruff.next");
        fs::write(
            &tool,
            "#!/bin/sh\n/bin/mv \"$0.next\" \"$0\"\nprintf 'ruff 0.16.8\\n'\n",
        )
        .unwrap();
        fs::write(
            &replacement,
            "#!/bin/sh\nprintf executed > \"$0.executed\"\nprintf '[]'\n",
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&replacement, fs::Permissions::from_mode(0o700)).unwrap();
        let report = super::observe(&tool, b"x = 1\n", Instant::now() + Duration::from_secs(5));
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["reason"], "python_syntax_tool_changed");
        assert!(
            !root.join("ruff.executed").exists(),
            "变更后的制品不能作为第二次调用执行"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn only_consistent_located_syntax_reports_are_classified() {
        let item = json!({"code":"invalid-syntax","message":"expected token","filename":"/codeguard_input.py","severity":"error","location":{"row":1,"column":1}});
        assert!(parse_syntax(0, b"[]", b"x = 1\n").is_some());
        assert!(
            parse_syntax(
                1,
                &serde_json::to_vec(&vec![item.clone()]).unwrap(),
                b"def (\n"
            )
            .is_some()
        );
        assert!(
            parse_syntax(
                0,
                &serde_json::to_vec(&vec![item.clone()]).unwrap(),
                b"def (\n"
            )
            .is_none()
        );
        assert!(parse_syntax(1, b"[]", b"def (\n").is_none());
        for (field, value) in [
            ("code", json!("F401")),
            ("code", json!("E902")),
            ("filename", json!("/other.py")),
            ("severity", json!("warning")),
            ("location", json!({"row":99,"column":1})),
        ] {
            let mut bad = item.clone();
            bad[field] = value;
            assert!(
                parse_syntax(1, &serde_json::to_vec(&vec![bad]).unwrap(), b"def (\n").is_none()
            );
        }
    }
}
