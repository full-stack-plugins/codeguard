//! Codeguard函数文档结构策略组导入；同文件原上下文归并，不定义单函数身份或原生警告ID。
use super::{FindingInput, ReportInput};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 返回明确Codeguard自有结构规则，不能伪装原Clang警告。
pub(crate) const RULE: &str = "codeguard.documentation.function_structure_required";
/// 规范化既有原生警告封闭部分，仅用于校验，不导入原警告任务。
pub(crate) fn native_packet(r: &Value) -> Value {
    let mut n = r.clone();
    n.as_object_mut().expect("调用前为对象").remove("structure");
    n["report_type"] = Value::from("clang_documentation_workbench_observation");
    n["profile"] = Value::from("clang-documentation-v1");
    n["run_id"] = Value::from(r["run_id"].as_str().unwrap_or("").replacen(
        "clangdocstruct-",
        "clangdoc-",
        1,
    ));
    n
}
/// 结构检查器身份；明确区分原警告检查器。
pub(crate) fn checker(r: &Value) -> &'static str {
    if r["language"] == "c" {
        "c.clang.documentation_structure"
    } else {
        "cpp.clang.documentation_structure"
    }
}
/// 稳定文件策略组摘要，不受函数同名、重载或行号移动影响。
pub(crate) fn fingerprint(r: &Value) -> String {
    let mut h = Sha256::new();
    for part in [
        "clang-documentation-structure-policy-v1",
        r["path"].as_str().unwrap_or(""),
        r["language"].as_str().unwrap_or(""),
        r["standard"].as_str().unwrap_or(""),
        RULE,
    ] {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part.as_bytes());
    }
    format!("{:x}", h.finalize())
}
/// 核验封闭报告，原生警告与结构组件保持独立。
pub(crate) fn valid_shape(r: &Value) -> bool {
    if !r.is_object()
        || r["report_type"] != "clang_documentation_structure_workbench_observation"
        || r["profile"] != "clang-documentation-structure-v1"
        || !r["run_id"]
            .as_str()
            .is_some_and(|s| s.starts_with("clangdocstruct-"))
    {
        return false;
    }
    if !super::c_family_comments_report::valid_shape(&native_packet(r)) {
        return false;
    }
    let s = &r["structure"];
    if !s.as_object().is_some_and(|o| {
        o.len() == 4
            && [
                "status",
                "reason",
                "observation",
                "native_raw_diagnostic_count",
            ]
            .iter()
            .all(|k| o.contains_key(*k))
    }) {
        return false;
    }
    if !s["native_raw_diagnostic_count"].is_null()
        && !s["native_raw_diagnostic_count"].as_u64().is_some_and(|n| {
            n >= r["native"]["diagnostics"].as_array().map_or(0, |v| v.len()) as u64 && n <= 65536
        })
    {
        return false;
    }
    match s["status"].as_str() {
        Some("observed") => {
            s["reason"] == "clang_structure_observed"
                && s["native_raw_diagnostic_count"].is_u64()
                && r["local_scan_complete"] == true
                && codeguard_adapters::valid_clang_documentation_structure(&s["observation"], None)
        }
        Some("incomplete") => {
            s["observation"].is_null()
                && matches!(
                    s["reason"].as_str(),
                    Some(
                        "clang_structure_report_invalid"
                            | "clang_structure_unavailable"
                            | "clang_source_changed"
                            | "request_cancelled"
                            | "clang_execution_incomplete"
                    )
                )
        }
        _ => false,
    }
}
/// 校验当前源码身份、声明定位与组件，未知结构不授权修改。
pub(crate) fn current(root: &Path, r: &Value) -> bool {
    if !valid_shape(r) || !super::c_family_comments_report::current(root, &native_packet(r)) {
        return false;
    }
    let Some(path) = r["path"].as_str() else {
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
    let Some(source) = snapshot.files().get(&PathBuf::from(path)) else {
        return false;
    };
    r["structure"]["status"] == "incomplete"
        || codeguard_adapters::valid_clang_documentation_structure(
            &r["structure"]["observation"],
            Some(source),
        )
}
/// 本次已支持的缺失函数组件列表，未解析声明保持未知。
pub(crate) fn positions(r: &Value) -> Vec<Value> {
    r["structure"]["observation"]["functions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| {
            f["missing_components"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
        })
        .cloned()
        .collect()
}
/// 当前结构缺失只产生一个稳定文件策略任务。
pub(crate) fn task_ids(root: &Path, r: &Value) -> Vec<String> {
    if current(root, r) && r["structure"]["status"] == "observed" && !positions(r).is_empty() {
        vec![format!("CG-{}", &fingerprint(r)[..32])]
    } else {
        Vec::new()
    }
}
/// 导入结构策略组而不导入原警告、不关闭历史事实。
pub(super) fn parse(
    root: &Path,
    workspace_id: &str,
    file: &Path,
    r: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !valid_shape(r) || r["workspace_id"] != workspace_id {
        return Err("clang_structure_report_invalid");
    }
    let run = r["run_id"].as_str().ok_or("report_run_id_invalid")?;
    if file.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("report_run_id_invalid");
    }
    let findings = if task_ids(root, r).is_empty() {
        Vec::new()
    } else {
        let fp = fingerprint(r);
        vec![FindingInput {
            checker_id: checker(r).into(),
            id: format!("CG-{}", &fp[..32]),
            fingerprint: fp,
            path: r["path"].as_str().unwrap_or("").into(),
            source_sha256: r["source_sha256"].as_str().unwrap_or("").into(),
            rule_id: RULE.into(),
            line: positions(r)
                .iter()
                .filter_map(|p| p["line"].as_u64())
                .min()
                .unwrap_or(1),
        }]
    };
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers: Vec::new(),
        historical_findings: 0,
    })
}
