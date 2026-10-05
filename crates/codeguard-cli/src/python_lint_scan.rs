//! Python 项目内已配置 Ruff 的逐文件原生扫描；只提供局部检查事实。

use crate::discovery::DiscoveryReport;
use crate::ruff_probe::{
    RuffConfigBinding, RuffProbeRequest, RuffProbeState, RuffSuppressionAudit, run_ruff_probe,
};
use codeguard_adapters::{CheckerConfiguration, RuffDiagnostic, RuffSettingsObservation};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 调用方锁定的工具和私有证据位置；工具批准来源仍由上层验证。
pub struct PythonLintScanRequest<'a> {
    pub root: &'a Path,
    pub discovery: &'a DiscoveryReport,
    /// 编辑快反馈的精确工作区相对路径；None 才表示本轮发现到的全部 Python 源码。
    pub selected_paths: Option<&'a [String]>,
    pub tool: Option<PathBuf>,
    pub tool_unavailable_reason: &'static str,
    pub expected_tool_sha256: [u8; 32],
    pub expected_version: String,
    pub evidence_dir: PathBuf,
    pub run_id: String,
    pub deadline: Instant,
}

/// 一个 Python 文件的配置、原生完成状态和有效诊断。
#[derive(Debug)]
pub struct PythonLintFileResult {
    pub path: String,
    pub config_ref: Option<String>,
    /// 本轮原生检查前后均一致的配置字节摘要；不代表配置已获批准。
    pub config_sha256: Option<String>,
    /// 本轮原生检查前后均一致的工具制品摘要；不代表工具已获批准。
    pub tool_sha256: Option<String>,
    /// 扫描前读取的源码摘要；仅在本轮输入未变化时可作为修复定位证据。
    pub source_sha256: Option<String>,
    /// 同轮原生逐文件设置观察；只反映全局规则选择，不能证明未被 suppress。
    pub rule_settings: Option<RuffSettingsObservation>,
    /// 同一原生工具忽略源码注释后的诊断差额；不改变正常 finding 集合。
    pub suppression_audit: Option<RuffSuppressionAudit>,
    pub completion: bool,
    pub reason: Option<String>,
    /// 原始消息仍属私有数据，不直接渲染到智能体对话。
    pub diagnostics: Vec<RuffDiagnostic>,
    /// 与 diagnostics 同序的局部稳定身份；输入变化或原生未完成时不签发。
    pub finding_keys: Vec<Option<LocalFindingKey>>,
}

/// 供未来 work sync 使用的局部身份；目前尚无持久任务或重命名协调。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalFindingKey {
    pub id: String,
    pub fingerprint: String,
}

/// 本轮可见 Python 文件的汇总；完整只指局部原生证据，不是项目门禁。
#[derive(Debug)]
pub struct PythonLintScanResult {
    pub files: Vec<PythonLintFileResult>,
    pub local_evidence_complete: bool,
    pub incomplete_reasons: Vec<String>,
}

/// 将本次配置状态和原生诊断整理为智能体可展示的数据，不输出原生消息或交付许可。
#[must_use]
pub fn python_lint_feedback(discovery: &DiscoveryReport, result: &PythonLintScanResult) -> Value {
    let configured: Vec<Value> = discovery
        .checker_configurations
        .iter()
        .filter(|checker| checker.checker_id == "python.ruff")
        .map(|checker| {
            json!({
                "checker_id": checker.checker_id,
                "build_root": checker.build_root,
                "configuration": checker.configuration,
                "configuration_ref": checker.configuration_ref,
                "next_action": checker.next_action,
            })
        })
        .collect();
    let files: Vec<Value> = result
        .files
        .iter()
        .map(|file| {
            let findings: Vec<Value> = file
                .diagnostics
                .iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    let identity = file.finding_keys.get(index).and_then(Option::as_ref);
                    json!({
                        "finding_id": identity.map(|key| &key.id),
                        "finding_fingerprint": identity.map(|key| &key.fingerprint),
                        "rule_id": diagnostic.code,
                        "path": file.path,
                        "line": diagnostic.location.row,
                        "column": diagnostic.location.column,
                        "rule_summary": rule_summary(&diagnostic.code),
                        "next_action": if file.completion { "核对原生规则与源码，修复后执行复检命令" } else { "先恢复检查完整性并重新运行原生工具" },
                        "repair_hint": repair_hint(file, &diagnostic.code),
                    })
                })
                .collect();
            json!({
                "path": file.path,
                "configuration_ref": file.config_ref,
                "config_sha256": file.config_sha256,
                "tool_sha256": file.tool_sha256,
                "source_sha256": file.source_sha256,
                "rule_settings": file.rule_settings,
                "suppression_audit": file.suppression_audit,
                "run_status": if file.completion {
                    if !findings.is_empty() { "findings" }
                    else if file.suppression_audit.as_ref().is_some_and(|audit| audit.suppressed_diagnostic_count > 0) { "suppressed" }
                    else { "passed" }
                } else { "incomplete" },
                "reason": file.reason,
                "findings": findings,
                "recheck_argv": ["ruff", "check", file.path],
                "recheck_cwd": discovery.root,
            })
        })
        .collect();
    json!({
        "schema_version": "0.3.0",
        "report_type": "python_lint_feedback",
        "scope": "local_native_scan_only",
        "checker_configurations": configured,
        "files": files,
        "incomplete_reasons": result.incomplete_reasons,
        "local_scan_complete": result.local_evidence_complete,
        "delivery_decision": "not_evaluated",
    })
}

fn repair_hint(file: &PythonLintFileResult, rule_id: &str) -> Value {
    if !file.completion || file.source_sha256.is_none() {
        return json!({"status":"verification_required","step":"先恢复原生检查完整性并重新扫描该源码"});
    }
    let (step, status) = match rule_id {
        "F401" => (
            "核对该导入是否确实未被使用；仅在确认后移除该导入",
            "bounded_repair_candidate",
        ),
        "E501" => (
            "核对已配置的行长限制；在保持语义的前提下重排行内容",
            "bounded_repair_candidate",
        ),
        "D100" => (
            "确认该公共模块的用途，并为模块补充准确的顶层 docstring",
            "bounded_repair_candidate",
        ),
        "D101" => (
            "确认该公共类的职责，并为类补充准确的 docstring",
            "bounded_repair_candidate",
        ),
        _ => (
            "查阅该原生规则及私有诊断，确认成因后制定修复",
            "investigation_required",
        ),
    };
    json!({
        "status": status,
        "allowed_path": file.path,
        "source_sha256": file.source_sha256,
        "step": step,
        "closure_condition": "对相同源码范围重新执行原生检查，并确认该规则发现消失",
    })
}

fn rule_summary(code: &str) -> &'static str {
    match code {
        "F401" => "导入未使用",
        "E501" => "行长度超出已配置限制",
        "invalid-syntax" => "Python原生语法错误",
        "D100" => "公共模块缺少文档字符串",
        "D101" => "公共类缺少文档字符串",
        _ => "查看原生规则说明及私有诊断详情",
    }
}

/// 复用同一总截止时间运行每个已配置文件，保留部分诊断和未完成原因。
#[must_use]
pub fn scan_python_lint(
    request: &PythonLintScanRequest<'_>,
    cancelled: &AtomicBool,
) -> PythonLintScanResult {
    let mut output = PythonLintScanResult {
        files: Vec::new(),
        local_evidence_complete: false,
        incomplete_reasons: Vec::new(),
    };
    if !request.root.is_absolute()
        || request.discovery.root != request.root.to_string_lossy()
        || request.run_id.is_empty()
        || !request
            .run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        output
            .incomplete_reasons
            .push("invalid_scan_request".into());
        return output;
    }
    if !request.discovery.observation_complete {
        output
            .incomplete_reasons
            .push("discovery_incomplete".into());
    }
    if request
        .discovery
        .unknown_conditions
        .iter()
        .any(|reason| reason == "selected_discovery_deadline_exceeded")
    {
        output
            .incomplete_reasons
            .push("request_deadline_exceeded".into());
    }
    if let Some(selected) = request.selected_paths {
        if selected.is_empty() {
            output
                .incomplete_reasons
                .push("selected_scope_empty".into());
            return output;
        }
        for target in selected {
            if !request
                .discovery
                .languages
                .get("python")
                .is_some_and(|python| python.source_files.contains(target))
            {
                output
                    .incomplete_reasons
                    .push(format!("target_not_discovered:{target}"));
            }
        }
    }
    let Some(python) = request.discovery.languages.get("python") else {
        output.incomplete_reasons.push("no_python_sources".into());
        return output;
    };
    if python.source_files.is_empty() {
        output.incomplete_reasons.push("no_python_sources".into());
        return output;
    }
    let sources: Vec<&String> = if let Some(selected) = request.selected_paths {
        python
            .source_files
            .iter()
            .filter(|source| selected.contains(source))
            .collect()
    } else {
        python.source_files.iter().collect()
    };
    let mut input_digests = BTreeMap::new();
    for source in &sources {
        let path = request.root.join(source);
        match digest_file(&path, 16 * 1024 * 1024) {
            Some(digest) => {
                input_digests.insert((*source).clone(), digest);
            }
            None => {
                output
                    .incomplete_reasons
                    .push(format!("source_unavailable:{source}"));
            }
        }
    }
    let mut config_digests = BTreeMap::new();
    for checker in request
        .discovery
        .checker_configurations
        .iter()
        .filter(|checker| {
            checker.checker_id == "python.ruff"
                && checker.configuration == "configured"
                && sources.iter().any(|source| {
                    select_checker(request.discovery, source).is_some_and(|selected| {
                        selected.configuration_ref == checker.configuration_ref
                    })
                })
        })
    {
        if let Some(digest) =
            digest_file(&request.root.join(&checker.configuration_ref), 256 * 1024)
        {
            config_digests.insert(checker.configuration_ref.clone(), digest);
        } else {
            output
                .incomplete_reasons
                .push(format!("config_unavailable:{}", checker.configuration_ref));
        }
    }
    for (index, source) in sources.iter().enumerate() {
        let source = *source;
        let Some(checker) = select_checker(request.discovery, source) else {
            output
                .files
                .push(not_run(source, None, "ruff_config_not_resolved"));
            continue;
        };
        if checker.configuration != "configured" {
            output.files.push(not_run(
                source,
                Some(&checker.configuration_ref),
                &checker.reason,
            ));
            continue;
        }
        if !input_digests.contains_key(source) {
            output.files.push(not_run(
                source,
                Some(&checker.configuration_ref),
                "source_unavailable",
            ));
            continue;
        }
        let Some(config_digest) = config_digests.get(&checker.configuration_ref).copied() else {
            output.files.push(not_run(
                source,
                Some(&checker.configuration_ref),
                "ruff_config_unavailable",
            ));
            continue;
        };
        let Some(tool) = &request.tool else {
            output.files.push(not_run(
                source,
                Some(&checker.configuration_ref),
                request.tool_unavailable_reason,
            ));
            continue;
        };
        let probe = RuffProbeRequest {
            source: request.root.join(source),
            tool: tool.clone(),
            expected_tool_sha256: request.expected_tool_sha256,
            expected_version: request.expected_version.clone(),
            evidence_dir: request.evidence_dir.clone(),
            run_id: format!("{}-{}", request.run_id, index),
            deadline: request.deadline,
            project_config: Some(RuffConfigBinding {
                path: request.root.join(&checker.configuration_ref),
                expected_sha256: config_digest,
            }),
        };
        let result = run_ruff_probe(&probe, cancelled);
        let mut completion = result.state == RuffProbeState::EvidenceComplete;
        let mut reason = result.reason.map(str::to_owned);
        let settings = if completion { result.settings } else { None };
        let suppression_audit = if completion {
            result.suppression_audit
        } else {
            None
        };
        let diagnostics = result
            .parsed
            .map_or_else(Vec::new, |parsed| parsed.diagnostics);
        let finding_keys = if completion {
            input_digests.get(source).and_then(|expected| {
                read_bounded_regular_file(&request.root.join(source), 16 * 1024 * 1024)
                    .ok()
                    .filter(|bytes| <[u8; 32]>::from(Sha256::digest(bytes)) == *expected)
                    .map(|bytes| derive_local_finding_keys(source, &bytes, &diagnostics))
            })
        } else {
            None
        };
        if completion
            && !diagnostics.is_empty()
            && finding_keys
                .as_ref()
                .is_none_or(|keys| keys.iter().any(Option::is_none))
        {
            completion = false;
            reason = Some("finding_identity_unavailable".into());
        }
        output.files.push(PythonLintFileResult {
            path: (*source).clone(),
            config_ref: Some(checker.configuration_ref.clone()),
            config_sha256: completion.then(|| hex_digest(&config_digest)),
            tool_sha256: completion.then(|| hex_digest(&request.expected_tool_sha256)),
            source_sha256: input_digests.get(source).map(hex_digest),
            rule_settings: if completion { settings } else { None },
            suppression_audit: if completion { suppression_audit } else { None },
            completion,
            reason,
            diagnostics,
            finding_keys: finding_keys.unwrap_or_default(),
        });
    }
    for (source, digest) in input_digests {
        if digest_file(&request.root.join(&source), 16 * 1024 * 1024) != Some(digest) {
            output
                .incomplete_reasons
                .push(format!("source_changed:{source}"));
            if let Some(file) = output.files.iter_mut().find(|file| file.path == source) {
                file.completion = false;
                file.reason = Some("source_changed".into());
                file.source_sha256 = None;
                file.config_sha256 = None;
                file.tool_sha256 = None;
                file.rule_settings = None;
                file.suppression_audit = None;
                file.finding_keys.clear();
            }
        }
    }
    for (config, digest) in config_digests {
        if digest_file(&request.root.join(&config), 256 * 1024) != Some(digest) {
            output
                .incomplete_reasons
                .push(format!("config_changed:{config}"));
            for file in output
                .files
                .iter_mut()
                .filter(|file| file.config_ref.as_deref() == Some(&config))
            {
                file.completion = false;
                file.reason = Some("config_changed".into());
                file.config_sha256 = None;
                file.tool_sha256 = None;
                file.rule_settings = None;
                file.suppression_audit = None;
                file.finding_keys.clear();
            }
        }
    }
    if request.tool.as_ref().is_some_and(|tool| {
        digest_file(tool, 128 * 1024 * 1024) != Some(request.expected_tool_sha256)
    }) {
        for file in output.files.iter_mut().filter(|file| file.completion) {
            file.completion = false;
            file.reason = Some("tool_identity_changed".into());
            file.config_sha256 = None;
            file.tool_sha256 = None;
            file.rule_settings = None;
            file.suppression_audit = None;
            file.finding_keys.clear();
        }
    }
    if Instant::now() >= request.deadline {
        output
            .incomplete_reasons
            .push("request_deadline_exceeded".into());
        for file in &mut output.files {
            file.completion = false;
            file.reason = Some("request_deadline_exceeded".into());
            file.config_sha256 = None;
            file.tool_sha256 = None;
            file.rule_settings = None;
            file.suppression_audit = None;
            file.finding_keys.clear();
        }
    }
    for file in &output.files {
        if let Some(reason) = &file.reason {
            output
                .incomplete_reasons
                .push(format!("{}:{reason}", file.path));
        }
    }
    output.incomplete_reasons.sort();
    output.incomplete_reasons.dedup();
    output.local_evidence_complete =
        output.incomplete_reasons.is_empty() && output.files.iter().all(|file| file.completion);
    output
}

pub(crate) fn select_checker<'a>(
    report: &'a DiscoveryReport,
    source: &str,
) -> Option<&'a CheckerConfiguration> {
    report
        .checker_configurations
        .iter()
        .filter(|checker| {
            checker.checker_id == "python.ruff"
                && (checker.build_root == "."
                    || source.starts_with(&format!("{}/", checker.build_root)))
        })
        .max_by_key(|checker| checker.build_root.len())
}

fn digest_file(path: &Path, limit: u64) -> Option<[u8; 32]> {
    let bytes = read_bounded_regular_file(path, limit).ok()?;
    Some(Sha256::digest(bytes).into())
}

fn hex_digest(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn derive_local_finding_keys(
    source: &str,
    bytes: &[u8],
    diagnostics: &[RuffDiagnostic],
) -> Vec<Option<LocalFindingKey>> {
    let lines: Vec<&[u8]> = bytes.split(|byte| *byte == b'\n').collect();
    let mut occurrences = BTreeMap::<[u8; 32], u64>::new();
    diagnostics
        .iter()
        .map(|diagnostic| {
            let row = diagnostic.location.row.checked_sub(1)? as usize;
            let line = lines.get(row)?;
            let mut anchor = line.trim_ascii();
            if anchor.is_empty()
                && diagnostic.code == "invalid-syntax"
                && diagnostic.severity == "error"
            {
                // 原生EOF位置保持不变；身份引用此前实际源码，不凭空给空文件签发发现。
                anchor = lines
                    .get(..row)?
                    .iter()
                    .rev()
                    .map(|line| line.trim_ascii())
                    .find(|line| !line.is_empty())?;
            }
            if anchor.is_empty() {
                return None;
            }
            let mut hash = Sha256::new();
            for part in [
                b"codeguard-ruff-finding-v1".as_slice(),
                source.as_bytes(),
                diagnostic.code.as_bytes(),
                anchor,
                diagnostic.message.as_bytes(),
            ] {
                hash.update((part.len() as u64).to_be_bytes());
                hash.update(part);
            }
            let base: [u8; 32] = hash.finalize().into();
            let ordinal = occurrences.entry(base).or_default();
            let mut final_hash = Sha256::new();
            final_hash.update(base);
            final_hash.update(ordinal.to_be_bytes());
            *ordinal += 1;
            let fingerprint = hex_digest(&final_hash.finalize().into());
            Some(LocalFindingKey {
                id: format!("CG-{}", &fingerprint[..32]),
                fingerprint,
            })
        })
        .collect()
}

fn not_run(source: &str, config_ref: Option<&str>, reason: &str) -> PythonLintFileResult {
    PythonLintFileResult {
        path: source.into(),
        config_ref: config_ref.map(str::to_owned),
        config_sha256: None,
        tool_sha256: None,
        source_sha256: None,
        rule_settings: None,
        suppression_audit: None,
        completion: false,
        reason: Some(reason.into()),
        diagnostics: Vec::new(),
        finding_keys: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::derive_local_finding_keys;
    use codeguard_adapters::{RuffDiagnostic, RuffLocation};

    fn diagnostic(code: &str, message: &str, row: u32) -> RuffDiagnostic {
        RuffDiagnostic {
            code: code.into(),
            message: message.into(),
            filename: "app.py".into(),
            location: RuffLocation { row, column: 1 },
            severity: "error".into(),
        }
    }

    #[test]
    fn native_syntax_at_empty_eof_uses_preceding_source_anchor() {
        let source = b"def run():\n";
        let keys = derive_local_finding_keys(
            "app.py",
            source,
            &[diagnostic(
                "invalid-syntax",
                "Expected an indented block",
                2,
            )],
        );
        assert!(
            keys[0].is_some(),
            "native EOF diagnostics need a stable repair identity"
        );
        let shifted = derive_local_finding_keys(
            "app.py",
            b"# heading\ndef run():\n",
            &[diagnostic(
                "invalid-syntax",
                "Expected an indented block",
                3,
            )],
        );
        assert_eq!(keys, shifted);
        let unrelated =
            derive_local_finding_keys("app.py", source, &[diagnostic("F401", "unused", 2)]);
        assert!(unrelated[0].is_none());
    }

    #[test]
    fn line_shift_keeps_local_identity_but_distinct_issues_do_not_merge() {
        let original = derive_local_finding_keys(
            "app.py",
            b"import os\nimport sys\n",
            &[
                diagnostic("F401", "os unused", 1),
                diagnostic("F401", "sys unused", 2),
            ],
        );
        let shifted = derive_local_finding_keys(
            "app.py",
            b"# inserted\nimport os\nimport sys\n",
            &[
                diagnostic("F401", "os unused", 2),
                diagnostic("F401", "sys unused", 3),
            ],
        );
        assert_eq!(original, shifted);
        assert_ne!(original[0], original[1]);
        assert_ne!(
            original[0],
            derive_local_finding_keys(
                "renamed.py",
                b"import os\n",
                &[diagnostic("F401", "os unused", 1)]
            )[0]
        );
        assert_ne!(
            original[0],
            derive_local_finding_keys(
                "app.py",
                b"import os\n",
                &[diagnostic("F402", "os unused", 1)]
            )[0]
        );
    }

    #[test]
    fn duplicate_native_diagnostics_get_distinct_keys_and_bad_location_gets_none() {
        let repeated = diagnostic("F401", "unused", 1);
        let keys = derive_local_finding_keys(
            "app.py",
            b"import os\n",
            &[repeated.clone(), repeated, diagnostic("F401", "unused", 3)],
        );
        assert_ne!(keys[0], keys[1]);
        assert!(keys[2].is_none());
    }
}
