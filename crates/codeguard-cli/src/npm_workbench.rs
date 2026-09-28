//! npm局部观察持久化；只创建CVE检查完整性任务，不授予漏洞确认或关闭。
use crate::{
    npm_audit_probe_request::NpmAuditProbeRequest,
    npm_audit_probe_result::NpmAuditProbeResult,
    work_sync::{save_local_report, sync_local_workspace},
    workspace_refresh::read_workspace_baseline,
};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
pub(crate) fn connect(
    root: &Path,
    req: &NpmAuditProbeRequest,
    result: &NpmAuditProbeResult,
) -> Value {
    if codeguard_runtime::sigint_cancellation_requested()
        || std::time::Instant::now() >= req.deadline
    {
        return json!({"status":"interrupted_not_persisted"});
    }
    match prepare(root, req, result) {
        Err(reason) => json!({"status":reason}),
        Ok(report) => {
            if codeguard_runtime::sigint_cancellation_requested()
                || std::time::Instant::now() >= req.deadline
            {
                return json!({"status":"interrupted_not_persisted"});
            }
            if let Err(reason) = save_local_report(root, &report) {
                return json!({"status":reason});
            }
            match sync_local_workspace(root) {
                Ok(s) => {
                    json!({"status":if s.failed_reports==0 {"synced_partial"} else {"sync_incomplete"},"new_blockers":s.new_blockers,"failed_reports":s.failed_reports})
                }
                Err(reason) => json!({"status":reason}),
            }
        }
    }
}
pub(crate) fn prepare(
    root: &Path,
    req: &NpmAuditProbeRequest,
    result: &NpmAuditProbeResult,
) -> Result<Value, &'static str> {
    if result.reason == Some("cancelled") || result.reason == Some("deadline") {
        return Err("interrupted_not_persisted");
    }
    if root.canonicalize().ok().as_deref() != Some(root) || !req.cwd.starts_with(root) {
        return Err("workspace_scope_changed");
    }
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let relative = req
        .cwd
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .ok_or("scope_outside_workspace")?;
    let build = if relative.is_empty() { "." } else { relative };
    let mut digests = Vec::new();
    for (file, limit) in [
        ("package.json", 256 * 1024),
        ("package-lock.json", 8 * 1024 * 1024),
    ] {
        let path = req.cwd.join(file);
        let bytes = read_bounded_regular_file(&path, limit).map_err(|_| "npm_input_unavailable")?;
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        if path.canonicalize().ok().as_ref() != Some(&path)
            || req.expected_sha256.get(&path) != Some(&hash)
        {
            return Err("npm_input_changed");
        }
        digests.push(format!("{:x}", Sha256::digest(&bytes)));
    }
    Ok(
        json!({"schema_version":"0.1.0","report_type":"npm_cve_workbench_observation","workspace_binding":"bound","workspace_id":id,"run_id":req.run_id,"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","checker_id":"node.npm.audit","build_root":build,"manifest_sha256":digests[0],"lock_sha256":digests[1],"diagnostic_reason":result.reason.unwrap_or("npm_database_and_policy_unverified"),"component_count":result.parsed.as_ref().map_or(0,|p|p.components.len()),"advisories":crate::npm_observation_projection::project(result)}),
    )
}

/// 冻结缺上下文、锁文件或清单配置损坏时的本地输入观察；不声称启动过原生工具。
/// 参数为物理工作区、构建根和唯一运行身份，返回完整性阻塞观察或具体原因。
pub(crate) fn prepare_preparation(
    root: &Path,
    project: &Path,
    run_id: &str,
) -> Result<Value, &'static str> {
    if root.canonicalize().ok().as_deref() != Some(root)
        || project.canonicalize().ok().as_deref() != Some(project)
        || !project.starts_with(root)
    {
        return Err("workspace_scope_changed");
    }
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let relative = project
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .ok_or("scope_outside_workspace")?;
    let build = if relative.is_empty() { "." } else { relative };
    let (manifest_state, manifest_digest) =
        crate::npm_input_state::observe(&project.join("package.json"), 256 * 1024);
    let (lock_state, lock_digest) =
        crate::npm_input_state::observe(&project.join("package-lock.json"), 8 * 1024 * 1024);
    if manifest_state != "present" || !matches!(lock_state, "present" | "missing") {
        let reason = if manifest_state != "present" {
            format!("npm_manifest_{manifest_state}")
        } else {
            format!("npm_lock_{lock_state}")
        };
        return Ok(
            json!({"schema_version":"0.3.0","report_type":"npm_cve_workbench_observation",
            "workspace_binding":"bound","workspace_id":id,"run_id":run_id,
            "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
            "checker_id":"node.npm.audit","build_root":build,"manifest_state":manifest_state,
            "lock_state":lock_state,"manifest_sha256":manifest_digest,"lock_sha256":lock_digest,
            "diagnostic_reason":reason,"component_count":0,"advisories":[]}),
        );
    }
    // 只把操作系统明确的NotFound记录为缺锁；拒绝链接、目录和不可读文件。
    let missing_lock = matches!(std::fs::symlink_metadata(project.join("package-lock.json")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound);
    let mut digests = Vec::new();
    let mut invalid_manifest_reason = None;
    for (file, limit) in [
        ("package.json", 256 * 1024),
        ("package-lock.json", 8 * 1024 * 1024),
    ] {
        if file == "package-lock.json" && missing_lock {
            digests.push(String::new());
            continue;
        }
        let path = project.join(file);
        if path.canonicalize().ok().as_ref() != Some(&path) {
            return Err("npm_input_path_invalid");
        }
        let bytes = read_bounded_regular_file(&path, limit).map_err(|_| "npm_input_unavailable")?;
        if file == "package.json" {
            let config = codeguard_adapters::inspect_npm_audit_config(&bytes, build, file);
            if matches!(
                config.reason.as_str(),
                "npm_manifest_invalid" | "npm_scripts_invalid"
            ) {
                invalid_manifest_reason = Some(config.reason);
            }
        }
        digests.push(format!("{:x}", Sha256::digest(bytes)));
    }
    let mut report = json!({"schema_version":"0.1.0","report_type":"npm_cve_workbench_observation",
        "workspace_binding":"bound","workspace_id":id,"run_id":run_id,
        "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "checker_id":"node.npm.audit","build_root":build,"manifest_sha256":digests[0],
        "lock_sha256":digests[1],"diagnostic_reason":invalid_manifest_reason.as_deref().unwrap_or("npm_execution_context_missing"),
        "component_count":0,"advisories":[]});
    if missing_lock {
        report["schema_version"] = json!("0.2.0");
        report["lock_state"] = json!("missing");
        report["lock_sha256"] = Value::Null;
        report["diagnostic_reason"] = json!("npm_lock_missing");
    }
    Ok(report)
}
