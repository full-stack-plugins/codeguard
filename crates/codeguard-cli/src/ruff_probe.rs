//! Ruff 单文件原生执行验收服务；只形成局部证据，不签发语言或项目能力。

use codeguard_adapters::{
    RuffParseState, RuffParsed, RuffSettingsObservation, inspect_ruff_config, parse_ruff_json,
    parse_ruff_settings,
};
use codeguard_runtime::{
    NativeVersionRequest, ProcessSpec, Termination, observe_native_version,
    read_bounded_regular_file, run_process_recorded,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 显式固定的单文件 Ruff 试运行输入。
pub struct RuffProbeRequest {
    /// 要读取和复检的绝对普通源文件路径。
    pub source: PathBuf,
    /// 原生 Ruff 二进制的绝对普通文件路径。
    pub tool: PathBuf,
    /// 预期二进制内容 SHA-256。
    pub expected_tool_sha256: [u8; 32],
    /// 预期 `ruff --version` 原文，例如 `ruff 0.16.8`。
    pub expected_version: String,
    /// 调用者预先创建的私有证据目录。
    pub evidence_dir: PathBuf,
    /// 本次试运行的安全日志前缀。
    pub run_id: String,
    /// 版本探测与扫描共用的绝对截止时间。
    pub deadline: Instant,
    /// None 是旧隔离试运行；Some 才尝试项目配置模式。
    pub project_config: Option<RuffConfigBinding>,
}

/// 项目 Ruff 配置身份；绑定路径必须是目标源码的原生最近配置。
pub struct RuffConfigBinding {
    /// 配置文件的绝对普通文件路径。
    pub path: PathBuf,
    /// 执行前后应保持的内容 SHA-256。
    pub expected_sha256: [u8; 32],
}

/// 局部原生证据是否完整；不代表策略判定或项目交付通过。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuffProbeState {
    /// 版本、工具内容、源内容、原生执行、机器报告及私有证据均一致。
    EvidenceComplete,
    /// 任一必要环节未确认，不能借原生退出码放行。
    Incomplete,
}

/// 本轮原生 `--ignore-noqa` 对照发现的注释抑制；不包含原始源码或诊断消息。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuffSuppressionAudit {
    /// 对照报告的字节摘要，原文保留在私有运行日志。
    pub audit_sha256: String,
    /// 正常扫描未报告、忽略源码注释后出现的诊断数。
    pub suppressed_diagnostic_count: usize,
    /// 被注释抑制的原生规则 ID，去重排序。
    pub suppressed_rule_ids: Vec<String>,
    /// 对照只覆盖源码注释；配置、项目策略及目标范围仍需独立核验。
    pub scope: &'static str,
}

/// 试运行证据与解析结果；原生消息仍是私有原文，对外展示前需脱敏。
pub struct RuffProbeResult {
    /// 局部证据完整性。
    pub state: RuffProbeState,
    /// 未完成原因；完整时为 None。
    pub reason: Option<&'static str>,
    /// 可解析的原生诊断；失败时也保留有效部分。
    pub parsed: Option<RuffParsed>,
    /// 与本次原生扫描同工具、同源码和同配置的设置观察；不证明 suppression 缺席。
    pub settings: Option<RuffSettingsObservation>,
    /// 对正常报告与忽略注释的原生报告做一致性对照；仅配置项目提供。
    pub suppression_audit: Option<RuffSuppressionAudit>,
}

/// 通过统一 Rust runtime 调用锁定 Ruff，并复核源文件与工具内容。
#[must_use]
pub fn run_ruff_probe(request: &RuffProbeRequest, cancelled: &AtomicBool) -> RuffProbeResult {
    if !request
        .run_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        || request.run_id.is_empty()
        || !request.source.is_absolute()
        || !request.tool.is_absolute()
    {
        return incomplete("invalid_probe_request", None);
    }
    let Ok(source_before) = read_bounded_regular_file(&request.source, 16 * 1024 * 1024) else {
        return incomplete("source_unavailable", None);
    };
    let source_digest: [u8; 32] = Sha256::digest(&source_before).into();
    if hash_tool(&request.tool) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_mismatch", None);
    }
    if let Some(binding) = &request.project_config {
        if let Err(reason) = verify_project_config(&request.source, binding) {
            return incomplete(reason, None);
        }
    }

    let Some(cwd) = request.source.parent() else {
        return incomplete("source_unavailable", None);
    };
    let version = ProcessSpec {
        executable: request.tool.clone(),
        args: vec!["--version".into()],
        cwd: cwd.to_path_buf(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 4096,
    };
    let version_log = format!("{}-version.log", request.run_id);
    let version_observation = observe_native_version(
        &NativeVersionRequest {
            process: version,
            expected_tool_sha256: request.expected_tool_sha256,
            expected_stdout: format!("{}\n", request.expected_version).into_bytes(),
            evidence_root: request.evidence_dir.clone(),
            log_name: version_log,
        },
        cancelled,
    );
    if !version_observation.complete {
        return incomplete(
            version_observation
                .reason
                .unwrap_or("version_observation_incomplete"),
            None,
        );
    }

    if request.project_config.is_some() {
        let selection = ProcessSpec {
            executable: request.tool.clone(),
            args: vec![
                "check".into(),
                "--show-files".into(),
                request.source.as_os_str().to_os_string(),
            ],
            cwd: cwd.to_path_buf(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: request.deadline,
            output_limit_bytes: 4096,
        };
        let selection_log = format!("{}-selection.log", request.run_id);
        let selected = match run_process_recorded(
            &selection,
            cancelled,
            &request.evidence_dir,
            &selection_log,
        ) {
            Ok(outcome) => outcome,
            Err(failure) => return incomplete(failure.kind.reason(), None),
        };
        let expected = request.source.to_str();
        if selected.termination != Termination::Exited(0)
            || !selected.stderr.is_empty()
            || selected.stdout.strip_suffix(b"\n") != expected.map(str::as_bytes)
        {
            return incomplete("source_not_selected_by_ruff", None);
        }
    }

    let settings = if request.project_config.is_some() {
        if request.expected_version != "ruff 0.16.8" {
            return incomplete("ruff_settings_version_unvalidated", None);
        }
        let settings_spec = ProcessSpec {
            executable: request.tool.clone(),
            args: vec![
                "check".into(),
                "--show-settings".into(),
                request.source.as_os_str().to_os_string(),
            ],
            cwd: cwd.to_path_buf(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: request.deadline,
            output_limit_bytes: 2 * 1024 * 1024,
        };
        let log = format!("{}-settings.log", request.run_id);
        let observed =
            match run_process_recorded(&settings_spec, cancelled, &request.evidence_dir, &log) {
                Ok(observed) => observed,
                Err(failure) => return incomplete(failure.kind.reason(), None),
            };
        if observed.termination != Termination::Exited(0) || !observed.stderr.is_empty() {
            return incomplete("ruff_settings_execution_incomplete", None);
        }
        let parsed = match parse_ruff_settings(&observed.stdout) {
            Ok(parsed) => parsed,
            Err(reason) => return incomplete(reason, None),
        };
        if hash_tool(&request.tool) != Some(request.expected_tool_sha256) {
            return incomplete("tool_identity_changed", None);
        }
        if let Some(binding) = &request.project_config {
            if let Err(reason) = verify_project_config(&request.source, binding) {
                return incomplete(reason, None);
            }
        }
        Some(parsed)
    } else {
        None
    };

    let mut args = vec!["check".into()];
    if request.project_config.is_none() {
        args.push("--isolated".into());
    } else {
        args.push("--no-cache".into());
    }
    args.extend([
        "--output-format".into(),
        "json".into(),
        request.source.as_os_str().to_os_string(),
    ]);
    let scan = ProcessSpec {
        executable: request.tool.clone(),
        args,
        cwd: cwd.to_path_buf(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 16 * 1024 * 1024,
    };
    let scan_log = format!("{}-scan.log", request.run_id);
    let scan_outcome =
        match run_process_recorded(&scan, cancelled, &request.evidence_dir, &scan_log) {
            Ok(outcome) => (outcome, None),
            Err(failure) => (failure.outcome, Some(failure.kind.reason())),
        };
    let native_exit = match scan_outcome.0.termination {
        Termination::Exited(code) => code,
        _ => -1,
    };
    let mut parsed = parse_ruff_json(native_exit, &scan_outcome.0.stdout);
    if let Some(reason) = scan_outcome.1 {
        return incomplete(reason, Some(parsed));
    }
    if hash_tool(&request.tool) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_changed", Some(parsed));
    }
    if let Some(binding) = &request.project_config {
        if let Err(reason) = verify_project_config(&request.source, binding) {
            return incomplete(reason, Some(parsed));
        }
    }
    let Ok(source_after) = read_bounded_regular_file(&request.source, 16 * 1024 * 1024) else {
        return incomplete("source_changed", Some(parsed));
    };
    if <[u8; 32]>::from(Sha256::digest(&source_after)) != source_digest {
        return incomplete("source_changed", Some(parsed));
    }
    let Ok(expected_source) = request.source.canonicalize() else {
        return incomplete("source_changed", Some(parsed));
    };
    let original_count = parsed.diagnostics.len();
    parsed.diagnostics.retain(|diagnostic| {
        let reported = Path::new(&diagnostic.filename);
        let reported = if reported.is_absolute() {
            reported.to_path_buf()
        } else {
            cwd.join(reported)
        };
        reported
            .canonicalize()
            .is_ok_and(|path| path == expected_source)
    });
    if parsed.diagnostics.len() != original_count {
        return incomplete("report_source_mismatch", Some(parsed));
    }
    if native_exit == -1 {
        return incomplete("native_execution_incomplete", Some(parsed));
    }
    if parsed.state == RuffParseState::Incomplete {
        return incomplete(
            parsed.reason.unwrap_or("invalid_native_report"),
            Some(parsed),
        );
    }
    if settings.as_ref().is_some_and(|settings| {
        parsed.diagnostics.iter().any(|diagnostic| {
            (matches!(diagnostic.code.as_str(), "F401" | "E501")
                || codeguard_adapters::is_ruff_pydocstyle_rule(&diagnostic.code))
                && !settings.native_rule_enabled(&diagnostic.code)
        })
    }) {
        return incomplete("rule_settings_report_mismatch", Some(parsed));
    }
    let suppression_audit = if request.project_config.is_some() {
        let audit_spec = ProcessSpec {
            executable: request.tool.clone(),
            args: vec![
                "check".into(),
                "--no-cache".into(),
                "--ignore-noqa".into(),
                "--output-format".into(),
                "json".into(),
                request.source.as_os_str().to_os_string(),
            ],
            cwd: cwd.to_path_buf(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: request.deadline,
            output_limit_bytes: 16 * 1024 * 1024,
        };
        let log = format!("{}-ignore-noqa.log", request.run_id);
        let observed =
            match run_process_recorded(&audit_spec, cancelled, &request.evidence_dir, &log) {
                Ok(observed) => observed,
                Err(failure) => return incomplete(failure.kind.reason(), Some(parsed)),
            };
        let native_exit = match observed.termination {
            Termination::Exited(code) => code,
            _ => return incomplete("suppression_audit_execution_incomplete", Some(parsed)),
        };
        let audit = parse_ruff_json(native_exit, &observed.stdout);
        if audit.state != RuffParseState::Valid || !observed.stderr.is_empty() {
            return incomplete("suppression_audit_report_invalid", Some(parsed));
        }
        if audit.diagnostics.iter().any(|diagnostic| {
            let path = Path::new(&diagnostic.filename);
            let path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            };
            !path
                .canonicalize()
                .is_ok_and(|path| path == expected_source)
        }) {
            return incomplete("suppression_audit_source_mismatch", Some(parsed));
        }
        let (suppressed_diagnostic_count, suppressed_rule_ids) =
            match suppression_difference(&parsed.diagnostics, &audit.diagnostics) {
                Ok(value) => value,
                Err(reason) => return incomplete(reason, Some(parsed)),
            };
        Some(RuffSuppressionAudit {
            audit_sha256: format!("{:x}", Sha256::digest(&observed.stdout)),
            suppressed_diagnostic_count,
            suppressed_rule_ids,
            scope: "source_comments_only",
        })
    } else {
        None
    };
    if hash_tool(&request.tool) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_changed", Some(parsed));
    }
    if let Some(binding) = &request.project_config {
        if let Err(reason) = verify_project_config(&request.source, binding) {
            return incomplete(reason, Some(parsed));
        }
    }
    let Ok(source_after_audit) = read_bounded_regular_file(&request.source, 16 * 1024 * 1024)
    else {
        return incomplete("source_changed", Some(parsed));
    };
    if <[u8; 32]>::from(Sha256::digest(&source_after_audit)) != source_digest {
        return incomplete("source_changed", Some(parsed));
    }
    if Instant::now() >= request.deadline {
        return incomplete("request_deadline_exceeded", Some(parsed));
    }
    RuffProbeResult {
        state: RuffProbeState::EvidenceComplete,
        reason: None,
        parsed: Some(parsed),
        settings,
        suppression_audit,
    }
}

fn suppression_difference(
    normal: &[codeguard_adapters::RuffDiagnostic],
    unsuppressed: &[codeguard_adapters::RuffDiagnostic],
) -> Result<(usize, Vec<String>), &'static str> {
    type Key = (String, String, u32, u32, String, String);
    let key = |diagnostic: &codeguard_adapters::RuffDiagnostic| -> Key {
        (
            diagnostic.code.clone(),
            diagnostic.filename.clone(),
            diagnostic.location.row,
            diagnostic.location.column,
            diagnostic.message.clone(),
            diagnostic.severity.clone(),
        )
    };
    let mut remaining = BTreeMap::<Key, usize>::new();
    for diagnostic in normal {
        *remaining.entry(key(diagnostic)).or_default() += 1;
    }
    let mut suppressed_count = 0_usize;
    let mut suppressed_rules = BTreeSet::new();
    for diagnostic in unsuppressed {
        if diagnostic.code.is_empty()
            || diagnostic.code.len() > 12
            || !diagnostic
                .code
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            return Err("suppression_audit_rule_invalid");
        }
        let entry = remaining.entry(key(diagnostic)).or_default();
        if *entry > 0 {
            *entry -= 1;
        } else {
            suppressed_count += 1;
            suppressed_rules.insert(diagnostic.code.clone());
        }
    }
    if remaining.values().any(|count| *count > 0) {
        return Err("suppression_audit_inconsistent");
    }
    Ok((suppressed_count, suppressed_rules.into_iter().collect()))
}

fn verify_project_config(source: &Path, binding: &RuffConfigBinding) -> Result<(), &'static str> {
    if !binding.path.is_absolute() {
        return Err("invalid_config_binding");
    }
    let config_bytes = read_bounded_regular_file(&binding.path, 256 * 1024)
        .map_err(|_| "ruff_config_unavailable")?;
    if <[u8; 32]>::from(Sha256::digest(&config_bytes)) != binding.expected_sha256 {
        return Err("ruff_config_changed");
    }
    let selected = inspect_ruff_config(
        &config_bytes,
        ".",
        binding.path.to_str().unwrap_or("<non-utf8>"),
        binding
            .path
            .file_name()
            .is_some_and(|name| name == "pyproject.toml"),
    );
    if selected.configuration != "configured" {
        return Err("ruff_config_not_validated");
    }
    let config_dir = binding.path.parent().ok_or("invalid_config_binding")?;
    let mut directory = source.parent().ok_or("invalid_config_binding")?;
    if !directory.starts_with(config_dir) {
        return Err("config_outside_source_hierarchy");
    }
    loop {
        for name in [".ruff.toml", "ruff.toml", "pyproject.toml"] {
            let candidate = directory.join(name);
            let metadata = match std::fs::symlink_metadata(&candidate) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == ErrorKind::NotFound => continue,
                Err(_) => return Err("ruff_config_selection_unknown"),
            };
            if !metadata.file_type().is_file() {
                return Err("ruff_config_selection_unknown");
            }
            if candidate == binding.path {
                return Ok(());
            }
            if name == "pyproject.toml" {
                let bytes = read_bounded_regular_file(&candidate, 256 * 1024)
                    .map_err(|_| "ruff_config_selection_unknown")?;
                let observed = inspect_ruff_config(&bytes, ".", "pyproject.toml", true);
                if observed.configuration == "missing" {
                    continue;
                }
            }
            return Err("ruff_config_selection_changed");
        }
        if directory == config_dir {
            return Err("ruff_config_selection_unknown");
        }
        directory = directory.parent().ok_or("ruff_config_selection_unknown")?;
    }
}

fn hash_tool(path: &Path) -> Option<[u8; 32]> {
    let bytes = read_bounded_regular_file(path, 128 * 1024 * 1024).ok()?;
    Some(Sha256::digest(bytes).into())
}

fn incomplete(reason: &'static str, parsed: Option<RuffParsed>) -> RuffProbeResult {
    RuffProbeResult {
        state: RuffProbeState::Incomplete,
        reason: Some(reason),
        parsed,
        settings: None,
        suppression_audit: None,
    }
}
