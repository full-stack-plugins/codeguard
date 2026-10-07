//! 明确占位文档的独立文件策略任务导入；不关闭问题或借用结构复检权限。
use super::{FindingInput, ReportInput};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 占位描述自有规则，与原生警告及缺失组件策略保持不同身份。
pub(crate) const RULE: &str = "codeguard.documentation.placeholder_description";

fn structure_packet(report: &Value) -> Value {
    let mut packet = report.clone();
    if let Some(object) = packet.as_object_mut() {
        object.remove("placeholders");
    }
    packet["report_type"] = Value::from("clang_documentation_structure_workbench_observation");
    packet["profile"] = Value::from("clang-documentation-structure-v1");
    packet["run_id"] = Value::from(report["run_id"].as_str().unwrap_or("").replacen(
        "clangdocplaceholder-",
        "clangdocstruct-",
        1,
    ));
    packet
}

/// 校验独立封闭占位包及原生结构关联；不以本地报告证明可信执行。
pub(crate) fn valid_shape(report: &Value) -> bool {
    report.is_object()
        && report["report_type"] == "clang_documentation_placeholder_workbench_observation"
        && report["profile"] == "clang-documentation-placeholder-v1"
        && report["run_id"]
            .as_str()
            .is_some_and(|run| run.starts_with("clangdocplaceholder-"))
        && super::c_family_structure_report::valid_shape(&structure_packet(report))
        && report["structure"]["status"] == "observed"
        && codeguard_adapters::valid_clang_documentation_placeholders(
            &report["placeholders"],
            &report["structure"]["observation"],
            None,
        )
}

fn fingerprint(report: &Value) -> String {
    let mut digest = Sha256::new();
    for part in [
        "clang-documentation-placeholder-policy-v1",
        report["path"].as_str().unwrap_or(""),
        report["language"].as_str().unwrap_or(""),
        report["standard"].as_str().unwrap_or(""),
        RULE,
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

/// 返回当前局部观察对应的稳定文件任务；只供持久化状态投影，不授予修复权限。
pub(crate) fn task_ids(root: &Path, report: &Value) -> Vec<String> {
    if valid_shape(report)
        && super::c_family_structure_report::current(root, &structure_packet(report))
        && report["placeholders"]["positions"]
            .as_array()
            .is_some_and(|positions| !positions.is_empty())
    {
        vec![format!("CG-{}", &fingerprint(report)[..32])]
    } else {
        Vec::new()
    }
}

/// 导入当前源码关联的占位文件任务；位置漂移复用身份，消失仍不关闭历史任务。
pub(super) fn parse(
    root: &Path,
    workspace: &str,
    file: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !valid_shape(report) || report["workspace_id"] != workspace {
        return Err("clang_placeholder_report_invalid");
    }
    let run = report["run_id"].as_str().ok_or("report_run_id_invalid")?;
    if file.file_stem().and_then(|stem| stem.to_str()) != Some(run) {
        return Err("report_run_id_invalid");
    }
    let mut findings = Vec::new();
    if super::c_family_structure_report::current(root, &structure_packet(report)) {
        let path = report["path"]
            .as_str()
            .ok_or("clang_placeholder_report_invalid")?;
        let snapshot = codeguard_runtime::SourceSnapshot::capture(
            root,
            [PathBuf::from(path)],
            1,
            1024 * 1024,
            1024 * 1024,
        )
        .map_err(|_| "clang_placeholder_source_unavailable")?;
        let source = snapshot
            .files()
            .get(&PathBuf::from(path))
            .ok_or("clang_placeholder_source_unavailable")?;
        if !codeguard_adapters::valid_clang_documentation_placeholders(
            &report["placeholders"],
            &report["structure"]["observation"],
            Some(source),
        ) {
            return Err("clang_placeholder_source_mismatch");
        }
        let positions = report["placeholders"]["positions"]
            .as_array()
            .ok_or("clang_placeholder_report_invalid")?;
        if !positions.is_empty() {
            let fp = fingerprint(report);
            findings.push(FindingInput {
                checker_id: if report["language"] == "c" {
                    "c.clang.documentation_placeholder"
                } else {
                    "cpp.clang.documentation_placeholder"
                }
                .into(),
                id: format!("CG-{}", &fp[..32]),
                fingerprint: fp,
                path: path.into(),
                source_sha256: report["source_sha256"].as_str().unwrap_or("").into(),
                rule_id: RULE.into(),
                line: positions
                    .iter()
                    .filter_map(|p| p["line"].as_u64())
                    .min()
                    .unwrap_or(1),
            });
        }
    }
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers: Vec::new(),
        historical_findings: 0,
    })
}
