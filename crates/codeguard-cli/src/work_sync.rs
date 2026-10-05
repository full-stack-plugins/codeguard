//! 本地 Ruff 报告到脱敏修复记录的初步同步；不参与质量门禁。

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

use codeguard_adapters::{
    bundled_ruff_rulepack, configured_p3c_rulesets, p3c_rule_in_selected_rulesets,
};
use codeguard_runtime::TaskFileLock;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::java_p3c_command::{has_projectable_findings, render_pom};
use crate::java_p3c_scan::finding_record;
use crate::workspace_refresh::read_workspace_baseline;

const MAX_REPORT_BYTES: u64 = 16 * 1024 * 1024;
mod checkstyle_report;
mod doctor_report;
mod eslint_report;
mod npm_preparation_report;
mod npm_report;
mod python_cve_report;
mod python_syntax_confirmation_report;
mod rust_build_report;
mod rust_cve_report;
mod rustdoc_report;
mod syntax_confirmation_report;
mod task_projection;
static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);

struct Arguments {
    root: PathBuf,
    json: bool,
}

struct FindingInput {
    checker_id: String,
    id: String,
    fingerprint: String,
    path: String,
    source_sha256: String,
    rule_id: String,
    line: u64,
}

struct BlockerInput {
    checker_id: String,
    id: String,
    fingerprint: String,
    reason: String,
    diagnostic_reason: Option<String>,
    build_root: String,
    scope: String,
    affected_paths: Vec<String>,
}

struct ReportInput {
    workspace_id: String,
    run_id: String,
    digest: String,
    findings: Vec<FindingInput>,
    blockers: Vec<BlockerInput>,
    historical_findings: u64,
}

/// 本地报告同步计数；仅描述待办更新，不表示质量认证。
pub struct SyncSummary {
    pub workspace_id: String,
    pub imported_reports: u64,
    pub already_consumed_reports: u64,
    pub new_findings: u64,
    pub new_blockers: u64,
    pub historical_findings: u64,
    pub failed_reports: u64,
    pub restored_task_projections: u64,
}

/// 将当前 CLI 的脱敏报告写入已初始化工作区的本地报告队列。
pub fn save_local_report(root: &Path, report: &Value) -> Result<(), &'static str> {
    if report["workspace_binding"] != "bound" {
        return Ok(());
    }
    let Some(run_id) = report["run_id"].as_str().filter(|id| safe_run_id(id)) else {
        return Err("invalid_run_id");
    };
    let reports = root.join(".codeguard/reports");
    let state = root.join(".codeguard/state");
    if !real_directory(&reports) || !real_directory(&state) {
        return Err("reports_directory_unavailable");
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|_| "report_encoding_failed")?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return Err("report_too_large");
    }
    write_once(&reports.join(format!("{run_id}.json")), &bytes, &state)
        .map_err(|_| "report_write_failed")
}

/// 消费当前工作区所有未导入的局部 Ruff 报告；尚不关闭旧问题或认证交付。
pub fn run(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let root = match parsed.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("项目路径不可读取");
            return ExitCode::from(3);
        }
    };
    let summary = match sync_local_workspace(&root) {
        Ok(summary) => summary,
        Err(reason) => return print_unavailable(parsed.json, reason),
    };
    let mut report = json!({
        "schema_version":"0.2.0",
        "report_type":"work_sync_preview",
        "operation":"work_sync",
        "workspace_id":summary.workspace_id,
        "command_status":if summary.failed_reports == 0 { "partial" } else { "incomplete" },
        "imported_reports":summary.imported_reports,
        "already_consumed_reports":summary.already_consumed_reports,
        "new_findings":summary.new_findings,
        "new_blockers":summary.new_blockers,
        "historical_findings":summary.historical_findings,
        "failed_reports":summary.failed_reports,
        "delivery_decision":"not_evaluated",
        "next_actions":["inspect_findings_and_tasks", "implement_native_reverification_and_full_gate"]
    });
    if summary.restored_task_projections > 0 {
        report["schema_version"] = json!("0.3.0");
        report["restored_task_projections"] = json!(summary.restored_task_projections);
    }
    if parsed.json {
        println!("{report}");
    } else {
        println!(
            "CodeGuard 同步：新增 {} 项问题、{} 项环境阻塞；导入 {} 份报告；失败 {} 份；质量门禁未评估",
            summary.new_findings,
            summary.new_blockers,
            summary.imported_reports,
            summary.failed_reports
        );
        if summary.restored_task_projections > 0 {
            println!(
                "已恢复 {} 份任务投影；原事实和关闭条件保持不变",
                summary.restored_task_projections
            );
        }
    }
    ExitCode::from(3)
}

/// 供已启用的检查入口自动消费本地报告；失败不影响原生检查结果。
pub fn sync_local_workspace(root: &Path) -> Result<SyncSummary, &'static str> {
    let workspace_id = match read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => id.to_owned(),
            None => return Err("workspace_id_unavailable"),
        },
        _ => return Err("workspace_not_initialized"),
    };
    let reports = root.join(".codeguard/reports");
    let findings = root.join(".codeguard/findings");
    let tasks = root.join(".codeguard/tasks");
    let state = root.join(".codeguard/state");
    if ![&reports, &findings, &tasks, &state]
        .iter()
        .all(|path| real_directory(path))
    {
        return Err("workspace_directories_unavailable");
    }
    let _sync_lock = TaskFileLock::acquire(&state.join("work-sync.lock")).map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            "work_sync_busy"
        } else {
            "work_sync_lock_unavailable"
        }
    })?;
    let consumed = state.join("consumed");
    if fs::create_dir(&consumed).is_err() && !real_directory(&consumed) {
        return Err("consumed_state_unavailable");
    }
    let restored_before_import = task_projection::recover_missing(root, true)?;
    let Ok(entries) = fs::read_dir(&reports) else {
        return Err("reports_unreadable");
    };
    let mut paths = Vec::new();
    let mut failures = 0_u64;
    for entry in entries {
        match entry {
            Ok(entry)
                if entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json") =>
            {
                paths.push(entry.path());
            }
            Ok(_) => {}
            Err(_) => failures += 1,
        }
    }
    paths.sort();
    if paths.len() > 1000 {
        return Err("report_queue_limit_exceeded");
    }
    let mut imported = 0_u64;
    let mut skipped = 0_u64;
    let mut new_findings = 0_u64;
    let mut new_blockers = 0_u64;
    let mut historical_findings = 0_u64;
    let mut receipt_error = false;
    for path in paths {
        match import_one(root, &workspace_id, &path, &findings, &tasks, &consumed) {
            Ok((was_imported, created, blocked, historical)) => {
                if was_imported {
                    imported += 1;
                } else {
                    skipped += 1;
                }
                new_findings += created;
                new_blockers += blocked;
                historical_findings += historical;
            }
            Err(reason) => {
                failures += 1;
                if record_import_failure(&path, &state, &workspace_id, reason).is_err() {
                    receipt_error = true;
                }
            }
        }
    }
    if receipt_error {
        return Err("import_failure_receipt_write_failed");
    }
    let restored_task_projections =
        restored_before_import + task_projection::recover_missing(root, false)?;
    Ok(SyncSummary {
        workspace_id,
        imported_reports: imported,
        already_consumed_reports: skipped,
        new_findings,
        new_blockers,
        historical_findings,
        failed_reports: failures,
        restored_task_projections,
    })
}

fn record_import_failure(
    path: &Path,
    state: &Path,
    workspace_id: &str,
    reason: &'static str,
) -> Result<(), &'static str> {
    let run_id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| safe_run_id(stem))
        .ok_or("report_name_invalid")?;
    let bytes = read_bounded_file(path, MAX_REPORT_BYTES)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let directory = state.join("import-failures");
    if fs::create_dir(&directory).is_err() && !real_directory(&directory) {
        return Err("import_failure_state_unavailable");
    }
    let receipt = json!({
        "schema_version":"0.1.0", "record_type":"local_import_failure",
        "workspace_id":workspace_id, "run_id":run_id, "report_sha256":digest,
        "reason":reason, "authority":"local_unverified",
        "delivery_decision":"not_evaluated"
    });
    let encoded = serde_json::to_vec_pretty(&receipt).map_err(|_| "receipt_encoding_failed")?;
    write_once(
        &directory.join(format!("{run_id}-{digest}.json")),
        &encoded,
        state,
    )
    .map_err(|_| "import_failure_receipt_write_failed")
}

/// 从最新已同步的本地 Ruff 报告读取仍存在的精确发现观察；不证明工具或策略获批。
pub(crate) fn latest_current_finding_observation(
    root: &Path,
    finding_id: &str,
) -> Result<Value, &'static str> {
    let workspace_id = read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|baseline| baseline.workspace_id().map(str::to_owned))
        .ok_or("workspace_not_initialized")?;
    let reports = root.join(".codeguard/reports");
    let consumed = root.join(".codeguard/state/consumed");
    if !real_directory(&reports) || !real_directory(&consumed) {
        return Err("report_queue_unavailable");
    }
    let mut count = 0_usize;
    let mut latest: Option<(u128, u32, PathBuf)> = None;
    for entry in fs::read_dir(&reports).map_err(|_| "report_queue_unavailable")? {
        let path = entry.map_err(|_| "report_queue_unavailable")?.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        count += 1;
        if count > 1000 {
            return Err("report_queue_limit_exceeded");
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            return Err("report_name_invalid");
        };
        let Some(rest) = stem.strip_prefix("lint-") else {
            continue;
        };
        let Some((pid, nanos)) = rest.split_once('-') else {
            return Err("report_name_invalid");
        };
        let pid = pid.parse::<u32>().map_err(|_| "report_name_invalid")?;
        let nanos = nanos.parse::<u128>().map_err(|_| "report_name_invalid")?;
        if latest
            .as_ref()
            .is_none_or(|(old_nanos, old_pid, _)| (nanos, pid) > (*old_nanos, *old_pid))
        {
            latest = Some((nanos, pid, path));
        }
    }
    let (_, _, path) = latest.ok_or("no_local_scan_report")?;
    let bytes = read_bounded_file(&path, MAX_REPORT_BYTES)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let report: Value = serde_json::from_slice(&bytes).map_err(|_| "latest_report_invalid")?;
    let parsed = parse_report(root, &workspace_id, &path, &report, digest.clone())
        .map_err(|_| "latest_report_invalid")?;
    let marker = consumed.join(format!("{}.json", parsed.run_id));
    let expected_marker = serde_json::to_vec_pretty(&json!({
        "schema_version":"0.1.0", "workspace_id":workspace_id,
        "run_id":parsed.run_id, "report_sha256":digest
    }))
    .map_err(|_| "latest_report_invalid")?;
    if read_bounded_file(&marker, 4096).ok().as_deref() != Some(expected_marker.as_slice()) {
        return Err("latest_report_not_synced");
    }
    let finding = parsed
        .findings
        .iter()
        .find(|finding| finding.id == finding_id)
        .ok_or("finding_not_in_latest_scan")?;
    let file = report["files"]
        .as_array()
        .and_then(|files| files.iter().find(|file| file["path"] == finding.path))
        .ok_or("latest_report_invalid")?;
    let native_finding = file["findings"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["finding_id"] == finding_id))
        .ok_or("latest_report_invalid")?;
    if matches!(
        report["schema_version"].as_str(),
        Some("0.5.0" | "0.6.0" | "0.7.0" | "0.8.0" | "0.9.0" | "0.18.0")
    ) {
        let config_ref = file["configuration_ref"]
            .as_str()
            .filter(|path| safe_relative_path(path))
            .ok_or("latest_report_invalid")?;
        let config_bytes = read_bounded_file(&root.join(config_ref), 256 * 1024)
            .map_err(|_| "latest_configuration_changed")?;
        if file["config_sha256"] != format!("{:x}", Sha256::digest(config_bytes)) {
            return Err("latest_configuration_changed");
        }
        if !file["config_sha256"].as_str().is_some_and(|sha| {
            crate::ruff_verification_configuration::is_current(root, &finding.path, config_ref, sha)
        }) {
            return Err("latest_configuration_changed");
        }
    }
    if matches!(report["schema_version"].as_str(), Some("0.9.0" | "0.18.0")) {
        if let Some(claimed) = report["adapter_sha256"].as_str() {
            let current = crate::python_lint_command::current_adapter_sha256()
                .ok_or("adapter_binary_unavailable")?;
            if current != claimed {
                return Err("adapter_binary_changed_since_scan");
            }
        }
    }
    Ok(json!({
        "run_id":parsed.run_id, "report_sha256":parsed.digest,
        "workspace_id":workspace_id, "finding_fingerprint":finding.fingerprint,
        "source_sha256":finding.source_sha256, "native_rule_id":finding.rule_id,
        "target":finding.path, "tool_sha256":file["tool_sha256"],
        "adapter_sha256":report["adapter_sha256"],
        "config_sha256":file["config_sha256"],
        "native_tool_version":report["native_tool_version"],
        "codeguard_rule_id":native_finding["codeguard_rule_id"],
        "rulepack_sha256":native_finding["rulepack_sha256"],
        "rulepack_status":native_finding["rulepack_status"],
        "authority":"local_unverified"
    }))
}

/// 读取时核验npm局部观察及当前输入，不能用于证明受批准覆盖。
pub(crate) fn valid_npm_observation(root: &Path, report: &Value) -> bool {
    inspect_npm_observation(root, report).is_ok()
}

/// 保留报告损坏与当前输入过期的区别，供复检消费者选择重试或诊断。
pub(crate) fn inspect_npm_observation(root: &Path, report: &Value) -> Result<(), &'static str> {
    let baseline =
        crate::workspace_refresh::read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let run = report["run_id"]
        .as_str()
        .filter(|r| safe_run_id(r))
        .ok_or("report_run_id_invalid")?;
    npm_report::parse(
        root,
        id,
        &root.join(".codeguard/reports").join(format!("{run}.json")),
        report,
        String::new(),
    )
    .map(|_| ())
}

/// 复检历史只接受与当前工作区和输入严格绑定的 Python CVE 报告。
pub(crate) fn valid_python_cve_observation(root: &Path, report: &Value) -> bool {
    let Some(workspace_id) = read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|baseline| baseline.workspace_id().map(str::to_owned))
    else {
        return false;
    };
    let Some(run_id) = report["run_id"].as_str().filter(|id| safe_run_id(id)) else {
        return false;
    };
    python_cve_report::parse(
        root,
        &workspace_id,
        &root
            .join(".codeguard/reports")
            .join(format!("{run_id}.json")),
        report,
        String::new(),
    )
    .is_ok()
}

fn import_one(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    findings_dir: &Path,
    tasks_dir: &Path,
    consumed_dir: &Path,
) -> Result<(bool, u64, u64, u64), &'static str> {
    let bytes = read_bounded_file(path, MAX_REPORT_BYTES)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "report_invalid_json")?;
    if value["report_type"] == "npm_cve_workbench_observation" {
        codeguard_adapters::parse_unique_json(&bytes)
            .map_err(|_| "npm_report_duplicate_or_invalid_json")?;
    }
    if value["report_type"] == "eslint_preparation_observation" {
        codeguard_adapters::parse_unique_json(&bytes)
            .map_err(|_| "eslint_preparation_duplicate_or_invalid_json")?;
    }
    if (value["report_type"] == "python_lint_feedback" && value["schema_version"] == "0.18.0")
        || matches!(
            value["report_type"].as_str(),
            Some(
                "python_syntax_confirmation_observation"
                    | "syntax_confirmation_observation"
                    | "syntax_task_recheck"
            )
        )
    {
        codeguard_adapters::parse_unique_json(&bytes)
            .map_err(|_| "python_syntax_confirmation_duplicate_or_invalid_json")?;
    }
    // 已消费的历史报告按原字节收据确认，不用当前源码重演历史输入。
    if (value["report_type"] == "python_lint_feedback" && value["schema_version"] == "0.18.0")
        || matches!(
            value["report_type"].as_str(),
            Some(
                "eslint_workbench_observation"
                    | "npm_cve_workbench_observation"
                    | "eslint_task_recheck"
                    | "eslint_preparation_observation"
                    | "java_checkstyle_workbench_observation"
                    | "checkstyle_task_recheck"
                    | "checkstyle_preparation_observation"
                    | "checkstyle_preparation_recheck"
                    | "rust_cve_workbench_observation"
                    | "python_cve_workbench_observation"
                    | "python_syntax_confirmation_observation"
                    | "syntax_confirmation_observation"
                    | "syntax_task_recheck"
            )
        )
    {
        let run = value["run_id"]
            .as_str()
            .filter(|s| safe_run_id(s))
            .ok_or("report_run_id_invalid")?;
        if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
            return Err("report_run_id_invalid");
        }
        let marker = consumed_dir.join(format!("{run}.json"));
        if marker.exists() {
            let expected=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":workspace_id,"run_id":run,"report_sha256":digest})).map_err(|_|"marker_encoding_failed")?;
            return if read_bounded_file(&marker, 4096)? == expected {
                Ok((false, 0, 0, 0))
            } else {
                Err("run_id_digest_conflict")
            };
        }
    }
    let input = parse_report(root, workspace_id, path, &value, digest)?;
    let marker = consumed_dir.join(format!("{}.json", input.run_id));
    let marker_bytes = serde_json::to_vec_pretty(&json!({
        "schema_version":"0.1.0", "workspace_id":workspace_id,
        "run_id":input.run_id, "report_sha256":input.digest
    }))
    .map_err(|_| "marker_encoding_failed")?;
    if marker.exists() {
        return if read_bounded_file(&marker, 4096)? == marker_bytes {
            Ok((false, 0, 0, 0))
        } else {
            Err("run_id_digest_conflict")
        };
    }
    let mut created = 0_u64;
    for finding in &input.findings {
        if persist_finding(
            findings_dir,
            tasks_dir,
            consumed_dir.parent().ok_or("state_missing")?,
            &input,
            finding,
        )? {
            created += 1;
        }
    }
    let mut blocked = 0_u64;
    for blocker in &input.blockers {
        if persist_blocker(
            findings_dir,
            tasks_dir,
            consumed_dir.parent().ok_or("state_missing")?,
            &input,
            blocker,
        )? {
            blocked += 1;
        }
    }
    #[cfg(test)]
    if std::env::var("CODEGUARD_TEST_EXIT_AFTER_RECORDS")
        .ok()
        .as_deref()
        == Some(input.run_id.as_str())
    {
        // 单元测试子进程在事件落盘后退出，生产构建不包含此故障注入入口。
        std::process::exit(97);
    }
    write_once(
        &marker,
        &marker_bytes,
        consumed_dir.parent().ok_or("state_missing")?,
    )
    .map_err(|_| "marker_write_failed")?;
    Ok((true, created, blocked, input.historical_findings))
}

fn parse_report(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if report["report_type"] == "syntax_task_recheck" {
        if !crate::syntax_task_recheck::valid_shape(root, report)
            || report["workspace_id"] != workspace_id
        {
            return Err("syntax_task_recheck_invalid");
        }
        let run = report["run_id"].as_str().ok_or("report_run_id_invalid")?;
        if path.file_stem().and_then(|p| p.to_str()) != Some(run) {
            return Err("report_run_id_invalid");
        }
        return Ok(ReportInput {
            workspace_id: workspace_id.into(),
            run_id: run.into(),
            digest,
            findings: Vec::new(),
            blockers: Vec::new(),
            historical_findings: 0,
        });
    }
    if report["report_type"] == "syntax_confirmation_observation" {
        return syntax_confirmation_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "python_syntax_confirmation_observation" {
        return python_syntax_confirmation_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "python_cve_workbench_observation" {
        return python_cve_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "rust_cve_workbench_observation" {
        return rust_cve_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "rustdoc_task_recheck" {
        return rustdoc_report::parse_recheck(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "rustdoc_local_observation" {
        return rustdoc_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "rust_build_local_observation" {
        return rust_build_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "rust_build_task_recheck" {
        return rust_build_report::parse_recheck(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "eslint_task_recheck" {
        return eslint_report::parse_recheck(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "eslint_preparation_observation" {
        return eslint_report::parse_preparation(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "checkstyle_preparation_recheck" {
        return checkstyle_report::parse_preparation_recheck(
            root,
            workspace_id,
            path,
            report,
            digest,
        );
    }
    if report["report_type"] == "checkstyle_preparation_observation" {
        return checkstyle_report::parse_preparation(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "checkstyle_task_recheck" {
        return checkstyle_report::parse_recheck(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "java_checkstyle_workbench_observation" {
        return checkstyle_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "eslint_workbench_observation" {
        return eslint_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "doctor_observation" {
        return doctor_report::parse(workspace_id, path, report, digest);
    }
    if report["report_type"] == "go_lint_local_feedback" {
        return parse_go_report(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "npm_cve_workbench_observation" {
        return npm_report::parse(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "java_cve_project_probe" {
        return parse_cve_report(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "rust_clippy_local_observation" {
        return parse_rust_report(root, workspace_id, path, report, digest);
    }
    if report["report_type"] == "java_p3c_project_observation" {
        return parse_java_report(root, workspace_id, path, report, digest);
    }
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id))
        .ok_or("report_run_id_invalid")?;
    let report_v05 = report["schema_version"] == "0.5.0";
    let report_v06 = report["schema_version"] == "0.6.0";
    let report_v07 = report["schema_version"] == "0.7.0";
    let report_v08 = report["schema_version"] == "0.8.0";
    let report_v18 = report["schema_version"] == "0.18.0";
    if report_v18
        && (!crate::python_confirmation_recheck::valid_binding(root, report)
            || (report["task_input_stable"] == true
                && !crate::python_confirmation_recheck::inputs_current(root, report)))
    {
        return Err("python_confirmation_recheck_binding_invalid");
    }
    let report_v09 = report["schema_version"] == "0.9.0" || report_v18;
    let has_suppression_audit = report_v08 || report_v09;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run_id)
        || !(report["schema_version"] == "0.4.0"
            || report_v05
            || report_v06
            || report_v07
            || has_suppression_audit)
        || report["report_type"] != "python_lint_feedback"
        || report["operation"] != "lint"
        || report["language"] != "python"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["delivery_decision"] != "not_evaluated"
        || report["command_status"] != "incomplete"
        || report["tool_approval"] != "unverified"
        || ((report_v06 || report_v07 || has_suppression_audit)
            && report["rulepack_approval"] != "unverified")
    {
        return Err("report_identity_invalid");
    }
    if (report_v09
        && (report.get("adapter_sha256").is_none()
            || (!report["adapter_sha256"].is_null()
                && !report["adapter_sha256"].as_str().is_some_and(valid_sha256))))
        || (!report_v09 && report.get("adapter_sha256").is_some())
    {
        return Err("adapter_identity_invalid");
    }
    let files = report["files"].as_array().ok_or("report_files_invalid")?;
    let mut findings = Vec::new();
    let mut historical_findings = 0_u64;
    let mut ids = BTreeSet::new();
    let mut blockers = BTreeMap::<(String, String, String), BTreeSet<String>>::new();
    let configurations = report["checker_configurations"].as_array();
    for file in files {
        let file_path = file["path"]
            .as_str()
            .filter(|path| safe_relative_path(path))
            .ok_or("report_path_invalid")?;
        if file["recheck_cwd"] != root.to_string_lossy().as_ref() {
            return Err("report_root_mismatch");
        }
        if file["recheck_argv"] != json!(["ruff", "check", file_path]) {
            return Err("recheck_scope_mismatch");
        }
        if !matches!(
            file["run_status"].as_str(),
            Some("passed" | "findings" | "suppressed" | "incomplete")
        ) {
            return Err("file_status_invalid");
        }
        if file["run_status"] == "suppressed" && !has_suppression_audit {
            return Err("file_status_invalid");
        }
        if report_v05 || report_v06 || report_v07 || has_suppression_audit {
            let completed = file["run_status"] != "incomplete";
            let valid_artifact_hashes = ["tool_sha256", "config_sha256"].into_iter().all(|field| {
                if completed {
                    file[field].as_str().is_some_and(valid_sha256)
                } else {
                    file[field].is_null()
                }
            });
            if !valid_artifact_hashes {
                return Err("observed_artifact_identity_invalid");
            }
        }
        if report_v07 || has_suppression_audit {
            let complete = file["run_status"] != "incomplete";
            if (complete
                && (report["native_tool_version"] != "ruff 0.16.8"
                    || !valid_rule_settings_observation(&file["rule_settings"])))
                || (!complete && !file["rule_settings"].is_null())
            {
                return Err("rule_settings_observation_invalid");
            }
        }
        if has_suppression_audit {
            let complete = file["run_status"] != "incomplete";
            if (complete && !valid_suppression_audit(&file["suppression_audit"]))
                || (!complete && !file["suppression_audit"].is_null())
            {
                return Err("suppression_audit_invalid");
            }
            if complete {
                let findings = file["findings"].as_array().ok_or("findings_invalid")?;
                let suppressed = file["suppression_audit"]["suppressed_diagnostic_count"]
                    .as_u64()
                    .ok_or("suppression_audit_invalid")?;
                if (file["run_status"] == "passed" && (!findings.is_empty() || suppressed > 0))
                    || (file["run_status"] == "suppressed"
                        && (!findings.is_empty() || suppressed == 0))
                    || (file["run_status"] == "findings" && findings.is_empty())
                {
                    return Err("file_status_evidence_mismatch");
                }
            }
        }
        if file["run_status"] == "incomplete" {
            let reason = file["reason"]
                .as_str()
                .filter(|reason| safe_reason(reason))
                .ok_or("blocker_reason_invalid")?;
            let build_root = configurations
                .into_iter()
                .flatten()
                .filter(|config| config["checker_id"] == "python.ruff")
                .filter_map(|config| config["build_root"].as_str())
                .filter(|root| *root == "." || safe_relative_path(root))
                .filter(|root| *root == "." || file_path.starts_with(&format!("{root}/")))
                .max_by_key(|root| root.len())
                .unwrap_or(".");
            let scope = if file_scoped_reason(reason) {
                file_path
            } else {
                build_root
            };
            blockers
                .entry((reason.into(), build_root.into(), scope.into()))
                .or_default()
                .insert(file_path.into());
            continue;
        }
        if file["run_status"] != "findings" {
            continue;
        }
        let source_sha256 = file["source_sha256"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
            .ok_or("source_identity_invalid")?;
        let items = file["findings"].as_array().ok_or("findings_invalid")?;
        if items.is_empty() {
            return Err("findings_empty");
        }
        let current_matches = read_bounded_file(&root.join(file_path), MAX_REPORT_BYTES)
            .ok()
            .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == source_sha256);
        for item in items {
            let id = item["finding_id"]
                .as_str()
                .filter(|id| safe_finding_id(id))
                .ok_or("finding_id_invalid")?;
            let fingerprint = item["finding_fingerprint"]
                .as_str()
                .filter(|sha| valid_sha256(sha))
                .ok_or("finding_fingerprint_invalid")?;
            if id != format!("CG-{}", &fingerprint[..32]) || item["path"] != file_path {
                return Err("finding_identity_mismatch");
            }
            if !ids.insert(id.to_owned()) {
                return Err("duplicate_finding_id");
            }
            let rule_id = item["rule_id"]
                .as_str()
                .filter(|rule| safe_rule_id(rule))
                .ok_or("finding_rule_invalid")?;
            if report_v06 || report_v07 || has_suppression_audit {
                let pack = bundled_ruff_rulepack().map_err(|_| "bundled_rulepack_invalid")?;
                let tool_version = report["native_tool_version"]
                    .as_str()
                    .ok_or("tool_version_unavailable")?;
                if let Some(mapping) = pack
                    .supports_tool_version(tool_version)
                    .then(|| pack.mapping(rule_id))
                    .flatten()
                {
                    if item["codeguard_rule_id"] != mapping.codeguard_rule_id
                        || item["rulepack_sha256"] != pack.sha256
                        || item["rulepack_status"] != "candidate_unapproved"
                    {
                        return Err("rulepack_observation_invalid");
                    }
                } else if !item["codeguard_rule_id"].is_null()
                    || !item["rulepack_sha256"].is_null()
                    || item["rulepack_status"] != "unmapped_or_unvalidated_version"
                {
                    return Err("rulepack_observation_invalid");
                }
            }
            let line = item["line"]
                .as_u64()
                .filter(|line| *line > 0)
                .ok_or("finding_location_invalid")?;
            let finding = FindingInput {
                checker_id: "python.ruff".into(),
                id: id.into(),
                fingerprint: fingerprint.into(),
                path: file_path.into(),
                source_sha256: source_sha256.into(),
                rule_id: rule_id.into(),
                line,
            };
            if current_matches {
                findings.push(finding);
            } else {
                historical_findings += 1;
            }
        }
    }
    let blockers = blockers
        .into_iter()
        .map(|((reason, build_root, scope), paths)| {
            let mut hash = Sha256::new();
            for part in [
                "codeguard-blocker-v1",
                "python.ruff",
                &reason,
                &build_root,
                &scope,
            ] {
                hash.update((part.len() as u64).to_be_bytes());
                hash.update(part.as_bytes());
            }
            let fingerprint = format!("{:x}", hash.finalize());
            BlockerInput {
                checker_id: "python.ruff".into(),
                diagnostic_reason: None,
                id: format!("CG-B-{}", &fingerprint[..32]),
                fingerprint,
                reason,
                build_root,
                scope,
                affected_paths: paths.into_iter().collect(),
            }
        })
        .collect();
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings,
        blockers,
        historical_findings,
    })
}

fn parse_cve_report(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id))
        .ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run_id)
        || report["schema_version"] != "0.3.0"
        || report["operation"] != "check"
        || report["checker_id"] != "java.maven.dependency_check"
        || report["language"] != "java"
        || report["category"] != "cve"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["command_status"] != "incomplete"
        || report["delivery_decision"] != "not_evaluated"
        || report["authority"] != "local_unverified"
        || report["tool_approval"] != "unverified"
        || report["rulepack_approval"] != "unverified"
        || report["database_freshness"] != "unverified"
        || report["coverage_proven"] != false
    {
        return Err("report_identity_invalid");
    }
    let probes = report["probes"].as_array().ok_or("report_probes_invalid")?;
    let attributions = report["attribution_probes"]
        .as_array()
        .ok_or("report_attribution_invalid")?;
    if probes.is_empty()
        || probes.len() > 1_000
        || probes.len() != attributions.len()
        || report["configured_build_root_count"].as_u64() != Some(probes.len() as u64)
    {
        return Err("report_probe_count_invalid");
    }
    let mut observed = 0_u64;
    let mut blockers = Vec::new();
    let mut unique_roots = BTreeSet::new();
    for (probe, attribution) in probes.iter().zip(attributions) {
        let build_root = probe["build_root"]
            .as_str()
            .filter(|value| *value == "." || safe_relative_path(value))
            .ok_or("report_build_root_invalid")?;
        let config_ref = probe["configuration_ref"]
            .as_str()
            .filter(|value| safe_relative_path(value))
            .ok_or("report_configuration_ref_invalid")?;
        let expected_ref = if build_root == "." {
            "pom.xml".to_owned()
        } else {
            format!("{build_root}/pom.xml")
        };
        if config_ref != expected_ref
            || !unique_roots.insert(build_root)
            || attribution["build_root"] != build_root
            || !attribution["bindings"].is_array()
        {
            return Err("report_probe_identity_invalid");
        }
        let observation = &probe["observation"];
        if observation["checker_id"] != "java.maven.dependency_check"
            || observation["report_type"] != "owasp_maven_probe"
            || observation["database_freshness"] != "unverified"
            || observation["coverage_proven"] != false
            || observation["authority"] != "local_unverified"
            || observation["delivery_decision"] != "not_evaluated"
        {
            return Err("report_native_identity_invalid");
        }
        let native_status = observation["native_status"]
            .as_str()
            .ok_or("report_native_status_invalid")?;
        let reason = match native_status {
            "incomplete" => observation["reason"]
                .as_str()
                .filter(|value| safe_reason(value))
                .ok_or("report_native_reason_invalid")?,
            "findings_observed_untrusted" | "empty_report_unverified" => {
                observed += 1;
                let advisories = observation["advisories"]
                    .as_array()
                    .filter(|items| items.len() <= 10_000)
                    .ok_or("report_native_advisories_invalid")?;
                if (native_status == "findings_observed_untrusted") != !advisories.is_empty() {
                    return Err("report_native_advisory_status_conflict");
                }
                "cve_database_freshness_unverified"
            }
            _ => return Err("report_native_status_invalid"),
        };
        let pom = read_bounded_file(&root.join(config_ref), 4 * 1024 * 1024).ok();
        let config_matches = pom.as_ref().is_some_and(|bytes| {
            observation["native_plan_sha256"] == format!("{:x}", Sha256::digest(bytes))
        });
        let blocker_reason = if native_status != "incomplete" && !config_matches {
            "cve_configuration_changed_after_scan"
        } else {
            reason
        };
        let mut hash = Sha256::new();
        for part in [
            "codeguard-blocker-v1",
            "java.maven.dependency_check",
            blocker_reason,
            build_root,
            build_root,
        ] {
            hash.update((part.len() as u64).to_be_bytes());
            hash.update(part.as_bytes());
        }
        let fingerprint = format!("{:x}", hash.finalize());
        blockers.push(BlockerInput {
            checker_id: "java.maven.dependency_check".into(),
            diagnostic_reason: None,
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: blocker_reason.into(),
            build_root: build_root.into(),
            scope: build_root.into(),
            affected_paths: vec![config_ref.into()],
        });
    }
    if report["observed_report_count"].as_u64() != Some(observed)
        || report["local_probe_complete"] != (observed as usize == probes.len())
    {
        return Err("report_counts_invalid");
    }
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings: Vec::new(),
        blockers,
        historical_findings: 0,
    })
}

fn parse_java_report(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id))
        .ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run_id)
        || report["schema_version"] != "0.2.0"
        || report["operation"] != "lint"
        || report["language"] != "java"
        || report["checker_id"] != "java.maven.p3c"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["command_status"] != "incomplete"
        || report["delivery_decision"] != "not_evaluated"
        || report["tool_approval"] != "unverified"
        || report["rulepack_approval"] != "unverified"
        || report["coverage_proven"] != false
    {
        return Err("report_identity_invalid");
    }
    let files = report["files"].as_array().ok_or("report_files_invalid")?;
    let flat = report["findings"]
        .as_array()
        .ok_or("report_findings_invalid")?;
    if files.is_empty()
        || files.len() > 100_000
        || flat.len() > 100_000
        || report["source_file_count"].as_u64() != Some(files.len() as u64)
    {
        return Err("report_scope_invalid");
    }
    let mut findings = Vec::new();
    let mut historical_findings = 0_u64;
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut occurrences = BTreeMap::<String, u64>::new();
    let mut blocked = BTreeMap::<(String, String), BTreeSet<String>>::new();
    let mut flat_index = 0_usize;
    let mut observed_count = 0_u64;
    let mut raw_count = 0_u64;
    for file in files {
        let relative = file["path"]
            .as_str()
            .filter(|value| safe_relative_path(value))
            .ok_or("report_path_invalid")?;
        if !paths.insert(relative) {
            return Err("report_path_duplicate");
        }
        let build_root = file["build_root"]
            .as_str()
            .filter(|value| *value == "." || safe_relative_path(value))
            .ok_or("report_build_root_invalid")?;
        if build_root != "." && !relative.starts_with(&format!("{build_root}/")) {
            return Err("report_build_root_mismatch");
        }
        let configuration = file["configuration"]
            .as_str()
            .filter(|value| matches!(*value, "configured" | "missing" | "unknown" | "invalid"))
            .ok_or("report_configuration_invalid")?;
        let file_reason = file["reason"]
            .as_str()
            .filter(|value| safe_reason(value))
            .ok_or("report_file_reason_invalid")?;
        let native = &file["observation"];
        let native_returned = file_reason == "native_probe_returned";
        let projection_incomplete = file_reason == "finding_projection_incomplete";
        if (native_returned || projection_incomplete) != native.is_object()
            || ((native_returned || projection_incomplete) && configuration != "configured")
        {
            return Err("report_observation_invalid");
        }
        let mut blocker_reason = file_reason;
        if native_returned || projection_incomplete {
            let config_ref = file["configuration_ref"]
                .as_str()
                .filter(|value| safe_relative_path(value))
                .ok_or("report_configuration_ref_invalid")?;
            let config_sha = file["configuration_sha256"]
                .as_str()
                .filter(|value| valid_sha256(value))
                .ok_or("report_configuration_sha_invalid")?;
            let config_bytes = read_bounded_file(&root.join(config_ref), MAX_REPORT_BYTES).ok();
            let config_matches = config_bytes
                .as_ref()
                .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == config_sha);
            let selected = if config_matches {
                Some(
                    configured_p3c_rulesets(
                        config_bytes
                            .as_deref()
                            .ok_or("report_configuration_unavailable")?,
                    )
                    .ok_or("report_configuration_rulesets_unresolved")?,
                )
            } else {
                None
            };
            if let Some(selected) = selected.as_deref() {
                if native["declared_rulesets"] != json!(selected)
                    || native["native_plan_sha256"]
                        != format!("{:x}", Sha256::digest(render_pom(selected).as_bytes()))
                {
                    return Err("report_ruleset_selection_mismatch");
                }
            }
            if native["report_type"] != "java_p3c_local_feedback"
                || native["path"] != root.join(relative).to_string_lossy().as_ref()
                || native["coverage_proven"] != false
                || native["delivery_decision"] != "not_evaluated"
            {
                return Err("report_native_identity_invalid");
            }
            let status = native["local_status"]
                .as_str()
                .ok_or("report_native_status_invalid")?;
            let reason = native["reason"]
                .as_str()
                .filter(|value| safe_reason(value))
                .ok_or("report_native_reason_invalid")?;
            let diagnostics = native["findings"]
                .as_array()
                .ok_or("report_native_findings_invalid")?;
            raw_count += diagnostics.len() as u64;
            if projection_incomplete {
                blocked
                    .entry((build_root.into(), blocker_reason.into()))
                    .or_default()
                    .insert(relative.into());
                continue;
            }
            let completed = matches!(
                status,
                "findings_observed_untrusted" | "clean_scope_unproven"
            );
            // 旧 0.2 报告只在嵌套观察中保留失败诊断，没有平铺投影；仍按阻塞导入。
            let projected_partial = has_projectable_findings(native)
                && flat
                    .get(flat_index)
                    .is_some_and(|finding| finding["path"] == relative);
            if completed || projected_partial {
                if completed {
                    observed_count += 1;
                }
                let source_sha = native["source_sha256"]
                    .as_str()
                    .filter(|sha| valid_sha256(sha))
                    .ok_or("source_identity_invalid")?;
                let current = read_bounded_file(&root.join(relative), MAX_REPORT_BYTES).ok();
                let current_matches = current
                    .as_ref()
                    .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == source_sha);
                for diagnostic in diagnostics {
                    let rule = diagnostic["rule_id"]
                        .as_str()
                        .filter(|rule| safe_rule_id(rule))
                        .ok_or("finding_rule_invalid")?;
                    if selected.as_deref().is_some_and(|selected| {
                        !diagnostic["ruleset"].as_str().is_some_and(|ruleset| {
                            p3c_rule_in_selected_rulesets(selected, ruleset, rule)
                        })
                    }) {
                        return Err("report_native_rule_outside_configuration");
                    }
                    let line = diagnostic["line"]
                        .as_u64()
                        .filter(|line| *line > 0)
                        .ok_or("finding_location_invalid")?;
                    if diagnostic["path"] != native["path"] {
                        return Err("finding_path_invalid");
                    }
                    let projected = flat.get(flat_index).ok_or("finding_projection_missing")?;
                    flat_index += 1;
                    let id = projected["finding_id"]
                        .as_str()
                        .filter(|id| safe_finding_id(id))
                        .ok_or("finding_id_invalid")?;
                    let fingerprint = projected["finding_fingerprint"]
                        .as_str()
                        .filter(|sha| valid_sha256(sha))
                        .ok_or("finding_fingerprint_invalid")?;
                    if id != format!("CG-{}", &fingerprint[..32])
                        || !ids.insert(id.to_owned())
                        || projected["path"] != relative
                        || projected["rule_id"] != rule
                        || projected["line"] != line
                        || projected["column"] != diagnostic["column"]
                        || projected["source_sha256"] != source_sha
                    {
                        return Err("finding_identity_mismatch");
                    }
                    if current_matches && config_matches {
                        let expected = finding_record(
                            relative,
                            current.as_ref().expect("已匹配当前源码"),
                            diagnostic,
                            &mut occurrences,
                        )
                        .ok_or("finding_projection_invalid")?;
                        if expected != *projected {
                            return Err("finding_projection_mismatch");
                        }
                        findings.push(FindingInput {
                            checker_id: "java.maven.p3c".into(),
                            id: id.into(),
                            fingerprint: fingerprint.into(),
                            path: relative.into(),
                            source_sha256: source_sha.into(),
                            rule_id: rule.into(),
                            line,
                        });
                    } else {
                        historical_findings += 1;
                    }
                }
                if status == "clean_scope_unproven" && !diagnostics.is_empty() {
                    return Err("clean_report_has_findings");
                }
                if !config_matches {
                    blocked
                        .entry((
                            build_root.into(),
                            "p3c_configuration_changed_after_scan".into(),
                        ))
                        .or_default()
                        .insert(relative.into());
                }
                if completed {
                    continue;
                }
            }
            blocker_reason = reason;
        } else if configuration == "configured" && file_reason == "p3c_configuration_not_confirmed"
        {
            return Err("report_configuration_conflict");
        }
        blocked
            .entry((build_root.into(), blocker_reason.into()))
            .or_default()
            .insert(relative.into());
    }
    if flat_index != flat.len()
        || report["observed_file_count"].as_u64() != Some(observed_count)
        || report["finding_count"].as_u64() != Some(raw_count)
        || report["local_observation_complete"] != (observed_count as usize == files.len())
    {
        return Err("report_counts_invalid");
    }
    let blockers = blocked
        .into_iter()
        .map(|((build_root, reason), paths)| {
            let mut hash = Sha256::new();
            for part in [
                "codeguard-blocker-v1",
                "java.maven.p3c",
                &reason,
                &build_root,
                &build_root,
            ] {
                hash.update((part.len() as u64).to_be_bytes());
                hash.update(part.as_bytes());
            }
            let fingerprint = format!("{:x}", hash.finalize());
            BlockerInput {
                checker_id: "java.maven.p3c".into(),
                diagnostic_reason: None,
                id: format!("CG-B-{}", &fingerprint[..32]),
                fingerprint,
                reason,
                scope: build_root.clone(),
                build_root,
                affected_paths: paths.into_iter().collect(),
            }
        })
        .collect();
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings,
        blockers,
        historical_findings,
    })
}

fn parse_go_report(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id))
        .ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run_id)
        || !matches!(
            report["schema_version"].as_str(),
            Some("0.4.0" | "0.5.0" | "0.6.0" | "0.7.0")
        )
        || report["operation"] != "lint"
        || report["language"] != "go"
        || report["checker_id"] != "go.vet"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["command_status"] != "incomplete"
        || report["exit_code"] != 3
        || report["delivery_decision"] != "not_evaluated"
        || report["coverage_proven"] != false
        || report["authority"] != "local_unverified"
        || report["tool_approval"] != "unverified"
        || report["rulepack_approval"] != "unverified"
    {
        return Err("report_identity_invalid");
    }
    if matches!(report["schema_version"].as_str(), Some("0.5.0" | "0.7.0")) {
        let target = &report["task_target"];
        if !target["task_id"].as_str().is_some_and(safe_finding_id)
            || !target["path"].as_str().is_some_and(safe_relative_path)
            || !target["source_sha256"].as_str().is_some_and(valid_sha256)
            || !report["task_input_stable"].is_boolean()
        {
            return Err("task_observation_identity_invalid");
        }
    }
    if matches!(report["schema_version"].as_str(), Some("0.6.0" | "0.7.0")) {
        if !report["source_snapshot_sha256"].is_null()
            && !report["source_snapshot_sha256"]
                .as_str()
                .is_some_and(valid_sha256)
        {
            return Err("source_snapshot_identity_invalid");
        }
        if report["schema_version"] == "0.7.0"
            && (report["task_scope"]["schema_version"] != "0.1.0"
                || !matches!(
                    report["task_scope"]["status"].as_str(),
                    Some("incomplete" | "selected" | "excluded" | "not_selected")
                ))
        {
            return Err("task_scope_observation_invalid");
        }
    }
    let status = report["native_status"]
        .as_str()
        .filter(|value| {
            matches!(
                *value,
                "incomplete" | "findings_observed_unverified" | "clean_observed_unverified"
            )
        })
        .ok_or("report_status_invalid")?;
    let reason = report["reason"]
        .as_str()
        .filter(|reason| safe_reason(reason))
        .ok_or("blocker_reason_invalid")?;
    if status != "incomplete"
        && (report["native_tool_version"] != "go1.23.4"
            || !report["tool_sha256"].as_str().is_some_and(valid_sha256)
            || report["source_file_count"].as_u64().unwrap_or(0) == 0
            || reason != "project_scope_and_policy_unverified")
    {
        return Err("report_status_invalid");
    }
    if status != "incomplete"
        && matches!(report["schema_version"].as_str(), Some("0.6.0" | "0.7.0"))
        && !report["source_snapshot_sha256"]
            .as_str()
            .is_some_and(valid_sha256)
    {
        return Err("source_snapshot_identity_invalid");
    }
    let identities = report["module_identities"]
        .as_object()
        .filter(|items| items.len() <= 64)
        .ok_or("module_identity_invalid")?;
    if status != "incomplete" && !identities.contains_key(".") {
        return Err("module_identity_invalid");
    }
    let mut current_modules = BTreeSet::new();
    for (module, identity) in identities {
        if module != "." && !safe_relative_path(module) {
            return Err("module_path_invalid");
        }
        let manifest_sha = identity["manifest_sha256"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
            .ok_or("manifest_identity_invalid")?;
        let sum = match identity["go_sum_sha256"].as_str() {
            Some(sha) if valid_sha256(sha) => Some(sha),
            None if identity["go_sum_sha256"].is_null() => None,
            _ => return Err("manifest_identity_invalid"),
        };
        let directory = if module == "." {
            root.to_path_buf()
        } else {
            root.join(module)
        };
        let manifest_current = read_bounded_file(&directory.join("go.mod"), MAX_REPORT_BYTES)
            .is_ok_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == manifest_sha);
        let sum_current = match sum {
            Some(expected) => read_bounded_file(&directory.join("go.sum"), MAX_REPORT_BYTES)
                .is_ok_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected),
            None => fs::symlink_metadata(directory.join("go.sum"))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        };
        if manifest_current && sum_current {
            current_modules.insert(module.as_str());
        }
    }
    let modules = report["module_results"]
        .as_array()
        .filter(|items| items.len() <= 64)
        .ok_or("module_results_invalid")?;
    let mut completed = BTreeMap::new();
    let mut module_ids = BTreeSet::new();
    for module in modules {
        let name = module["root"]
            .as_str()
            .filter(|name| *name == "." || safe_relative_path(name))
            .ok_or("module_path_invalid")?;
        if !module_ids.insert(name) {
            return Err("module_identity_conflict");
        }
        let count = module["diagnostic_count"]
            .as_u64()
            .filter(|count| *count <= 100_000)
            .ok_or("module_results_invalid")?;
        match module["status"].as_str() {
            Some("clean_observed_unverified") if count == 0 && identities.contains_key(name) => {
                completed.insert(name, count);
            }
            Some("findings_observed_unverified") if count > 0 && identities.contains_key(name) => {
                completed.insert(name, count);
            }
            Some("incomplete")
                if count == 0 && module["reason"].as_str().is_some_and(safe_reason) => {}
            _ => return Err("module_results_invalid"),
        }
    }
    if report["modules_completed"].as_u64() != Some(completed.len() as u64) {
        return Err("module_results_invalid");
    }
    let items = report["findings"]
        .as_array()
        .filter(|items| items.len() <= 100_000)
        .ok_or("findings_invalid")?;
    if report["native_diagnostic_count"].as_u64() != Some(items.len() as u64)
        || completed.values().sum::<u64>() != items.len() as u64
        || (status == "clean_observed_unverified" && !items.is_empty())
        || (status == "findings_observed_unverified" && items.is_empty())
        || (status != "incomplete"
            && (completed.len() != identities.len()
                || report["module_count"].as_u64() != Some(completed.len() as u64)))
    {
        return Err("report_status_invalid");
    }
    let snapshot_current = report["source_snapshot_sha256"]
        .as_str()
        .is_none_or(|expected| {
            crate::go_lint_command::current_source_snapshot_sha256(root).as_deref()
                == Some(expected)
        });
    let mut findings = Vec::new();
    let mut historical_findings = 0;
    let mut ids = BTreeSet::new();
    let mut counts = BTreeMap::<&str, u64>::new();
    for item in items {
        let module = item["module_root"]
            .as_str()
            .filter(|name| completed.contains_key(name))
            .ok_or("finding_module_invalid")?;
        let target = item["path"]
            .as_str()
            .filter(|path| safe_relative_path(path) && path.ends_with(".go"))
            .ok_or("report_path_invalid")?;
        let owner = identities
            .keys()
            .filter(|name| name.as_str() == "." || target.starts_with(&format!("{name}/")))
            .max_by_key(|name| name.len());
        if owner.map(String::as_str) != Some(module) {
            return Err("finding_module_invalid");
        }
        let id = item["finding_id"]
            .as_str()
            .filter(|id| safe_finding_id(id))
            .ok_or("finding_id_invalid")?;
        let fingerprint = item["finding_fingerprint"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
            .ok_or("finding_fingerprint_invalid")?;
        if id != format!("CG-{}", &fingerprint[..32]) || !ids.insert(id) {
            return Err("finding_identity_mismatch");
        }
        let source_sha = item["source_sha256"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
            .ok_or("source_identity_invalid")?;
        let rule = item["rule_id"]
            .as_str()
            .filter(|rule| safe_rule_id(rule))
            .ok_or("finding_rule_invalid")?;
        let line = item["line"]
            .as_u64()
            .filter(|line| *line > 0)
            .ok_or("finding_location_invalid")?;
        let column = item["column"]
            .as_u64()
            .filter(|column| *column > 0)
            .ok_or("finding_location_invalid")?;
        if !item["native_message_sha256"]
            .as_str()
            .is_some_and(valid_sha256)
            || item["message_status"] != "native_text_private_untrusted"
        {
            return Err("finding_evidence_invalid");
        }
        *counts.entry(module).or_default() += 1;
        let bytes = read_bounded_file(&root.join(target), MAX_REPORT_BYTES).ok();
        if snapshot_current
            && current_modules.contains(module)
            && bytes
                .as_ref()
                .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == source_sha)
        {
            let source = bytes.as_ref().expect("已核对源码");
            if !source
                .split(|byte| *byte == b'\n')
                .nth((line - 1) as usize)
                .is_some_and(|row| !row.trim_ascii().is_empty() && column <= row.len() as u64 + 1)
            {
                return Err("finding_location_invalid");
            }
            findings.push(FindingInput {
                checker_id: "go.vet".into(),
                id: id.into(),
                fingerprint: fingerprint.into(),
                path: target.into(),
                source_sha256: source_sha.into(),
                rule_id: rule.into(),
                line,
            });
        } else {
            historical_findings += 1;
        }
    }
    if completed
        .iter()
        .any(|(name, count)| counts.get(name).copied().unwrap_or(0) != *count)
    {
        return Err("module_results_invalid");
    }
    // 局部扫描完成不证明规则和平台覆盖；环境故障与策略缺口独立于源码任务。
    let blocker_reason = if status == "incomplete" {
        reason
    } else {
        "go_policy_and_coverage_unverified"
    };
    let mut hash = Sha256::new();
    for part in ["codeguard-blocker-v1", "go.vet", blocker_reason, ".", "."] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let fingerprint = format!("{:x}", hash.finalize());
    let blockers = vec![BlockerInput {
        checker_id: "go.vet".into(),
        diagnostic_reason: None,
        id: format!("CG-B-{}", &fingerprint[..32]),
        fingerprint,
        reason: blocker_reason.into(),
        build_root: ".".into(),
        scope: ".".into(),
        affected_paths: vec!["go.mod".into()],
    }];
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings,
        blockers,
        historical_findings,
    })
}

fn parse_rust_report(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id))
        .ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run_id)
        || !matches!(report["schema_version"].as_str(), Some("0.2.0" | "0.3.0"))
        || report["operation"] != "lint"
        || report["language"] != "rust"
        || report["checker_id"] != "rust.cargo_clippy"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["command_status"] != "incomplete"
        || report["delivery_decision"] != "not_evaluated"
        || report["tool_approval"] != "unverified"
        || report["rulepack_approval"] != "unverified"
        || report["scope"] != "default_features_all_targets"
        || report["coverage_proven"] != false
    {
        return Err("report_identity_invalid");
    }
    let completed = report["local_scan_complete"]
        .as_bool()
        .ok_or("report_status_invalid")?;
    let reason = report["reason"]
        .as_str()
        .filter(|reason| safe_reason(reason))
        .ok_or("blocker_reason_invalid")?;
    if (completed && reason != "native_observed_unverified")
        || (!completed && reason == "native_observed_unverified")
        || (completed && !report["tool_sha256"].as_str().is_some_and(valid_sha256))
        || !matches!(
            report["configuration"].as_str(),
            Some("unknown" | "configured")
        )
    {
        return Err("report_status_invalid");
    }
    let manifest_sha = report["manifest_sha256"].as_str();
    if manifest_sha.is_some_and(|sha| !valid_sha256(sha)) {
        return Err("manifest_identity_invalid");
    }
    let items = report["findings"].as_array().ok_or("findings_invalid")?;
    if items.len() > 100_000 {
        return Err("finding_limit_exceeded");
    }
    let mut findings = Vec::new();
    let mut historical_findings = 0_u64;
    let mut ids = BTreeSet::new();
    for item in items {
        let id = item["finding_id"]
            .as_str()
            .filter(|id| safe_finding_id(id))
            .ok_or("finding_id_invalid")?;
        let fingerprint = item["finding_fingerprint"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
            .ok_or("finding_fingerprint_invalid")?;
        let source_sha = item["source_sha256"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
            .ok_or("source_identity_invalid")?;
        let target = item["path"]
            .as_str()
            .filter(|path| safe_relative_path(path))
            .ok_or("report_path_invalid")?;
        let rule = item["rule_id"]
            .as_str()
            .filter(|rule| rule.starts_with("clippy::") && safe_rule_id(rule))
            .ok_or("finding_rule_invalid")?;
        let line = item["line"]
            .as_u64()
            .filter(|line| *line > 0)
            .ok_or("finding_location_invalid")?;
        if id != format!("CG-{}", &fingerprint[..32]) || !ids.insert(id.to_owned()) {
            return Err("finding_identity_mismatch");
        }
        let finding = FindingInput {
            checker_id: "rust.cargo_clippy".into(),
            id: id.into(),
            fingerprint: fingerprint.into(),
            path: target.into(),
            source_sha256: source_sha.into(),
            rule_id: rule.into(),
            line,
        };
        if read_bounded_file(&root.join(target), MAX_REPORT_BYTES)
            .ok()
            .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == source_sha)
        {
            findings.push(finding);
        } else {
            historical_findings += 1;
        }
    }
    let blockers = if completed {
        Vec::new()
    } else {
        let mut hash = Sha256::new();
        for part in [
            "codeguard-blocker-v1",
            "rust.cargo_clippy",
            reason,
            ".",
            ".",
        ] {
            hash.update((part.len() as u64).to_be_bytes());
            hash.update(part.as_bytes());
        }
        let fingerprint = format!("{:x}", hash.finalize());
        vec![BlockerInput {
            checker_id: "rust.cargo_clippy".into(),
            diagnostic_reason: None,
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: reason.into(),
            build_root: ".".into(),
            scope: ".".into(),
            affected_paths: if reason == "cargo_lock_unavailable" {
                vec!["Cargo.toml".into(), "Cargo.lock".into()]
            } else {
                vec!["Cargo.toml".into()]
            },
        }]
    };
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings,
        blockers,
        historical_findings,
    })
}

fn persist_finding(
    findings_dir: &Path,
    tasks_dir: &Path,
    state_dir: &Path,
    report: &ReportInput,
    finding: &FindingInput,
) -> Result<bool, &'static str> {
    let directory = findings_dir.join(&finding.id);
    if fs::create_dir(&directory).is_err() && !real_directory(&directory) {
        return Err("finding_directory_conflict");
    }
    let events = directory.join("events");
    if fs::create_dir(&events).is_err() && !real_directory(&events) {
        return Err("finding_events_conflict");
    }
    let fact_path = directory.join("finding.json");
    let task_path = tasks_dir.join(format!("{}.md", finding.id));
    let first = !fact_path.exists();
    let fact = serde_json::to_vec_pretty(&json!({
        "schema_version":"0.1.0", "kind":"finding", "id":finding.id,
        "workspace_id":report.workspace_id,
        "fingerprint":finding.fingerprint, "checker_id":finding.checker_id,
        "native_rule_id":finding.rule_id, "path":finding.path,
        "first_source_sha256":finding.source_sha256, "first_run_id":report.run_id,
        "first_report_sha256":report.digest, "state":"open",
        "authority":"local_unverified", "delivery_decision":"not_evaluated"
    }))
    .map_err(|_| "finding_encoding_failed")?;
    let write_event = if first {
        let task = render_task(report, finding);
        write_once(&task_path, task.as_bytes(), state_dir).map_err(|_| "task_conflict")?;
        write_once(&fact_path, &fact, state_dir).map_err(|_| "finding_conflict")?;
        true
    } else {
        let old = read_bounded_file(&fact_path, 128 * 1024)?;
        let value: Value = serde_json::from_slice(&old).map_err(|_| "finding_corrupt")?;
        if value["fingerprint"] != finding.fingerprint
            || value["checker_id"] != finding.checker_id
            || value["path"] != finding.path
            || value["workspace_id"] != report.workspace_id
            || value["native_rule_id"] != finding.rule_id
            || value["state"] != "open"
            || !task_path.is_file()
        {
            return Err("finding_identity_conflict");
        }
        if value["first_run_id"] == report.run_id && value["first_report_sha256"] != report.digest {
            return Err("first_report_conflict");
        }
        value["first_run_id"] == report.run_id
            || should_emit_after_verification(&events, &report.run_id)?
    };
    persist_local_finding_observation(state_dir, report, finding)?;
    if !write_event {
        return Ok(false);
    }
    let event = serde_json::to_vec_pretty(&json!({
        "schema_version":"0.1.0", "event":"observed", "finding_id":finding.id,
        "workspace_id":report.workspace_id,
        "run_id":report.run_id, "report_sha256":report.digest,
        "parent_event_id":null, "state_after":"open"
    }))
    .map_err(|_| "event_encoding_failed")?;
    write_once(
        &events.join(format!("{}.json", report.run_id)),
        &event,
        state_dir,
    )
    .map_err(|_| "event_conflict")?;
    Ok(first)
}

fn persist_local_finding_observation(
    state_dir: &Path,
    report: &ReportInput,
    finding: &FindingInput,
) -> Result<(), &'static str> {
    let root = state_dir.join("observations");
    if fs::create_dir(&root).is_err() && !real_directory(&root) {
        return Err("observation_root_unavailable");
    }
    let directory = root.join(&finding.id);
    if fs::create_dir(&directory).is_err() && !real_directory(&directory) {
        return Err("observation_directory_unavailable");
    }
    let bytes = serde_json::to_vec_pretty(&json!({
        "schema_version":"0.1.0", "record_type":"local_finding_observation",
        "finding_id":finding.id, "workspace_id":report.workspace_id,
        "run_id":report.run_id, "report_sha256":report.digest,
        "source_sha256":finding.source_sha256, "line":finding.line,
        "authority":"local_unverified"
    }))
    .map_err(|_| "observation_encoding_failed")?;
    write_once(
        &directory.join(format!("{}.json", report.run_id)),
        &bytes,
        state_dir,
    )
    .map_err(|_| "observation_write_failed")
}

fn should_emit_after_verification(
    events: &Path,
    current_run_id: &str,
) -> Result<bool, &'static str> {
    let mut latest_verify = None;
    let mut latest_observed = None;
    for entry in fs::read_dir(events).map_err(|_| "finding_events_unreadable")? {
        let entry = entry.map_err(|_| "finding_events_unreadable")?;
        let name = entry.file_name();
        let Some(stem) = name.to_str().and_then(|value| value.strip_suffix(".json")) else {
            continue;
        };
        if let Some(run_id) = stem.strip_prefix("verify-") {
            let Some(sequence) = run_sequence(run_id) else {
                return Ok(true);
            };
            latest_verify = Some(latest_verify.map_or(sequence, |old: u128| old.max(sequence)));
        } else if let Some(sequence) = run_sequence(stem) {
            latest_observed = Some(latest_observed.map_or(sequence, |old: u128| old.max(sequence)));
        }
    }
    let Some(verified) = latest_verify else {
        return Ok(false);
    };
    let Some(current) = run_sequence(current_run_id) else {
        return Ok(true);
    };
    Ok(current > verified && latest_observed.is_none_or(|observed| observed <= verified))
}

fn run_sequence(run_id: &str) -> Option<u128> {
    if run_id.starts_with("rustdoc-")
        || run_id.starts_with("cargo-build-")
        || run_id.starts_with("rust-cve-")
    {
        let mut parts = run_id.rsplit('-');
        let last = parts.next()?.parse::<u128>().ok()?;
        return if run_id.starts_with("rust-cve-") {
            parts.next()?.parse::<u128>().ok()
        } else {
            Some(last)
        };
    }
    if run_id.starts_with("checkstyle-") || run_id.starts_with("eslint-") {
        return run_id.rsplit('-').next()?.parse().ok();
    }
    if let Some(value) = run_id
        .strip_prefix("rust-lint-")
        .or_else(|| run_id.strip_prefix("go-lint-"))
        .or_else(|| run_id.strip_prefix("java-p3c-"))
        .or_else(|| run_id.strip_prefix("java-cve-"))
        .or_else(|| run_id.strip_prefix("doctor-"))
    {
        value.rsplit_once('-')?.0.rsplit_once('-')?.1.parse().ok()
    } else {
        run_id
            .strip_prefix("lint-")?
            .rsplit_once('-')?
            .1
            .parse()
            .ok()
    }
}

fn persist_blocker(
    findings_dir: &Path,
    tasks_dir: &Path,
    state_dir: &Path,
    report: &ReportInput,
    blocker: &BlockerInput,
) -> Result<bool, &'static str> {
    let directory = findings_dir.join(&blocker.id);
    if fs::create_dir(&directory).is_err() && !real_directory(&directory) {
        return Err("blocker_directory_conflict");
    }
    let events = directory.join("events");
    if fs::create_dir(&events).is_err() && !real_directory(&events) {
        return Err("blocker_events_conflict");
    }
    let fact_path = directory.join("finding.json");
    let task_path = tasks_dir.join(format!("{}.md", blocker.id));
    let first = !fact_path.exists();
    let fact = serde_json::to_vec_pretty(&diagnostic_metadata(
        json!({
            "schema_version":"0.1.0", "kind":"blocker", "id":blocker.id,
            "workspace_id":report.workspace_id, "fingerprint":blocker.fingerprint,
            "checker_id":blocker.checker_id, "reason_code":blocker.reason,
            "build_root":blocker.build_root, "scope":blocker.scope,
            "first_affected_paths":blocker.affected_paths,
            "first_run_id":report.run_id, "first_report_sha256":report.digest,
            "state":"open", "authority":"local_unverified",
            "delivery_decision":"not_evaluated"
        }),
        "first_diagnostic_reason",
        blocker.diagnostic_reason.as_deref(),
    ))
    .map_err(|_| "blocker_encoding_failed")?;
    let write_event = if first {
        let task = render_blocker_task(report, blocker);
        write_once(&task_path, task.as_bytes(), state_dir).map_err(|_| "blocker_task_conflict")?;
        write_once(&fact_path, &fact, state_dir).map_err(|_| "blocker_fact_conflict")?;
        true
    } else {
        let old = read_bounded_file(&fact_path, 128 * 1024)?;
        let value: Value = serde_json::from_slice(&old).map_err(|_| "blocker_fact_corrupt")?;
        if value["kind"] != "blocker"
            || value["checker_id"] != blocker.checker_id
            || value["fingerprint"] != blocker.fingerprint
            || value["workspace_id"] != report.workspace_id
            || value["reason_code"] != blocker.reason
            || value["build_root"] != blocker.build_root
            || value["scope"] != blocker.scope
            || value["state"] != "open"
            || !task_path.is_file()
        {
            return Err("blocker_identity_conflict");
        }
        if value["first_run_id"] == report.run_id && value["first_report_sha256"] != report.digest {
            return Err("first_report_conflict");
        }
        value["first_run_id"] == report.run_id
            || should_emit_after_verification(&events, &report.run_id)?
    };
    persist_local_blocker_observation(state_dir, report, blocker)?;
    if !write_event {
        return Ok(false);
    }
    let event = serde_json::to_vec_pretty(&diagnostic_metadata(
        json!({
            "schema_version":"0.1.0", "event":"observed", "blocker_id":blocker.id,
            "workspace_id":report.workspace_id, "run_id":report.run_id,
            "report_sha256":report.digest, "reason_code":blocker.reason,
            "affected_paths":blocker.affected_paths, "state_after":"open"
        }),
        "diagnostic_reason",
        blocker.diagnostic_reason.as_deref(),
    ))
    .map_err(|_| "blocker_event_encoding_failed")?;
    write_once(
        &events.join(format!("{}.json", report.run_id)),
        &event,
        state_dir,
    )
    .map_err(|_| "blocker_event_conflict")?;
    Ok(first)
}

fn persist_local_blocker_observation(
    state_dir: &Path,
    report: &ReportInput,
    blocker: &BlockerInput,
) -> Result<(), &'static str> {
    let observations = state_dir.join("observations");
    if fs::create_dir(&observations).is_err() && !real_directory(&observations) {
        return Err("observation_root_unavailable");
    }
    let directory = observations.join(&blocker.id);
    if fs::create_dir(&directory).is_err() && !real_directory(&directory) {
        return Err("observation_directory_unavailable");
    }
    let bytes = serde_json::to_vec_pretty(&diagnostic_metadata(
        json!({
            "schema_version":"0.1.0", "record_type":"local_blocker_observation",
            "blocker_id":blocker.id, "workspace_id":report.workspace_id,
            "run_id":report.run_id, "report_sha256":report.digest,
            "reason_code":blocker.reason, "affected_paths":blocker.affected_paths,
            "authority":"local_unverified"
        }),
        "diagnostic_reason",
        blocker.diagnostic_reason.as_deref(),
    ))
    .map_err(|_| "observation_encoding_failed")?;
    write_once(
        &directory.join(format!("{}.json", report.run_id)),
        &bytes,
        state_dir,
    )
    .map_err(|_| "observation_write_failed")
}

fn render_blocker_task(report: &ReportInput, blocker: &BlockerInput) -> String {
    if blocker.diagnostic_reason.as_deref() == Some("syntax_recovery_incomplete") {
        return format!(
            "# {} 语法检查能力恢复任务\n\n- 问题证据：范围 `{}`；报告 `.codeguard/reports/{}.json`，摘要 `{}`；固定 grammar 的恢复扫描未完成或错误无法定位，恢复节点数组为空；报告保留源码和 grammar 身份，没有可用的源码错误位置。\n- 规则依据：初检完整性与原生确认要求；零恢复不能代表语法通过，本任务不是已确认源码违规。\n- 允许修改范围：适用检查工具、语言版本和 grammar 配置；原生确认前不得修改源码，不关闭检查器，不伪造定位。\n- 修复步骤：核对原报告的语言、版本与已知限制，恢复适用原生 lint/编译器或调查 grammar；确认 adapter 缺失时提出具体能力决策，不重复无依据的源码修补。\n- 复检命令：codeguard task verify {} . --format=json；codeguard next . --format=json 显示当前能力缺口，恢复检查后再对同一范围复扫。\n- 历史尝试：首次 run {}；后续扫描和失败尝试保留在同一任务，正文不代表完整历史。\n- 关闭条件：同一源码范围的有效原生确认、完整覆盖及既有关闭策略均满足；安装、WASM 零恢复或勾选均不能关闭。\n",
            blocker.id, blocker.scope, report.run_id, report.digest, blocker.id, report.run_id
        );
    }
    if blocker.checker_id == "syntax.native_confirmation"
        && blocker.diagnostic_reason.as_deref() == Some("swift_native_first_observation")
    {
        return format!(
            "# {} Swift 原生检查任务\n\n- 问题证据：报告 `.codeguard/reports/{}.json`，摘要 `{}`，当前范围 `{}`；首次来源为原生 compiler，没有 WASM 观察。\n- 规则依据：Apple Swift 6.4 冻结单文件 frontend parse，语法诊断与工具/预算阻塞分别保留；类型与构建尚未检查。\n- 允许修改范围：仅当前原生语法位置对应源码；上下文或工具阻塞先恢复检查环境，不改无关源码。\n- 修复步骤：运行 next 核对最新证据，按语法位置修复或恢复项目上下文，保留历史尝试。\n- 复检命令：codeguard task verify {} . --swift-tool <已核验绝对路径> --format=json。\n- 历史尝试：首次 run {}，后续扫描和复检追加在原任务。\n- 关闭条件：有效原工具复检、正式策略及完整项目覆盖满足；局部零诊断或勾选不自动关闭。\n",
            blocker.id, report.run_id, report.digest, blocker.scope, blocker.id, report.run_id
        );
    }
    if blocker.checker_id == "syntax.native_confirmation"
        && blocker.diagnostic_reason.as_deref() == Some("kotlin_native_first_observation")
    {
        return format!(
            "# {} Kotlin 原生检查任务\n\n- 问题证据：报告 `.codeguard/reports/{}.json`，摘要 `{}`，当前范围 `{}`；首次来源为原生 compiler，没有 WASM 观察。\n- 规则依据：Kotlin/JVM 2.4.10 冻结单文件编译，语法诊断与上下文/环境阻塞分别保留。\n- 允许修改范围：仅当前原生语法位置对应源码；上下文或工具阻塞先恢复检查环境，不改无关源码。\n- 修复步骤：运行 next 核对最新证据，按语法位置修复或恢复项目上下文，保留历史尝试。\n- 复检命令：codeguard task verify {} . --kotlinc-tool <已核验绝对路径> --format=json。\n- 历史尝试：首次 run {}，后续扫描和复检追加在原任务。\n- 关闭条件：有效原工具复检、正式策略及完整项目覆盖满足；局部零诊断或勾选不自动关闭。\n",
            blocker.id, report.run_id, report.digest, blocker.scope, blocker.id, report.run_id
        );
    }
    if blocker.checker_id == "syntax.native_confirmation"
        && blocker.diagnostic_reason.as_deref() == Some("erlang_native_first_observation")
    {
        return format!(
            "# {} Erlang 原生检查发现待处理\n\n- 问题证据：范围 `{}`；报告 `.codeguard/reports/{}.json`，摘要 `{}`，记录当前源码、工具身份和原生诊断或环境阻塞；首次证据不含 WASM 观察。\n- 规则依据：OTP 28 原生 scanner/parser；局部语法诊断与缺工具、版本、预处理阻塞分别处理，不视为完整项目 lint 结论。\n- 允许修改范围：当前原生诊断成立时仅修复该范围源码；环境阻塞仅恢复原工具、版本和项目预处理上下文，不修改无关源码或关闭检查。\n- 修复步骤：先运行 codeguard next . --format=json 核对最新证据和允许动作，再按当前原生位置修复或恢复具体环境；源码或工具改变先复检，旧位置不能沿用。\n- 复检命令：codeguard task verify {} . --erl-tool <next 建议或已核验的绝对路径> --format=json；复用原生工具，不因已有诊断重复安装。\n- 历史尝试：首次 run {}；后续扫描、尝试和复检追加在同一任务，任务正文不是完整历史。\n- 关闭条件：原工具复检、可信项目策略与覆盖及正式关闭流程均满足；局部零诊断、任务勾选或安装完成不能自行关闭。\n",
            blocker.id, blocker.scope, report.run_id, report.digest, blocker.id, report.run_id
        );
    }
    if blocker.checker_id == "syntax.native_confirmation" {
        return format!(
            "# {} 原生语法确认待处理\n\n- 问题证据：范围 `{}`；报告 `.codeguard/reports/{}.json`，摘要 `{}`，含固定 grammar、源码身份和原字节疑似位置。\n- 规则依据：候选 ERROR/MISSING 恢复不是已确认源码违规。\n- 允许修改范围：对应原生工具、版本和适用项目配置；原生确认前不要修改无关源码或关闭检查。\n- 修复步骤：查看原报告语言及已知限制，准备适用 lint/编译器，确认其语法能力和同一源码范围；原生诊断成立后修复，反证进入 grammar 误报调查。\n- 复检命令：codeguard task verify {} . --format=json；先用 codeguard next 查看适用原生 adapter 和工具参数；能力缺口会明确反馈，不能以其它语言的工具替代。\n- 历史尝试：首次 run {}；追加事件和尝试保存于同一任务。\n- 关闭条件：当前输入与适用原生语法能力确认，并满足既有关闭策略；安装、WASM 零恢复或任务勾选均不能关闭。\n",
            blocker.id, blocker.scope, report.run_id, report.digest, blocker.id, report.run_id
        );
    }
    if blocker.checker_id == "python.pip_audit" {
        return format!(
            "# {} Python CVE 检查待处理\n\n- 问题证据：构建根 `{}`；本轮诊断 `{}`；报告摘要 `{}`，首次 run `{}`。原生 advisory 在脱敏本地报告中，数据库身份及时效未核验。\n- 规则依据：pip-audit 原生 advisory、标准 pylock 解析版本归属与可信漏洞源；本地任务没有白名单批准权威。\n- 允许范围：该构建根的 pyproject.toml、标准 pylock、审计工具和依赖版本；不得改动无关源码或关闭检查来消除问题。\n- 修复步骤：先恢复明确的工具、版本和锁输入；核对环境/依赖组及逐 advisory 归属，真实漏洞升级依赖，误报提出精确待审候选。\n- 复检命令：codeguard task verify {} . --pip-audit-tool <已核验绝对路径> --pip-audit-version <已核验版本> --format json。\n- 历史尝试：后续原生观察与复检记录在同一任务；首次任务文字不是完整历史。\n- 关闭条件：原工具复检及可信漏洞源、项目依赖范围和策略覆盖均核验；局部零漏洞或任务勾选不能关闭。\n",
            blocker.id,
            blocker.build_root,
            blocker.diagnostic_reason.as_deref().unwrap_or("unresolved"),
            report.digest,
            report.run_id,
            blocker.id
        );
    }
    if blocker.checker_id == "rust.cargo_audit" {
        return format!(
            "# {} Rust CVE 检查待处理\n\n- 问题证据：本轮诊断 `{}`；报告摘要 `{}`，首次 run `{}`。原生 advisory 留在脱敏本地报告；数据库时效未核验，不将其当作已确认安全结论。\n- 规则依据：cargo-audit 原生 advisory、精确 Cargo.lock 归属及可信漏洞库时效；本地报告没有白名单批准权威。\n- 允许范围：Cargo.toml、Cargo.lock、原生审计工具及离线 RustSec 数据库；按原生 advisory 定位依赖，不修改无关源码或关闭规则。\n- 修复步骤：恢复原工具与锁输入，核对 advisory、解析版本、数据库来源及时效；真实漏洞升级依赖，工具误判进入精确白名单调查。\n- 复检命令：codeguard task verify {} . --cargo-audit-tool <已核验绝对路径> --rustsec-db <已核验绝对目录> --format json。\n- 历史尝试：后续原生观察与复检记录在同一任务；首次任务文字不代表完整历史。\n- 关闭条件：原工具复检及可信数据库、依赖范围和策略覆盖均满足；局部零漏洞、任务勾选和自写白名单不能关闭。\n",
            blocker.id,
            blocker.diagnostic_reason.as_deref().unwrap_or("unresolved"),
            report.digest,
            report.run_id,
            blocker.id
        );
    }
    if blocker.checker_id == "rust.cargo_check" {
        return format!(
            "# {} 构建检查准备任务\n\n- 问题证据：原生构建未完成或输入已失配，原因 `{}`；报告摘要 `{}`。\n- 规则依据：原生编译报告完整性和稳定输入；不是源码违规。\n- 允许范围：Cargo工具、清单/锁、原构建环境及输入身份；不修改无关源码。\n- 修复步骤：恢复显式Cargo、锁定离线依赖和稳定输入，复核原生目标归属。\n- 复检命令：codeguard build rust . --cargo-tool <已核验绝对路径> --format json。\n- 历史尝试：首次run {}，后续观察保留在同一问题；首张任务不是完整历史。\n- 关闭条件：原生受阻检查恢复并完成正式策略和覆盖核验；同步或勾选不能关闭。\n",
            blocker.id, blocker.reason, report.digest, report.run_id
        );
    }
    if blocker.checker_id == "rust.cargo_rustdoc" {
        return format!(
            "# {} 文档检查准备任务\n\n- 阻塞证据：rustdoc 本轮未完成或输入已失配，原因 `{}`；报告摘要 `{}`。\n- 规则依据：原生观察完整性与稳定输入要求；不是源码违规。\n- 允许范围：检查环境、Cargo 清单/锁及身份调查；不得按陈旧文档定位修改无关源码。\n- 修复步骤：恢复显式 Cargo、锁定离线依赖和稳定输入；歧义先核查目标归属。\n- 复检命令：codeguard comments rust . --cargo-tool <已核验绝对路径> --format json；由同一原生 rustdoc 重新检查。\n- 历史尝试：首次 run {}；后续观察保存在同一问题；复检沿同一问题保存，首次任务文字不代表当前历史。\n- 关闭条件：受阻范围完成有效原工具复检与完整策略核验；任务勾选不能关闭。\n",
            blocker.id, blocker.reason, report.digest, report.run_id
        );
    }
    if blocker.checker_id == "node.eslint.preparation"
        && blocker.diagnostic_reason.as_deref() == Some("eslint_syntax_confirmation_needed")
    {
        return format!(
            "# {} TypeScript 语法原生确认任务\n\n- 问题证据：源码范围 `{}` 的候选 WASM 初检尚未验收；脱敏疑似位置、源码及 grammar 摘要在 `.codeguard/reports/{}.json` 的 `syntax_evidence`，报告 SHA-256 `{}`。这些不是已确认源码违规。\n- 规则依据：Tree-sitter 恢复节点只提示疑似语法位置；原生 TypeScript/ESLint 能力和项目原配置仍须确认。\n- 允许范围：核对本源码、对应构建根、原生工具及配置；不得仅凭 WASM 恢复节点修改源码、增加忽略或白名单。\n- 修复步骤：先查看同 run 的脱敏位置，再恢复适用的原生语法检查；若原生反证，保留证据并进入 grammar 误报调查。\n- 复检命令：codeguard lint typescript <原源码> --workspace <原工作区> --node-tool <核验绝对路径> --eslint-entry <核验绝对路径> --eslint-version <核验版本> --config <原配置绝对路径> --cwd <原工作目录绝对路径>。\n- 历史尝试：首次 run `{}`；后续观察、尝试和复检保留在同一任务，首张 Markdown 不是完整历史。\n- 关闭条件：同输入、范围、方言及语法能力的原生复检和正式策略/覆盖核验均满足；安装、WASM 零恢复或任务勾选不能关闭。\n",
            blocker.id, blocker.scope, report.run_id, report.digest, report.run_id
        );
    }
    if blocker.checker_id == "node.eslint.preparation" {
        return format!(
            "# {} ESLint 检查完整性任务\n\n- 问题证据：诊断 `{}`，报告摘要 `{}`。\n- 规则依据：原工具、配置和报告检查完整性；不是已确认源码违规。\n- 允许范围：核对Node、ESLint入口/版本、原配置、parser/插件及报告归属；不修改无关源码。\n- 修复步骤：恢复缺失上下文；若为解析或抑制诊断，先复现并定位根因，不默认添加忽略或白名单。\n- 复检：codeguard lint typescript <原源码> --workspace <原工作区> --node-tool <核验绝对路径> --eslint-entry <核验绝对路径> --eslint-version <核验版本> --config <原配置绝对路径> --cwd <原工作目录绝对路径>。\n- 历史尝试：首次run `{}`，后续诊断保留在同一任务观察中。\n- 关闭条件：本来受阻的原检查恢复并完成正式复检；任务勾选和局部成功不能关闭。\n",
            blocker.id,
            blocker.diagnostic_reason.as_deref().unwrap_or("unknown"),
            report.digest,
            report.run_id
        );
    }
    if blocker.checker_id == "java.checkstyle.preparation" {
        return format!(
            "# {} Checkstyle 准备任务\n\n- 问题证据：局部诊断 `{}`，报告摘要 `{}`。\n- 规则依据：Checkstyle 原生工具及原配置前置；不是源码违规。\n- 允许范围：核对 Java、JAR、原配置及运行条件，不修改无关源码，不自动关闭规则。\n- 修复步骤：先定位缺失或不明的前置，恢复后按原工具复扫；不自动安装、增加源码抑制或白名单。\n- 复检：codeguard lint java <受影响源码> --checker=checkstyle --workspace . --java-tool <核对后的绝对路径> --checkstyle-jar <核对后的 JAR> --config <原配置>。\n- 历史尝试：首次 run `{}`，后续诊断保留在同一任务观察中。\n- 关闭条件：恢复本来受阻的原工具检查，并核对工具/策略/完整覆盖；任务勾选和局部成功不关闭任务。\n\n> 开放的环境准备记录，不能作为交付通过。\n",
            blocker.id,
            blocker.diagnostic_reason.as_deref().unwrap_or("unknown"),
            report.digest,
            report.run_id
        );
    }
    if blocker.checker_id == "python.ruff.doctor" {
        return format!(
            "# {} Ruff 版本环境调查\n\n- 问题证据：诊断 `{}`，报告 `{}`，run `{}`；后续诊断保存在该任务的本地 observations 中。\n- 规则依据：固定原生版本契约；必需前置/批准策略尚未核验，不是源码违规。\n- 允许范围：工具入口、版本、运行权限及批准前置；不得修改无关源码。\n- 修复步骤：查看报告 ruff_version 的具体原因，恢复原生工具或纠正选择，保留失败尝试；版本恢复后核验必需前置及可信来源。\n- 复检 argv：[\"codeguard\",\"doctor\",\".\",\"--ruff-tool\",\"<已核验绝对路径>\"]。\n- 历史尝试：首次 run `{}`；重复观察不创建第二张任务。\n- 关闭条件：当前前置要求/来源及原生证据均核验后才能关闭；局部版本成功、任务勾选或报告导入不关闭任务，不改变质量门禁。\n",
            blocker.id,
            blocker.diagnostic_reason.as_deref().unwrap_or("unresolved"),
            report.digest,
            report.run_id,
            report.run_id
        );
    }
    if blocker.checker_id == "go.vet" {
        return format!(
            "# {} Go 检查环境/策略待处理\n\n- 问题证据：原因 `{}`，报告摘要 `{}`。\n- 规则依据：原生 vet 和独立项目质量策略。\n- 允许范围：检查工具、模块配置、平台与规则覆盖；不得无依据修改源码。\n- 修复步骤：准备 Go 1.23.4，核对模块和原生错误；策略缺口须确认必需规则/平台覆盖。\n- 复检 argv：[\"codeguard\",\"lint\",\"go\",\".\",\"--go-tool\",\"<已核验绝对路径>\"]。\n- 历史尝试：首次 run `{}`。\n- 关闭条件：环境恢复且完整策略经核验；局部零诊断不关闭任务。\n",
            blocker.id, blocker.reason, report.digest, report.run_id
        );
    }

    if blocker.checker_id == "node.npm.audit" {
        return format!(
            "# {} npm CVE检查待处理\n\n- 问题证据：诊断 `{}`；报告摘要 `{}`，首次run `{}`。\n- 规则依据：原生npm审计、锁节点与合格漏洞数据覆盖；不是已确认源码漏洞或白名单批准。\n- 允许范围：构建根 {:?} 的清单、锁文件及审计工具/配置，避免修改无关源码。\n- 修复步骤：先恢复package.json及锁文件的可读普通文件和物理路径，再核对语法、重复字段、scripts类型，并恢复原生调用并核对漏洞源覆盖、时效和组件归属；观察到advisory时按具体锁版本修订依赖。\n- 复检命令：codeguard cve typescript <构建根> --node-tool <绝对路径> --npm-entry <绝对路径> --npm-version <原具体版本> --userconfig <绝对路径> --globalconfig <绝对路径> --registry <已核验源>。\n- 历史尝试：尚无记录；重复扫描更新本任务。\n- 关闭条件：原工具有效复检及漏洞数据覆盖经核验；局部零发现不关闭任务，仍须完整交付门禁。\n",
            blocker.id,
            blocker.diagnostic_reason.as_deref().unwrap_or("unknown"),
            report.digest,
            report.run_id,
            blocker.build_root
        );
    }
    if blocker.checker_id == "java.maven.dependency_check" {
        let root = serde_json::to_string(&blocker.build_root).expect("构建根可编码");
        let recheck = serde_json::to_string(&[
            "codeguard",
            "check",
            "java",
            ".",
            "--maven-tool",
            "<绝对路径>",
            "--java-home",
            "<绝对路径>",
            "--maven-repo",
            "<绝对路径>",
            "--repo-sha256",
            "<固定摘要>",
            "--cve-data-dir",
            "<离线漏洞库目录>",
            "--cve-data-sha256",
            "<固定摘要>",
        ])
        .expect("复检参数可编码");
        return format!(
            "# {} CVE 检查环境/证据待处理\n\n- 阻塞证据：原生 OWASP 局部观察无法证明 CVE 义务完成；原因 `{}`；首次报告摘要 `{}`，run `{}`。报告中即使有 advisory，也尚未核验漏洞库时效与完整依赖归属。\n- 规则依据：CVE 检查必须绑定解析依赖、advisory 与合格漏洞库；本任务不是已确认源码漏洞，也不是误报白名单批准。\n- 允许范围：Maven 构建根 {} 及其检查工具/离线漏洞库；不要反复修改无关源码。\n- 修复步骤：核对 POM、Maven/JDK、离线依赖闭包和漏洞库来源及时效；原生失败先恢复检查环境；观察到 advisory 时再核验组件坐标、制品摘要与风险处置。\n- 复检 argv（占位值替换为已核验绝对路径与摘要）：\n\n    {}\n\n- 历史尝试：尚无记录。\n- 关闭条件：原生检查在相同受控策略下完成复检，依赖与漏洞库时效均获核验；之后仍需独立全量门禁。\n\n> 本地待处理记录，不是安全通过或漏洞确认。\n",
            blocker.id, blocker.reason, report.digest, report.run_id, root, recheck
        );
    }
    if blocker.checker_id == "java.maven.p3c" {
        let root = serde_json::to_string(&blocker.build_root).expect("构建根可编码");
        let paths = serde_json::to_string(&blocker.affected_paths).expect("路径可编码");
        let recheck = serde_json::to_string(&[
            "codeguard",
            "check",
            "all",
            ".",
            "--maven-tool",
            "<绝对路径>",
            "--java-home",
            "<绝对路径>",
            "--maven-repo",
            "<绝对路径>",
            "--repo-sha256",
            "<固定摘要>",
        ])
        .expect("复检参数可编码");
        return format!(
            "# {} 环境/配置待处理\n\n- 阻塞证据：Java/P3C 局部原生检查未完成；原因 `{}`；首次报告摘要 `{}`。\n- 规则依据：原生检查完整性要求；不是源码违规。\n- 允许范围：Maven 构建根 {}；受影响源码 {}。\n- 修复步骤：先核查该模块是否要求 P3C；若要求，恢复插件配置、Maven/JDK 与固定离线依赖闭包；若不要求，提交独立策略决策，不得修改无关源码。\n- 复检 argv（占位值替换为已核验绝对路径与摘要）：\n\n    {}\n\n- 历史尝试：尚无记录；首次 run `{}`。\n- 关闭条件：原检查器对受阻范围完成有效复检；局部报告不等于项目通过。\n\n> 本地待处理记录，不是交付通过证明。\n",
            blocker.id, blocker.reason, report.digest, root, paths, recheck, report.run_id
        );
    }
    if blocker.checker_id == "rust.cargo_clippy" {
        let step = if blocker.reason == "cargo_lock_unavailable" {
            "恢复项目原 Cargo.lock 或按项目依赖流程准备锁文件，再执行锁定离线检查；检查器不隐式生成锁，不修改无关源码。"
        } else {
            "按原因准备原生工具、配置或稳定输入并重跑锁定离线检查；若工具版本或规则不适用，提交策略决策。"
        };
        let recheck = serde_json::to_string(&[
            "cargo",
            "clippy",
            "--locked",
            "--offline",
            "--all-targets",
            "--message-format=json",
        ])
        .expect("复检参数可编码");
        return format!(
            "# {} 环境/配置待处理\n\n- 阻塞证据：Cargo Clippy 本轮未完成；原因 `{}`；首次报告摘要 `{}`。\n- 规则依据：原生检查完整性要求；不是源码违规。\n- 允许范围：项目根；优先恢复 Cargo、Clippy、配置或稳定输入，不得关闭检查器。\n- 修复步骤：{}\n- 复检 argv（项目根执行）：\n\n    {}\n\n- 历史尝试：尚无记录；首次 run `{}`。\n- 关闭条件：原检查器完成同范围复检；若产生发现，应继续处理。\n\n> 本地待处理记录，不是交付通过证明。\n",
            blocker.id, blocker.reason, report.digest, step, recheck, report.run_id
        );
    }
    if blocker.checker_id == "python.ruff" && blocker.reason == "ruff_local_tool_invalid" {
        return format!(
            "# {} Ruff本地环境修复任务\n\n- 问题证据：本地工具观察失败 `{}`，首次报告摘要 `{}`。\n- 规则依据：原生工具入口完整性，不是源码违规。\n- 允许范围：受检根 `.venv/bin/ruff`、普通父目录、执行权限与原工具字节；不修改无关源码。\n- 修复步骤：核对目录链接、损坏入口或执行权限，恢复原本地环境；不删除环境以换用全局工具绕过，确需改变工具选择时明确指定原检查上下文并复核。\n- 复检命令：codeguard task verify {} . --format json；自动发现恢复后的同根原生入口。\n- 历史尝试：首次run `{}`，后续观察和失败尝试保留在同一任务；首张文档不代表完整历史。\n- 关闭条件：原受阻检查恢复并完成有效原工具复检及策略/覆盖核验；仅安装、同步、勾选或局部零发现不能关闭。\n",
            blocker.id, blocker.reason, report.digest, blocker.id, report.run_id
        );
    }
    if blocker.checker_id == "python.ruff" && blocker.reason == "python_syntax_confirmation_needed"
    {
        return format!(
            "# {} Python 语法原生确认任务\n\n- 问题证据：源码范围 `{}` 的候选 WASM 初检尚未验收；脱敏疑似位置、源码及 grammar 摘要在 `.codeguard/reports/{}.json`，报告 SHA-256 `{}`。疑似位置不是已确认源码违规。\n- 规则依据：原始 Tree-sitter ERROR/MISSING 与独立 codeguard.python.required_suite 结构观察分别保留来源；结构观察的规则版本、配置摘要、父节点和零基字节坐标见报告。两者均须用适用的 Python 原生语法能力和项目原配置确认，不能将候选结构规则当作原生违规。\n- 允许范围：只核对本源码、对应构建根、原生工具及配置；不得凭候选观察修改无关源码或增加白名单。\n- 修复步骤：查看同 run 的位置，再恢复原生检查；若原生反证，保留证据并调查 grammar 误报。\n- 复检命令：codeguard task verify {} . --ruff-tool <已核验绝对路径> --format json。\n- 历史尝试：首次 run `{}`；后续候选观察归并到同一任务。\n- 关闭条件：当前输入、范围与语法能力匹配的原生复检和可信策略核验完成；安装工具、任务勾选或后续 WASM 零恢复节点都不能关闭。\n",
            blocker.id, blocker.scope, report.run_id, report.digest, blocker.id, report.run_id
        );
    }
    let step = if blocker.reason == "project_ruff_config_not_found" {
        "确认项目是否要求 Ruff；若要求，按已批准规则配置 Ruff 后复扫；若不要求，修订项目质量策略。"
    } else if blocker.reason.starts_with("ruff_config_") || blocker.reason == "config_changed" {
        "检查 Ruff 配置文件及其来源，修复配置或输入变更后复扫。"
    } else if blocker.reason.starts_with("ruff_tool_") || blocker.reason == "tool_identity_mismatch"
    {
        "检查 Ruff 可执行文件、版本与工具锁，准备匹配的原生工具后复扫。"
    } else if file_scoped_reason(&blocker.reason) {
        "检查目标文件是否可读、属于检查范围且在扫描期间稳定；恢复后复扫。"
    } else {
        "核对原生工具运行记录、配置和检查范围，恢复完整检查后复扫。"
    };
    let scope = serde_json::to_string(&blocker.scope).expect("范围可编码");
    let paths = serde_json::to_string(&blocker.affected_paths).expect("路径可编码");
    let recheck =
        serde_json::to_string(&["codeguard", "lint", "python", "."]).expect("复检参数可编码");
    format!(
        "# {} 环境/配置待处理\n\n- 阻塞证据：Ruff 检查未完成；结构化原因 `{}`；首次报告摘要 `{}`。\n- 规则依据：原生检查完整性要求；不是源码违规，也不是质量通过。\n- 允许范围：构建根 {}；首次受影响文件 {}。优先修复工具、配置或检查环境；不得通过关闭检查器消除阻塞。\n- 修复步骤：{}\n- 复检 argv（项目根执行）：\n\n    {}\n\n- 历史尝试：尚无记录；首次 run `{}`。\n- 关闭条件：原检查器对受阻义务完成有效复检；随后仍须处理新发现的真实问题。\n\n> 此任务是本地待处理记录，不是交付通过证明。\n",
        blocker.id, blocker.reason, report.digest, scope, paths, step, recheck, report.run_id
    )
}

fn render_task(report: &ReportInput, finding: &FindingInput) -> String {
    if finding.checker_id == "rust.cargo_check" {
        return format!(
            "# {} 编译待修复\n\n- 问题证据：原生编译 `{}`，首次行 {}；报告摘要 `{}`。\n- 规则依据：rustc原生编译错误码及对应类型/语言约束；不是Clippy或文档规则。\n- 允许范围：仅目标文件 {}；跨文件影响须先扩展可核验范围。\n- 修复步骤：核对本轮源码和原生错误码，修正类型或符号归属；不关闭检查替代修复。\n- 复检命令：codeguard build rust . --cargo-tool <本轮已核验绝对路径> --format json；类型检查不执行测试。\n- 历史尝试：首次run {}；后续观察保留在同一问题，首张任务不是当前完整历史。\n- 关闭条件：原工具稳定输入下确认问题消失，并完成项目策略/构建组合/测试核验；同步或勾选不能关闭。\n",
            finding.id,
            finding.rule_id,
            finding.line,
            report.digest,
            serde_json::to_string(&finding.path).expect("路径可编码"),
            report.run_id
        );
    }
    if finding.checker_id == "rust.cargo_rustdoc" {
        let step = if finding.rule_id == "missing_docs" {
            "核对公开项；库/模块用 //!，公开项用 ///，按实际实现补充用途和约束。"
        } else {
            "核对文档链接的真实符号、命名空间和作用域，修正链接或将纯文字改用适当格式。"
        };
        return format!(
            "# {} 文档待修复\n\n- 问题证据：原生 rustdoc `{}`，首次行 {}；报告摘要 `{}`。\n- 规则依据：Cargo/rustdoc 原生文档规则；项目策略尚未核验。\n- 允许范围：仅目标文件 {}，不关闭规则或添加抑制来替代修复。\n- 修复步骤：{}\n- 复检命令：项目根运行 codeguard comments rust . --cargo-tool <本轮工具的已核验绝对路径> --format json，原生 argv 为 cargo rustdoc --lib --locked --offline --message-format=json -- --warn missing_docs --warn rustdoc::broken_intra_doc_links。\n- 历史尝试：首次 run {}；后续观察位于同一 finding；复检与尝试记录沿同一问题保存，首次任务文字不代表当前历史。\n- 关闭条件：原工具在稳定输入及有效规则下确认问题消失，并完成策略和覆盖核验；本地同步或手动勾选不能关闭。\n",
            finding.id,
            finding.rule_id,
            finding.line,
            report.digest,
            serde_json::to_string(&finding.path).expect("路径可编码"),
            step,
            report.run_id
        );
    }
    if finding.checker_id == "node.eslint" {
        return format!(
            "# {} ESLint 待修复\n\n- 问题证据：原生规则 {}，首次行号 {}，报告摘要 {}。\n- 规则依据：项目原 ESLint 配置；完整策略/插件覆盖尚未核验。\n- 允许范围：仅源码 {}，不关闭规则、不增加抑制。\n- 修复步骤：核对原生位置与真实语义，按原规则修复；误报走精确裁定。\n- 复检：codeguard lint typescript <目标文件> --workspace <原工作台> --node-tool <原Node> --eslint-entry <原入口> --eslint-version <原版本> --config <原配置> --cwd <原工作目录>。\n- 历史尝试：首次run {}；后续在同一问题的观察事件保留。\n- 关闭条件：同一原工具/配置有效复检及完整策略核验；同步、勾选或零诊断不自行关闭。\n\n> 局部未核验记录，不是门禁通过。\n",
            finding.id,
            serde_json::to_string(&finding.rule_id).unwrap(),
            finding.line,
            report.digest,
            serde_json::to_string(&finding.path).unwrap(),
            report.run_id
        );
    }
    if finding.checker_id == "java.checkstyle" {
        return format!(
            "# {} 待修复\n\n- 问题证据：Checkstyle 原生规则 `{}`，首次行号 {}，报告摘要 `{}`。\n- 规则依据：原配置绑定的 Checkstyle 10.21.4 注释规则；完整项目策略尚未核验。\n- 允许范围：仅目标源码 {}；不得删除规则或修改无关源码。\n- 修复步骤：核对原配置对应检查类与原生位置，按真实类型/方法契约补正文档；误报提出精确候选。\n- 复检：codeguard lint java <目标文件> --checker=checkstyle --workspace . --config <原配置绝对路径> --java-tool <原工具绝对路径> --checkstyle-jar <原 JAR 绝对路径>。\n- 历史尝试：首次 run `{}`；后续观察保存在同一 finding 事件中。\n- 关闭条件：同一原工具与规则完整复检并确认消失；同步或勾选不关闭任务。\n\n> 局部未核验观察，不能作为交付通过。\n",
            finding.id,
            finding.rule_id,
            finding.line,
            report.digest,
            serde_json::to_string(&finding.path).expect("路径可编码"),
            report.run_id
        );
    }
    if finding.checker_id == "go.vet" {
        let path = serde_json::to_string(&finding.path).expect("路径可编码");
        return format!(
            "# {} 待修复\n\n- 问题证据：Go vet 规则 `{}`，首次行号 {}，报告摘要 `{}`。\n- 规则依据：Go 1.23.4 原生 vet；策略与覆盖尚未核验。\n- 允许范围：仅目标源码 {}，不关闭规则或修改无关源码。\n- 修复步骤：核对原生规则和私有诊断，保持语义修复；误报走精确白名单裁定。\n- 复检 argv（项目根执行）：[\"codeguard\",\"lint\",\"go\",\".\",\"--go-tool\",\"<已核验绝对路径>\"]。\n- 历史尝试：首次 run `{}`，后续在事件中保留。\n- 关闭条件：原工具按相同策略完整复检确认消失；本地同步不关闭任务。\n\n> 局部未核验观察，不是交付通过证明。\n",
            finding.id, finding.rule_id, finding.line, report.digest, path, report.run_id
        );
    }

    if finding.checker_id == "java.maven.p3c" {
        let path = serde_json::to_string(&finding.path).expect("路径可编码");
        let recheck = serde_json::to_string(&[
            "codeguard",
            "lint",
            "java",
            finding.path.as_str(),
            "--checker",
            "p3c",
            "--workspace",
            ".",
            "--maven-tool",
            "<绝对路径>",
            "--java-home",
            "<绝对路径>",
            "--maven-repo",
            "<绝对路径>",
            "--repo-sha256",
            "<固定摘要>",
        ])
        .expect("复检参数可编码");
        return format!(
            "# {} 待修复\n\n- 问题证据：原生 P3C/PMD 规则 `{}`，首次行号 {}；报告摘要 `{}`。\n- 规则依据：P3C 2.1.1 本轮已声明的规则子集；工具与规则批准、完整覆盖尚未核验。\n- 允许范围：仅下列项目内源码，不得靠关闭规则或修改无关文件消除诊断。\n\n    {}\n\n- 修复步骤：核对具体原生规则及其语义后修复该文件；若确为误报，提出精确白名单候选并等待独立裁定。\n- 复检 argv（占位值替换为已核验绝对路径与摘要）：\n\n    {}\n\n- 历史尝试：尚无记录；首次 run `{}`。\n- 关闭条件：同一原生规则、工具和范围复检确认该发现消失，完整质量策略另行核验。\n\n> 本地待处理记录，不是交付通过证明。\n",
            finding.id, finding.rule_id, finding.line, report.digest, path, recheck, report.run_id
        );
    }
    if finding.checker_id == "rust.cargo_clippy" {
        let path = serde_json::to_string(&finding.path).expect("路径可编码");
        let recheck = serde_json::to_string(&[
            "cargo",
            "clippy",
            "--locked",
            "--offline",
            "--all-targets",
            "--message-format=json",
        ])
        .expect("复检参数可编码");
        return format!(
            "# {} 待修复\n\n- 问题证据：原生 Clippy 规则 `{}`，首次行号 {}；报告摘要 `{}`。\n- 规则依据：Cargo Clippy `{}`；工具与规则批准尚未核验。\n- 允许范围：仅下列项目内文件，不得借 ignore 或关闭检查器消除问题。\n\n    {}\n\n- 修复步骤：阅读原生规则含义，修复该文件的实际问题；若确为误报，走精确白名单裁定。\n- 复检 argv（项目根执行）：\n\n    {}\n\n- 历史尝试：尚无记录；首次 run `{}`。\n- 关闭条件：原检查器在完整、同策略的复检中确认该发现消失。\n\n> 本地待处理记录，不是交付通过证明。\n",
            finding.id,
            finding.rule_id,
            finding.line,
            report.digest,
            finding.rule_id,
            path,
            recheck,
            report.run_id
        );
    }
    let step = match finding.rule_id.as_str() {
        "F401" => "核对导入是否仍被使用；确认后仅修改该文件的导入。",
        "E501" => "核对项目 Ruff 行长配置，保持语义并重排行内容。",
        "D100" => "确认该公共模块的用途，为模块补充准确的顶层 docstring。",
        "D101" => "确认该公共类的职责，为类补充准确的 docstring。",
        _ => "查阅原生规则和私有诊断，先确认根因再修改。",
    };
    let path = serde_json::to_string(&finding.path).expect("路径可编码");
    let recheck =
        serde_json::to_string(&["ruff", "check", finding.path.as_str()]).expect("复检参数可编码");
    format!(
        "# {} 待修复\n\n- 问题证据：Ruff 规则 `{}`，首次行号 {}；报告摘要 `{}`。\n- 规则依据：原生 Ruff `{}`；规则包与工具批准尚未核验。\n- 允许范围：仅下列项目内文件，不得用 ignore 或关闭检查器代替修复。\n\n    {}\n\n- 修复步骤：{}\n- 复检 argv（项目根执行）：\n\n    {}\n\n- 历史尝试：尚无记录；首次 run `{}`。\n- 关闭条件：原检查器在完整、同策略的复检中确认该发现消失。\n\n> 此任务是本地待处理记录，不是交付通过证明。\n",
        finding.id,
        finding.rule_id,
        finding.line,
        report.digest,
        finding.rule_id,
        path,
        step,
        recheck,
        report.run_id
    )
}

fn read_bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "file_unreadable")?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        return Err("not_bounded_regular_file");
    }
    let bytes = fs::read(path).map_err(|_| "file_unreadable")?;
    if bytes.len() as u64 > limit {
        return Err("file_too_large");
    }
    Ok(bytes)
}

pub(crate) fn write_once(path: &Path, bytes: &[u8], state_dir: &Path) -> Result<(), &'static str> {
    write_once_with_counter(path, bytes, state_dir, &NEXT_WRITE)
}

fn write_once_with_counter(
    path: &Path,
    bytes: &[u8],
    state_dir: &Path,
    counter: &AtomicU64,
) -> Result<(), &'static str> {
    if !real_directory(state_dir) || !path.parent().is_some_and(real_directory) {
        return Err("state_directory_invalid");
    }
    let mut staging = None;
    for _ in 0..32 {
        let temp = state_dir.join(format!(
            "write-{}-{}.tmp",
            std::process::id(),
            counter.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => {
                staging = Some((temp, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err("staging_create_failed"),
        }
    }
    let (temp, mut file) = staging.ok_or("staging_name_exhausted")?;
    let outcome = (|| {
        file.write_all(bytes).map_err(|_| "staging_write_failed")?;
        file.sync_all().map_err(|_| "staging_sync_failed")?;
        match fs::hard_link(&temp, path) {
            Ok(()) => {
                fs::File::open(path.parent().ok_or("target_parent_missing")?)
                    .and_then(|directory| directory.sync_all())
                    .map_err(|_| "target_sync_failed")?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if read_bounded_file(path, 16 * 1024 * 1024)? == bytes {
                    Ok(())
                } else {
                    Err("existing_content_conflict")
                }
            }
            Err(_) => Err("target_create_failed"),
        }
    })();
    drop(file);
    let _ = fs::remove_file(&temp);
    outcome
}

fn real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

fn safe_run_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn safe_finding_id(value: &str) -> bool {
    value.len() == 35
        && value.starts_with("CG-")
        && value[3..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_rule_settings_observation(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 4
        || !value["settings_sha256"].as_str().is_some_and(valid_sha256)
        || !value["per_file_ignores_present"].is_boolean()
        || value["coverage_proven"] != false
    {
        return false;
    }
    let Some(rules) = value["globally_enabled_mapped_rules"].as_array() else {
        return false;
    };
    let mut previous = "";
    for rule in rules {
        let Some(rule) = rule.as_str() else {
            return false;
        };
        if !matches!(rule, "E501" | "F401") || rule <= previous {
            return false;
        }
        previous = rule;
    }
    true
}

fn valid_suppression_audit(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 4
        || !value["audit_sha256"].as_str().is_some_and(valid_sha256)
        || value["scope"] != "source_comments_only"
    {
        return false;
    }
    let Some(count) = value["suppressed_diagnostic_count"].as_u64() else {
        return false;
    };
    if count > 1_000_000 {
        return false;
    }
    let Some(rules) = value["suppressed_rule_ids"].as_array() else {
        return false;
    };
    if rules.len() > 512 || (count == 0) != rules.is_empty() {
        return false;
    }
    let mut previous = "";
    for rule in rules {
        let Some(rule) = rule.as_str() else {
            return false;
        };
        if rule <= previous
            || rule.is_empty()
            || rule.len() > 12
            || !rule
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            return false;
        }
        previous = rule;
    }
    true
}

fn safe_rule_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
}

fn safe_reason(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn file_scoped_reason(reason: &str) -> bool {
    matches!(
        reason,
        "source_changed"
            | "source_unavailable"
            | "source_not_selected_by_ruff"
            | "finding_identity_unavailable"
    )
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains(':')
        && value.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && !segment.chars().any(char::is_control)
        })
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    if args.first().map(String::as_str) != Some("sync") {
        return Err("work 当前仅支持 sync".into());
    }
    let mut root = None;
    let mut json = false;
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--format" {
            index += 1;
            json = parse_format(args.get(index).ok_or("--format 缺少值")?)?;
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json = parse_format(value)?;
        } else if arg.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{arg}"));
        } else {
            root = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json,
    })
}

fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err("--format 仅支持 human/json".into()),
    }
}

fn print_unavailable(json_format: bool, reason: &str) -> ExitCode {
    if json_format {
        println!(
            "{}",
            json!({"schema_version":"0.2.0", "report_type":"work_sync_preview",
            "operation":"work_sync", "command_status":"incomplete", "reason":reason,
            "delivery_decision":"not_evaluated"})
        );
    } else {
        println!("CodeGuard 同步未完成：{reason}；质量门禁未评估");
    }
    ExitCode::from(3)
}

// 仅 doctor 观察附带诊断原因，保留既有扫描记录协议。
fn diagnostic_metadata(mut value: Value, key: &str, reason: Option<&str>) -> Value {
    if let Some(reason) = reason {
        value[key] = reason.into();
    }
    value
}

/// 校验 doctor 观察结构和局部身份，仅用于复检分类，不授予批准。
pub(crate) fn valid_doctor_report(report: &Value) -> bool {
    let (Some(workspace_id), Some(run_id)) =
        (report["workspace_id"].as_str(), report["run_id"].as_str())
    else {
        return false;
    };
    doctor_report::parse(
        workspace_id,
        Path::new(&format!("{run_id}.json")),
        report,
        String::new(),
    )
    .is_ok()
}

/// 复核当前 ESLint 局部报告，仅用于修复指引，不授予来源或门禁权威。
pub(crate) fn validate_eslint_observation(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> bool {
    eslint_report::parse(root, workspace, path, report, digest).is_ok()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::{Command, ExitCode};
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    use super::{sync_local_workspace, write_once_with_counter};

    #[test]
    fn stale_staging_file_from_a_reused_pid_does_not_block_recovery() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-sync-stale-staging-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let state = root.join("state");
        fs::create_dir(&state).unwrap();
        let old_temp = state.join(format!("write-{}-0.tmp", std::process::id()));
        fs::write(&old_temp, b"interrupted-write").unwrap();
        let counter = AtomicU64::new(0);
        let target = root.join("record.json");

        write_once_with_counter(&target, b"fresh-record", &state, &counter).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"fresh-record");
        assert_eq!(fs::read(&old_temp).unwrap(), b"interrupted-write");
        assert_eq!(counter.load(Ordering::Relaxed), 2);
        assert_eq!(fs::read_dir(&state).unwrap().count(), 1);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn exhausted_staging_names_do_not_delete_existing_files() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-sync-staging-exhausted-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let state = root.join("state");
        fs::create_dir(&state).unwrap();
        for sequence in 0..32 {
            fs::write(
                state.join(format!("write-{}-{sequence}.tmp", std::process::id())),
                b"older-data",
            )
            .unwrap();
        }
        let counter = AtomicU64::new(0);
        let target = root.join("record.json");
        assert_eq!(
            write_once_with_counter(&target, b"fresh-record", &state, &counter),
            Err("staging_name_exhausted")
        );
        assert!(!target.exists());
        assert_eq!(fs::read_dir(&state).unwrap().count(), 32);
        assert_eq!(
            fs::read(state.join(format!("write-{}-0.tmp", std::process::id()))).unwrap(),
            b"older-data"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn process_exit_after_event_replays_without_duplicate_records() {
        if let Ok(root) = std::env::var("CODEGUARD_TEST_CHILD_ROOT") {
            let _ = sync_local_workspace(Path::new(&root));
            panic!("导入应在消费标记写入前直接退出");
        }
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-sync-process-exit-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.py"), b"import os\n").unwrap();
        assert_eq!(
            crate::init_command::run(&[
                root.to_string_lossy().into_owned(),
                "--apply".into(),
                "--format=json".into(),
            ]),
            ExitCode::from(3)
        );
        let baseline: Value =
            serde_json::from_slice(&fs::read(root.join(".codeguard/workspace.json")).unwrap())
                .unwrap();
        let fingerprint = "a".repeat(64);
        let source_sha256 = format!(
            "{:x}",
            Sha256::digest(fs::read(root.join("app.py")).unwrap())
        );
        let report = json!({
            "schema_version":"0.4.0", "report_type":"python_lint_feedback",
            "operation":"lint", "language":"python", "run_id":"run-one",
            "workspace_binding":"bound", "workspace_id":baseline["workspace_id"],
            "command_status":"incomplete", "delivery_decision":"not_evaluated",
            "tool_approval":"unverified",
            "files":[{
                "path":"app.py", "source_sha256":source_sha256,
                "run_status":"findings", "recheck_cwd":root,
                "recheck_argv":["ruff","check","app.py"],
                "findings":[{"finding_id":format!("CG-{}", &fingerprint[..32]),
                    "finding_fingerprint":fingerprint, "path":"app.py", "rule_id":"F401", "line":1}]
            }]
        });
        fs::write(
            root.join(".codeguard/reports/run-one.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "work_sync::tests::process_exit_after_event_replays_without_duplicate_records",
            ])
            .env("CODEGUARD_TEST_CHILD_ROOT", &root)
            .env("CODEGUARD_TEST_EXIT_AFTER_RECORDS", "run-one")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(97),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let finding_dir = root.join(format!(".codeguard/findings/CG-{}", "a".repeat(32)));
        let event = finding_dir.join("events/run-one.json");
        let before = fs::read(&event).unwrap();
        let marker = root.join(".codeguard/state/consumed/run-one.json");
        assert!(!marker.exists());

        let recovered = sync_local_workspace(&root).unwrap();
        assert_eq!(recovered.imported_reports, 1);
        assert_eq!(recovered.new_findings, 0);
        assert_eq!(fs::read(&event).unwrap(), before);
        assert_eq!(fs::read_dir(event.parent().unwrap()).unwrap().count(), 1);
        assert!(marker.is_file());
        fs::remove_dir_all(&root).unwrap();
    }
}
