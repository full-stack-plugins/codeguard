//! Clang文档局部观察的封闭导入契约；任务为文件/语言标准/原生规则位置组。
use super::{BlockerInput, FindingInput, ReportInput, safe_relative_path, safe_run_id, valid_sha256};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn exact(v: &Value, fields: &[&str]) -> bool {
    v.as_object().is_some_and(|o| {
        o.len() == fields.len() && fields.iter().all(|field| o.contains_key(*field))
    })
}
/// 按原生规则组计算稳定身份；行号与本次源码摘要不参与身份。
pub(crate) fn fingerprint(r: &Value, rule: &str) -> String {
    let mut h = Sha256::new();
    for part in [
        "clang-documentation-rule-group-v1",
        r["path"].as_str().unwrap_or(""),
        r["language"].as_str().unwrap_or(""),
        r["standard"].as_str().unwrap_or(""),
        "clang-documentation-v1",
        rule,
    ] {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part.as_bytes());
    }
    format!("{:x}", h.finalize())
}
/// 返回语言专属检查器身份；调用者须先校验协议。
pub(crate) fn checker(r: &Value) -> &'static str {
    if r["language"] == "c" {
        "c.clang.documentation"
    } else {
        "cpp.clang.documentation"
    }
}
/// 核验封闭观察形状，不把可编辑报告当可信执行授权。
pub(crate) fn valid_shape(r: &Value) -> bool {
    if !exact(
        r,
        &[
            "schema_version",
            "report_type",
            "workspace_binding",
            "workspace_id",
            "run_id",
            "path",
            "language",
            "standard",
            "profile",
            "selected_tool",
            "source_sha256",
            "local_scan_complete",
            "native",
            "authority",
            "coverage_proven",
            "delivery_decision",
        ],
    ) || r["schema_version"] != "0.1.0"
        || r["report_type"] != "clang_documentation_workbench_observation"
        || r["workspace_binding"] != "bound"
        || r["authority"] != "local_unverified"
        || r["coverage_proven"] != false
        || r["delivery_decision"] != "not_evaluated"
        || r["profile"] != "clang-documentation-v1"
        || !matches!(
            (r["language"].as_str(), r["standard"].as_str()),
            (Some("c"), Some("c11")) | (Some("cpp"), Some("c++17"))
        )
        || !r["path"].as_str().is_some_and(safe_relative_path)
        || !r["workspace_id"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control))
        || !r["run_id"].as_str().is_some_and(|s| {
            safe_run_id(s) && {
                let parts: Vec<_> = s.split('-').collect();
                parts.len() == 4
                    && parts[0] == "clangdoc"
                    && parts[1..].iter().all(|p| {
                        !p.is_empty()
                            && p.bytes().all(|b| b.is_ascii_digit())
                            && p.parse::<u128>().is_ok()
                    })
            }
        })
        || !r["selected_tool"].as_str().is_some_and(|s| {
            Path::new(s).is_absolute() && s.len() <= 4096 && !s.chars().any(char::is_control)
        })
        || (!r["source_sha256"].is_null() && !r["source_sha256"].as_str().is_some_and(valid_sha256))
    {
        return false;
    }
    let native = &r["native"];
    if !exact(
        native,
        &["status", "reason", "version", "tool_sha256", "diagnostics"],
    ) || !native["reason"].as_str().is_some_and(super::safe_reason)
        || (!native["tool_sha256"].is_null()
            && !native["tool_sha256"].as_str().is_some_and(valid_sha256))
        || (!native["version"].is_null()
            && native["version"] != "Apple clang version 21.0.0 (clang-2100.3.34.2)")
    {
        return false;
    }
    let Some(rows) = native["diagnostics"]
        .as_array()
        .filter(|rows| rows.len() <= 64)
    else {
        return false;
    };
    if rows.iter().any(|d| {
        !exact(d, &["rule_id", "line", "column_byte", "level"])
            || !d["rule_id"].as_str().is_some_and(|s| {
                s.starts_with("clang.")
                    && s.len() > 6
                    && s.len() <= 134
                    && s[6..]
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            || !d["line"].as_u64().is_some_and(|v| v > 0)
            || !d["column_byte"].as_u64().is_some_and(|v| v > 0)
            || !matches!(d["level"].as_str(), Some("warning" | "error"))
    }) {
        return false;
    }
    let native_complete = match native["status"].as_str() {
        Some("completed") => rows.is_empty() && native["reason"] == "clang_native_no_diagnostics",
        Some("diagnostics_observed") => {
            !rows.is_empty() && native["reason"] == "clang_native_diagnostics"
        }
        Some("incomplete") => false,
        _ => return false,
    };
    if native["status"] != "incomplete"
        && (!native_complete
            || native["version"].is_null()
            || native["tool_sha256"].is_null()
            || r["source_sha256"].is_null())
    {
        return false;
    }
    r["local_scan_complete"] == (native_complete && !rows.iter().any(|d| d["level"] == "error"))
}
/// 核验当前工作区源码；路径跳转、变更和不可读均撤回定位权限。
pub(crate) fn current(root: &Path, r: &Value) -> bool {
    let Some(path) = r["path"].as_str().filter(|s| safe_relative_path(s)) else {
        return false;
    };
    let Ok(snapshot) = codeguard_runtime::SourceSnapshot::capture(
        root,
        [PathBuf::from(path)],
        1,
        1024 * 1024,
        1024 * 1024,
    ) else {
        return false;
    };
    let Some(bytes) = snapshot.files().get(&PathBuf::from(path)) else {
        return false;
    };
    r["source_sha256"] == format!("{:x}", Sha256::digest(bytes))
        && std::str::from_utf8(bytes).is_ok_and(|s| {
            r["native"]["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
                .all(|d| {
                    s.split('\n')
                        .nth(d["line"].as_u64().unwrap_or(0).saturating_sub(1) as usize)
                        .is_some_and(|line| {
                            let col = d["column_byte"].as_u64().unwrap_or(0) as usize;
                            col > 0 && col <= line.len() + 1 && line.is_char_boundary(col - 1)
                        })
                })
        })
}
fn groups(r: &Value) -> BTreeMap<String, u64> {
    let mut groups = BTreeMap::new();
    for row in r["native"]["diagnostics"].as_array().into_iter().flatten() {
        if let Some(rule) = row["rule_id"]
            .as_str()
            .filter(|s| codeguard_adapters::clang_documentation_guidance(s).is_some())
        {
            groups
                .entry(rule.to_owned())
                .and_modify(|line: &mut u64| *line = (*line).min(row["line"].as_u64().unwrap_or(1)))
                .or_insert(row["line"].as_u64().unwrap_or(1));
        }
    }
    groups
}
/// 返回局部发现与环境组身份；用于反馈，不用于质量门禁。
pub(crate) fn task_ids(root: &Path, r: &Value) -> Vec<String> {
    let complete = r["local_scan_complete"] == true && current(root, r);
    let mut ids: Vec<_> = if complete {
        groups(r)
            .keys()
            .map(|rule| format!("CG-{}", &fingerprint(r, rule)[..32]))
            .collect()
    } else {
        Vec::new()
    };
    if !complete
        || r["native"]["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| {
                d["rule_id"].as_str().is_some_and(|rule| {
                    codeguard_adapters::clang_documentation_guidance(rule).is_none()
                })
            })
    {
        ids.push(format!("CG-B-{}", &fingerprint(r, "environment")[..32]));
    }
    ids
}
/// 导入只读观察，按既有事务落盘；不关闭已有事实。
pub(super) fn parse(
    root: &Path,
    workspace_id: &str,
    file: &Path,
    r: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !valid_shape(r) || r["workspace_id"] != workspace_id {
        return Err("clang_documentation_report_invalid");
    }
    let run = r["run_id"].as_str().ok_or("report_run_id_invalid")?;
    if file.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("report_run_id_invalid");
    }
    let path = r["path"].as_str().ok_or("report_path_invalid")?;
    let complete = r["local_scan_complete"] == true && current(root, r);
    let findings = if complete {
        groups(r)
            .into_iter()
            .map(|(rule, line)| {
                let fingerprint = fingerprint(r, &rule);
                FindingInput {
                    checker_id: checker(r).into(),
                    id: format!("CG-{}", &fingerprint[..32]),
                    fingerprint,
                    path: path.into(),
                    source_sha256: r["source_sha256"].as_str().unwrap_or("").into(),
                    rule_id: rule,
                    line,
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut blockers = Vec::new();
    if task_ids(root, r).iter().any(|id| id.starts_with("CG-B-")) {
        let fingerprint = fingerprint(r, "environment");
        let reason = if !current(root, r) {
            "clang_documentation_input_unverified"
        } else if !complete {
            "clang_documentation_native_incomplete"
        } else {
            "clang_documentation_rule_mapping_incomplete"
        };
        blockers.push(BlockerInput {
            checker_id: checker(r).into(),
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: reason.into(),
            diagnostic_reason: r["native"]["reason"].as_str().map(str::to_owned),
            build_root: ".".into(),
            scope: path.into(),
            affected_paths: vec![path.into()],
        });
    }
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers,
        historical_findings: 0,
    })
}
