//! Clippy 修复事件的当前输入核对与脱敏投影；不在编辑事件中运行项目编译。
use crate::rust_lint_inputs::RustLintInputs;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Instant};

/// 复检启动前冻结已观察源码、清单、锁及配置；失败保持未完成而不制造源码违规。
pub(crate) fn capture(root: &Path) -> Option<RustLintInputs> {
    let inventory = crate::rust_input_inventory::RustInputInventory::capture(root).ok()?;
    RustLintInputs::capture(root, &inventory.source_files).ok()
}

/// 从当前同任务原生复检报告投影规则与行号；参数为根、原任务、报告、父请求快照与截止时间。
/// 只提供局部 lint 观察，不猜测列单位，不关闭任务或签发覆盖与交付许可。
pub(crate) fn project(
    root: &Path,
    brief: &Value,
    scan: &Value,
    inputs: Option<&RustLintInputs>,
    selected_tool: Option<&Path>,
    deadline: Instant,
) -> Result<Value, &'static str> {
    if !matches!(scan["schema_version"].as_str(), Some("0.2.0" | "0.3.0"))
        || scan["report_type"] != "rust_clippy_local_observation"
        || scan["checker_id"] != "rust.cargo_clippy"
        || scan["language"] != "rust"
        || scan["authority"] != "local_unverified"
        || scan["coverage_proven"] != false
        || scan["delivery_decision"] != "not_evaluated"
        || !scan["local_scan_complete"].is_boolean()
        || !scan["findings"].is_array()
    {
        return Err("verification_report_invalid");
    }
    let tool_current = (scan["local_scan_complete"] != true && scan["tool_sha256"].is_null())
        || selected_tool
            .and_then(|tool| tool.canonicalize().ok())
            .and_then(|path| {
                codeguard_runtime::read_bounded_regular_file(&path, 128 * 1024 * 1024).ok()
            })
            .is_some_and(|bytes| scan["tool_sha256"] == format!("{:x}", Sha256::digest(bytes)));
    let mut current = tool_current
        && Instant::now() < deadline
        && !codeguard_runtime::sigint_cancellation_requested()
        && inputs.is_some_and(|snapshot| snapshot.unchanged(root));
    let mut positions = Vec::new();
    if current && scan["local_scan_complete"] == true && brief["kind"] == "finding" {
        for finding in scan["findings"]
            .as_array()
            .ok_or("verification_report_invalid")?
        {
            if finding["finding_id"] != brief["task_id"]
                || finding["path"] != brief["scope"]
                || finding["rule_id"] != brief["native_rule_id"]
            {
                continue;
            }
            let path = finding["path"]
                .as_str()
                .ok_or("verification_report_invalid")?;
            if !Path::new(path)
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
            {
                return Err("verification_report_invalid");
            }
            let Ok(source) = crate::plain_syntax_source::read_plain_source(&root.join(path)) else {
                current = false;
                break;
            };
            let Ok(source_text) = std::str::from_utf8(&source) else {
                current = false;
                break;
            };
            let line = finding["line"]
                .as_u64()
                .filter(|line| *line > 0 && *line as usize <= source_text.split('\n').count())
                .ok_or("verification_report_invalid")?;
            let rule = finding["rule_id"]
                .as_str()
                .filter(|rule| {
                    rule.strip_prefix("clippy::").is_some_and(|name| {
                        !name.is_empty()
                            && name.len() <= 72
                            && name
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    })
                })
                .ok_or("verification_report_invalid")?;
            if finding["source_sha256"] != format!("{:x}", Sha256::digest(&source)) {
                current = false;
                break;
            }
            positions.push(json!({"line":line,"rule_id":rule}));
            if positions.len() > 32 {
                return Err("verification_output_exceeded");
            }
        }
    }
    current = current
        && Instant::now() < deadline
        && !codeguard_runtime::sigint_cancellation_requested()
        && inputs.is_some_and(|snapshot| snapshot.unchanged(root));
    if !current {
        positions.clear();
    }
    let status = if !current {
        "stale"
    } else if scan["local_scan_complete"] != true {
        "incomplete"
    } else if positions.is_empty() {
        "completed"
    } else {
        "diagnostics_observed"
    };
    Ok(
        json!({"schema_version":"0.8.0","native_confirmation_status":status,
        "native_confirmation_reason":if !current {"clippy_confirmation_inputs_changed"} else if status=="incomplete" {"clippy_confirmation_incomplete"} else if positions.is_empty(){"clippy_confirmation_no_task_diagnostics"}else{"clippy_confirmation_task_diagnostics"},
        "native_column_unit":"unavailable","native_diagnostic_positions":positions}),
    )
}
