//! 显式 Node 的隔离模块语法观察；不加载导入、不执行源码或代替项目 ESLint。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

/// 已安装固定 Node 的独立制品预算；其它原生检查器保留原有上限。
pub(crate) const NODE_ARTIFACT_BUDGET: u64 = 128 * 1024 * 1024;

/// 对冻结 UTF-8 stdin 执行固定 Node 24.18.0 的 module 语法检查。
/// 参数为显式绝对工具路径、原始源码和共同截止时间；返回仅具开发对照权威的有界观察。
pub(crate) fn observe(
    tool: &Path,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({"status":"incomplete","reason":"javascript_syntax_input_invalid","version":null,"tool_sha256":null,"input_type":"module","diagnostics":[]});
    if !tool.is_absolute() || source.len() > 1024 * 1024 || std::str::from_utf8(source).is_err() {
        return report;
    }
    report["reason"] = json!("javascript_syntax_tool_unavailable");
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, NODE_ARTIFACT_BUDGET) else {
        return report;
    };
    let sha = digest(&bytes);
    report["tool_sha256"] = json!(sha);
    let tool_current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, NODE_ARTIFACT_BUDGET)
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
        report["reason"] = json!("javascript_syntax_tool_changed");
        return report;
    }
    if version.termination != Termination::Exited(0)
        || version.stdout != b"v24.18.0\n"
        || !version.stderr.is_empty()
    {
        report["reason"] = json!("javascript_syntax_version_unverified");
        return report;
    }
    report["version"] = json!("v24.18.0");
    let outcome = invoke(&["--check", "--input-type=module"], Some(source.to_vec()));
    if !tool_current() {
        report["reason"] = json!("javascript_syntax_tool_changed");
        return report;
    }
    if !matches!(outcome.termination, Termination::Exited(0 | 1)) {
        report["reason"] = json!("javascript_syntax_execution_incomplete");
        return report;
    }
    if !outcome.stdout.is_empty() {
        report["reason"] = json!("javascript_syntax_report_invalid");
        return report;
    }
    let success = outcome.termination == Termination::Exited(0);
    let diagnostics = if success && outcome.stderr.is_empty() {
        Some(vec![])
    } else if !success {
        diagnostic_line(&outcome.stderr, source)
            .map(|line| vec![json!({"line":line,"rule_id":"javascript.syntax"})])
    } else {
        None
    };
    let Some(diagnostics) = diagnostics else {
        report["reason"] = json!("javascript_syntax_report_invalid");
        return report;
    };
    report["diagnostics"] = json!(diagnostics);
    report["status"] = json!(if success {
        "completed"
    } else {
        "diagnostics_observed"
    });
    report["reason"] = json!(if success {
        "javascript_native_syntax_no_diagnostics"
    } else {
        "javascript_native_syntax_diagnostics"
    });
    report
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// 仅保留已核对的原始行号；不从 caret 的显示宽度猜测字节列或传播源码/诊断文案。
fn diagnostic_line(stderr: &[u8], source: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(stderr).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let line = lines
        .first()?
        .strip_prefix("[stdin]:")?
        .parse::<u64>()
        .ok()?;
    let original = std::str::from_utf8(source)
        .ok()?
        .split('\n')
        .nth(usize::try_from(line.checked_sub(1)?).ok()?)?;
    if lines.get(1).copied() != Some(original.strip_suffix('\r').unwrap_or(original))
        || lines.get(3) != Some(&"")
        || !lines.get(4)?.starts_with("SyntaxError: ")
        || lines.last() != Some(&"Node.js v24.18.0")
    {
        return None;
    }
    let caret = lines.get(2)?;
    if !caret.bytes().all(|b| matches!(b, b' ' | b'\t' | b'^'))
        || (!original.is_empty() && !caret.contains('^'))
    {
        return None;
    }
    let stack = lines.get(5..lines.len().checked_sub(1)?)?;
    if !stack
        .iter()
        .any(|l| l.starts_with("    at checkSyntax (node:internal/main/check_syntax:"))
        || stack
            .iter()
            .any(|l| !l.is_empty() && !(l.starts_with("    at ") && l.contains("node:")))
    {
        return None;
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::diagnostic_line;
    #[test]
    fn only_frozen_stdin_syntax_header_is_accepted() {
        let valid=b"[stdin]:1\nconst x = ;\n          ^\n\nSyntaxError: Unexpected token\n    at checkSyntax (node:internal/main/check_syntax:72:5)\n\nNode.js v24.18.0\n";
        assert_eq!(diagnostic_line(valid, b"const x = ;\n"), Some(1));
        for changed in [
            String::from_utf8_lossy(valid).replace("const x = ;", "other source"),
            String::from_utf8_lossy(valid).replace("[stdin]:1", "[stdin]:0"),
            String::from_utf8_lossy(valid).replace("[stdin]:1", "[stdin]:3"),
            String::from_utf8_lossy(valid).replace("SyntaxError:", "Error:"),
            String::from_utf8_lossy(valid).replace("[stdin]", "/project/app.js"),
            String::from_utf8_lossy(valid).replace("v24.18.0", "v24.17.0"),
            String::from_utf8_lossy(valid).replace("          ^", "no caret"),
        ] {
            assert_eq!(diagnostic_line(changed.as_bytes(), b"const x = ;\n"), None);
        }
        assert_eq!(diagnostic_line(b"[stdin]:2\n\n\n\nSyntaxError: Unexpected end of input\n    at checkSyntax (node:internal/main/check_syntax:72:5)\n\nNode.js v24.18.0\n",b"function f() {\n"),Some(2));
    }
    #[test]
    fn changed_tool_is_not_invoked_after_version_probe() {
        use std::{
            fs,
            os::unix::fs::PermissionsExt,
            time::{Duration, Instant},
        };
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-node-version-change-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let tool = root.join("node");
        fs::write(&tool,"#!/bin/sh\nprintf '#!/bin/sh\\nprintf x >> \"$0.called\"\\nexit 0\\n' > \"$0\"\nprintf 'v24.18.0\\n'\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let report = super::observe(
            &tool,
            b"const x = 1;",
            Instant::now() + Duration::from_secs(5),
            &std::sync::atomic::AtomicBool::new(false),
        );
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["reason"], "javascript_syntax_tool_changed");
        assert!(!tool.with_extension("called").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn in_flight_native_check_observes_request_cancellation() {
        use std::{
            fs,
            os::unix::fs::PermissionsExt,
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            },
            thread,
            time::{Duration, Instant},
        };
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-node-cancel-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let tool = root.join("node");
        fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'v24.18.0\\n'; exit 0; fi\nprintf started > \"$0.started\"\nexec /bin/sleep 2\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let trigger = Arc::clone(&cancelled);
        let marker = tool.with_extension("started");
        let observer = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(4);
            while !marker.exists() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            assert!(marker.exists(), "必须在原生检查实际启动后取消");
            trigger.store(true, Ordering::Relaxed);
        });
        let started = Instant::now();
        let report = super::observe(
            &tool,
            b"const x = 1;",
            started + Duration::from_secs(5),
            &cancelled,
        );
        observer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
        assert_eq!(report["status"], "incomplete");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "取消不能等待原生调用自行完成"
        );
        assert_eq!(report["reason"], "javascript_syntax_execution_incomplete");
    }
}
