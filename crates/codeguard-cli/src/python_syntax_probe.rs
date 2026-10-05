//! 使用已安装Ruff对冻结stdin做语法检查；隔离入口与批准配置入口分别绑定上下文，不执行用户代码。
use codeguard_adapters::{RuffParseState, parse_ruff_json};
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

/// 返回固定 Ruff 0.16.8 / Python 3.12 的局部语法观察。
/// 参数为绝对工具路径、源码字节和总截止时间；没有项目、规则或交付授权。
#[cfg(any(feature = "wasm-precheck", test))]
pub(crate) fn observe(
    tool: &Path,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    observe_with_context(tool, source, "py312", None, deadline, cancelled)
}

/// 对冻结源码按明确目标Python版本执行隔离语法检查。
/// 参数为绝对工具路径、源码、宿主已核对目标及截止时间；返回局部观察，不推断项目目标或批准关闭。
#[cfg(test)]
pub(crate) fn observe_for_target(
    tool: &Path,
    source: &[u8],
    target: &str,
    deadline: Instant,
) -> Value {
    observe_with_context(
        tool,
        source,
        target,
        None,
        deadline,
        &AtomicBool::new(false),
    )
}

/// 对原始stdin按批准的项目配置与明确目标复检，绝不改写当前源码。
/// 参数包括普通配置与绑定源码的绝对路径；调用者必须在前后核对配置与输入身份。
pub(crate) fn observe_configured(
    tool: &Path,
    source: &[u8],
    target: &str,
    config: &Path,
    path: &Path,
    deadline: Instant,
) -> Value {
    observe_with_context(
        tool,
        source,
        target,
        Some((config, path)),
        deadline,
        &AtomicBool::new(false),
    )
}

fn observe_with_context(
    tool: &Path,
    source: &[u8],
    target: &str,
    context: Option<(&Path, &Path)>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({"status":"incomplete","reason":"python_syntax_target_unverified","version":null,"tool_sha256":null,"target_version":null,"diagnostics":[]});
    // 目标作为独立argv使用，版本集合固定于已验收Ruff，不接受自由参数或默认为项目版本。
    if !matches!(
        target,
        "py37" | "py38" | "py39" | "py310" | "py311" | "py312" | "py313" | "py314" | "py315"
    ) {
        return report;
    }
    report["target_version"] = json!(target);
    report["reason"] = json!("python_syntax_tool_unavailable");
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
            cancelled,
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
    let filename = context.map_or(Some("/codeguard_input.py"), |(_, path)| path.to_str());
    let Some(filename) = filename else {
        report["reason"] = json!("python_syntax_context_unverified");
        return report;
    };
    let mut args = vec![
        "check",
        "--no-cache",
        "--ignore-noqa",
        "--select",
        "E9",
        "--target-version",
        target,
        "--output-format",
        "json",
        "--stdin-filename",
        filename,
    ];
    if let Some((config, path)) = context {
        let Some(config) = config
            .to_str()
            .filter(|_| config.is_absolute() && path.is_absolute())
        else {
            report["reason"] = json!("python_syntax_context_unverified");
            return report;
        };
        args.extend(["--config", config]);
    } else {
        args.push("--isolated");
    }
    args.push("-");
    let outcome = invoke(&args, Some(source.to_vec()));
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
    let Some(rows) = parse_syntax_at(code, &outcome.stdout, source, filename) else {
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
#[cfg(test)]
fn parse_syntax(code: i32, stdout: &[u8], source: &[u8]) -> Option<Vec<Value>> {
    parse_syntax_at(code, stdout, source, "/codeguard_input.py")
}
fn parse_syntax_at(code: i32, stdout: &[u8], source: &[u8], filename: &str) -> Option<Vec<Value>> {
    let parsed = parse_ruff_json(code, stdout);
    if parsed.state != RuffParseState::Valid || parsed.diagnostics.len() > 512 {
        return None;
    }
    let text = std::str::from_utf8(source).ok()?;
    parsed.diagnostics.into_iter().map(|diagnostic| {
        let line = diagnostic.location.row as usize;
        let column = diagnostic.location.column as usize;
        // 保留原生列，不将它未经证明地转换为编辑坐标；这里只核验行/列的保守上界。
        if diagnostic.code != "invalid-syntax" || diagnostic.severity != "error"
            || diagnostic.filename != filename
            || !text.split('\n').nth(line.checked_sub(1)?).is_some_and(|s| column > 0 && column <= s.len() + 1) {
            return None;
        }
        Some(json!({"line":line,"column":column,"column_unit":"ruff_reported","rule_id":"invalid-syntax"}))
    }).collect()
}

/// 核对脱敏Python原生观察的封闭形状和可选源码位置，不证明工具或批准来源。
pub(crate) fn valid_native_observation(value: &Value, source: Option<&[u8]>) -> bool {
    // 零诊断也必须核对源码格式，不能让空数组绕过UTF-8或大小限制。
    if source.is_some_and(|bytes| bytes.len() > 1024 * 1024 || std::str::from_utf8(bytes).is_err())
    {
        return false;
    }
    let keys = [
        "status",
        "reason",
        "version",
        "tool_sha256",
        "target_version",
        "diagnostics",
    ];
    if !value
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
    {
        return false;
    }
    let target = value["target_version"].as_str().is_some_and(|target| {
        matches!(
            target,
            "py37" | "py38" | "py39" | "py310" | "py311" | "py312" | "py313" | "py314" | "py315"
        )
    });
    if !(target
        || (value["status"] == "incomplete"
            && value["target_version"].is_null()
            && value["reason"] == "python_syntax_target_unverified"))
        || !(value["version"].is_null() || value["version"] == "ruff 0.16.8")
        || !(value["tool_sha256"].is_null()
            || value["tool_sha256"].as_str().is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            }))
    {
        return false;
    }
    let Some(rows) = value["diagnostics"].as_array().filter(|r| r.len() <= 512) else {
        return false;
    };
    if !rows.iter().all(|row| {
        row.as_object().is_some_and(|r| r.len() == 4)
            && row["rule_id"] == "invalid-syntax"
            && row["column_unit"] == "ruff_reported"
            && row["line"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
            && row["column"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 1024 * 1024 + 1)
            && source.is_none_or(|bytes| {
                bytes.len() <= 1024 * 1024
                    && std::str::from_utf8(bytes).is_ok()
                    && bytes
                        .split(|b| *b == b'\n')
                        .nth(row["line"].as_u64().unwrap_or(0).saturating_sub(1) as usize)
                        .is_some_and(|line| {
                            row["column"].as_u64().unwrap_or(0) <= line.len() as u64 + 1
                        })
            })
    }) {
        return false;
    }
    match value["status"].as_str() {
        Some("completed" | "diagnostics_observed") => {
            value["version"] == "ruff 0.16.8"
                && value["tool_sha256"].is_string()
                && target
                && if value["status"] == "completed" {
                    rows.is_empty() && value["reason"] == "python_native_syntax_no_diagnostics"
                } else {
                    !rows.is_empty() && value["reason"] == "python_native_syntax_diagnostics"
                }
        }
        Some("incomplete") => matches!(
            value["reason"].as_str(),
            Some(
                "python_syntax_tool_unavailable"
                    | "python_syntax_target_unverified"
                    | "python_syntax_context_unverified"
                    | "python_syntax_tool_changed"
                    | "python_syntax_version_unverified"
                    | "python_syntax_execution_incomplete"
                    | "python_syntax_unexpected_stderr"
                    | "python_syntax_report_invalid"
            )
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_syntax;
    use serde_json::json;
    #[test]
    fn closed_native_observation_rejects_forgery_and_invalid_empty_source() {
        let clean = json!({"status":"completed","reason":"python_native_syntax_no_diagnostics",
            "version":"ruff 0.16.8","tool_sha256":"a".repeat(64),"target_version":"py312","diagnostics":[]});
        assert!(super::valid_native_observation(&clean, Some(b"x = 1\n")));
        assert!(!super::valid_native_observation(&clean, Some(&[255])));
        assert!(!super::valid_native_observation(
            &clean,
            Some(&vec![b'x'; 1024 * 1024 + 1])
        ));
        for (key, value) in [
            ("approved", json!(true)),
            ("version", json!("ruff 0.1.0")),
            ("target_version", json!("py316")),
            ("reason", json!("python_native_syntax_diagnostics")),
        ] {
            let mut forged = clean.clone();
            forged[key] = value;
            assert!(
                !super::valid_native_observation(&forged, Some(b"x = 1\n")),
                "{key}"
            );
        }
        let mut present = clean;
        present["status"] = json!("diagnostics_observed");
        present["reason"] = json!("python_native_syntax_diagnostics");
        present["diagnostics"] =
            json!([{"line":1,"column":1,"column_unit":"ruff_reported","rule_id":"invalid-syntax"}]);
        assert!(super::valid_native_observation(&present, Some(b"def (\n")));
        present["diagnostics"][0]["line"] = json!(99);
        assert!(!super::valid_native_observation(&present, Some(b"def (\n")));
    }
    #[test]
    fn invalid_target_is_rejected_before_tool_resolution() {
        let tool = std::path::Path::new("/does-not-exist/ruff");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        for target in ["", "py316", "3.12", "py312\n", "--fix"] {
            let report = super::observe_for_target(tool, b"x = 1\n", target, deadline);
            assert_eq!(report["reason"], "python_syntax_target_unverified");
            assert_eq!(report["status"], "incomplete");
            assert!(report["tool_sha256"].is_null());
            assert!(report["target_version"].is_null());
        }
    }
    #[test]
    #[ignore = "requires installed Ruff0.16.8 via CODEGUARD_RUFF_SYNTAX_BIN"]
    fn actual_target_version_changes_native_syntax_classification() {
        let tool = std::path::PathBuf::from(std::env::var("CODEGUARD_RUFF_SYNTAX_BIN").unwrap());
        let source = b"match value:\n    case 1:\n        pass\n";
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        for (target, expected) in [("py39", "diagnostics_observed"), ("py310", "completed")] {
            let report = super::observe_for_target(&tool, source, target, deadline);
            assert_eq!(report["target_version"], target);
            assert_eq!(report["status"], expected, "{report}");
        }
    }
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
            let report = super::observe(
                &tool,
                source,
                deadline,
                &std::sync::atomic::AtomicBool::new(false),
            );
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
        let report = super::observe(
            &alias,
            b"x = 1\n",
            Instant::now() + Duration::from_secs(5),
            &std::sync::atomic::AtomicBool::new(false),
        );
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
        let report = super::observe(
            &tool,
            b"x = 1\n",
            Instant::now() + Duration::from_secs(5),
            &std::sync::atomic::AtomicBool::new(false),
        );
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
