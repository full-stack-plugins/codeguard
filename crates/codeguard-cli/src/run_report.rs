//! 公共运行报告的严格消费边界；解析通过不等于来源可信或交付认证。

use crate::check_request::parse_check_request;
use codeguard_core::{
    AllowlistTarget, CHECK_CATEGORIES, FalsePositiveIdentity, match_false_positive_identity,
    valid_false_positive_identity,
};
use codeguard_runtime::SourceSnapshot;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// 结构验证后的报告视图；最终 allow 仍须可信来源与绑定复核。
#[derive(Clone, Debug)]
pub struct RunReport {
    /// 请求身份。
    pub request_id: String,
    /// 执行身份。
    pub run_id: String,
    /// Codeguard 新协议退出码。
    pub exit_code: u64,
    /// 已确认的发现 ID，含未完成义务中的有效发现。
    pub finding_ids: Vec<String>,
    /// 新协议中被声明为精确误报处置的原始发现 ID；来源仍待独立核验。
    pub whitelisted_finding_ids: Vec<String>,
    /// 新协议中仍会阻断交付的发现 ID。
    pub active_blocking_finding_ids: Vec<String>,
    /// 报告声明的交付判定，尚非可信认证。
    pub delivery_decision: String,
    document: Value,
}

impl RunReport {
    /// 借用已验证结构的报告；调用方仍需独立核验可信来源及内容身份。
    #[must_use]
    pub fn document(&self) -> &Value {
        &self.document
    }
}

/// 用宿主独立指定的工作区复核报告中全部源码目标的当前字节摘要。
///
/// 仅证明调用瞬间的文件内容与报告声明一致；不验证原生工具输出、批准来源、
/// 依赖图或最终门禁时间。宿主在终局判定前仍须再次复核输入和审批。
pub fn compare_claimed_source_hashes(
    report: &RunReport,
    trusted_root: &Path,
) -> Result<BTreeMap<String, String>, String> {
    if report.document["schema_version"] != "1.4" {
        return Err("旧报告没有完整源码身份".into());
    }
    let mut claimed = BTreeMap::new();
    for result in report.document["results"]
        .as_array()
        .ok_or("运行报告缺义务结果")?
    {
        for finding in result["findings"].as_array().ok_or("义务结果缺发现")? {
            let identity: FalsePositiveIdentity =
                serde_json::from_value(finding["native_identity"].clone())
                    .map_err(|_| "原生源码身份不可解析")?;
            if let AllowlistTarget::Source { path, file_sha256 } = identity.target {
                if claimed
                    .insert(path.clone(), file_sha256.clone())
                    .is_some_and(|old| old != file_sha256)
                {
                    return Err("同一源码目标声明了冲突的字节摘要".into());
                }
            }
        }
    }
    if claimed.is_empty() {
        return Err("报告没有可复核的源码目标".into());
    }
    let snapshot = SourceSnapshot::capture(
        trusted_root,
        claimed.keys().map(PathBuf::from),
        4096,
        16 * 1024 * 1024,
        256 * 1024 * 1024,
    )
    .map_err(|_| "源码目标不可安全读取或超出复核预算")?;
    for (path, bytes) in snapshot.files() {
        let normalized = path.to_str().ok_or("源码目标路径编码不可用")?;
        let actual = format!("{:x}", Sha256::digest(bytes));
        if claimed.get(normalized) != Some(&actual) {
            return Err("当前源码字节与报告声明不一致".into());
        }
    }
    Ok(claimed)
}

/// 拒绝未知版本/枚举、假完整、缺 gate impact、覆盖矛盾和局部 allow。
pub fn parse_run_report(bytes: &[u8]) -> Result<RunReport, String> {
    let document = codeguard_adapters::parse_unique_json(bytes).map_err(str::to_owned)?;
    let root = object(&document, "运行报告")?;
    keys(
        root,
        &[
            "schema_version",
            "report_type",
            "operation",
            "request_id",
            "run_id",
            "command_status",
            "exit_code",
            "request",
            "plan_id",
            "identities",
            "results",
            "checker_statuses",
            "dispositions",
            "delivery_gate",
            "warnings",
            "next_actions",
            "extensions",
        ],
        "运行报告",
    )?;
    let protocol_minor = version(root)?;
    let require_locations = protocol_minor >= 2;
    let require_dispositions = protocol_minor >= 3;
    let require_native_identity = protocol_minor >= 4;
    if string(root, "report_type", "运行报告")? != "run_report" {
        return Err("运行报告 report_type 错误".into());
    }
    extensions(root.get("extensions"))?;
    let request_id = string(root, "request_id", "运行报告")?.to_owned();
    let run_id = string(root, "run_id", "运行报告")?.to_owned();
    string(root, "plan_id", "运行报告")?;
    let request_value = root.get("request").ok_or("运行报告缺 request")?;
    let request = parse_check_request(request_value.to_string().as_bytes())?;
    let operation = string(root, "operation", "运行报告")?;
    if operation != request.command || request_id != request.request_id {
        return Err("运行报告与请求身份不一致".into());
    }
    let status = string(root, "command_status", "运行报告")?;
    let exit_code = root
        .get("exit_code")
        .and_then(Value::as_u64)
        .ok_or("运行报告 exit_code 非法")?;
    if !matches!(
        (status, exit_code),
        ("complete", 0 | 1) | ("incomplete", 3) | ("internal_error", 4) | ("cancelled", 130)
    ) {
        return Err("运行状态与退出码矛盾".into());
    }
    let tool_ids = validate_identities(root)?;
    validate_checker_statuses(root, require_locations)?;
    let mut result_ids = BTreeSet::new();
    let mut findings = Vec::new();
    let mut native_identities = BTreeMap::new();
    let mut blocking = BTreeSet::new();
    let mut complete_blocking = BTreeSet::new();
    let mut incomplete = BTreeSet::new();
    let mut coverage_gap = BTreeSet::new();
    let mut all_not_applicable = true;
    for result in array(root, "results", "运行报告")? {
        let result = object(result, "义务结果")?;
        keys(
            result,
            &[
                "obligation_id",
                "completion",
                "reason",
                "findings",
                "coverage",
                "execution_refs",
            ],
            "义务结果",
        )?;
        let id = string(result, "obligation_id", "义务结果")?;
        if !result_ids.insert(id.to_owned()) {
            return Err(format!("重复义务结果：{id}"));
        }
        let completion = string(result, "completion", "义务结果")?;
        if !matches!(completion, "complete" | "incomplete" | "not_applicable") {
            return Err(format!("{id}: 非法完成状态"));
        }
        if completion != "not_applicable" {
            all_not_applicable = false;
        }
        if completion != "complete" {
            string(result, "reason", "义务结果")?;
        }
        let execution_refs = unique_strings(result, "execution_refs", "义务结果")?;
        if completion == "complete" && execution_refs.is_empty() {
            return Err(format!("{id}: 完整结果缺执行证据"));
        }
        if completion == "complete" && tool_ids.is_empty() {
            return Err(format!("{id}: 完整结果缺工具身份"));
        }
        let coverage = object(result.get("coverage").ok_or("缺 coverage")?, "coverage")?;
        keys(
            coverage,
            &["expected", "observed", "unresolved", "proof_kind"],
            "coverage",
        )?;
        let expected = unique_strings(coverage, "expected", "coverage")?;
        let observed = unique_strings(coverage, "observed", "coverage")?;
        let unresolved = unique_strings(coverage, "unresolved", "coverage")?;
        let proof_kind = string(coverage, "proof_kind", "coverage")?;
        if !matches!(
            proof_kind,
            "native_report" | "file_manifest" | "build_model" | "none"
        ) {
            return Err(format!("{id}: 非法覆盖证明类别"));
        }
        if completion == "complete"
            && (expected.is_empty()
                || expected.iter().collect::<BTreeSet<_>>() != observed.iter().collect()
                || !unresolved.is_empty()
                || proof_kind == "none")
        {
            coverage_gap.insert(id.to_owned());
        }
        if completion == "incomplete" {
            incomplete.insert(id.to_owned());
        }
        let items = array(result, "findings", "义务结果")?;
        if completion == "not_applicable" && (!items.is_empty() || !observed.is_empty()) {
            return Err(format!("{id}: 不适用义务含检查证据"));
        }
        for item in items {
            let item = object(item, "发现")?;
            keys(
                item,
                &[
                    "id",
                    "native_rule_id",
                    "tool_id",
                    "severity",
                    "gate_impact",
                    "message",
                    "obligation_id",
                    "evidence_refs",
                    "locations",
                    "native_identity",
                ],
                "发现",
            )?;
            let finding_id = string(item, "id", "发现")?;
            string(item, "native_rule_id", "发现")?;
            let tool_id = string(item, "tool_id", "发现")?;
            if !tool_ids.contains(tool_id) {
                return Err(format!("{finding_id}: 来源工具未声明"));
            }
            string(item, "severity", "发现")?;
            string(item, "message", "发现")?;
            if string(item, "obligation_id", "发现")? != id {
                return Err(format!("{finding_id}: 发现义务身份不一致"));
            }
            if unique_strings(item, "evidence_refs", "发现")?.is_empty() {
                return Err(format!("{finding_id}: 缺原生证据引用"));
            }
            validate_locations(item, finding_id, require_locations)?;
            if require_native_identity {
                let identity = validate_native_finding_identity(root, item, finding_id)?;
                if let AllowlistTarget::Source { path, .. } = &identity.target {
                    if !expected.contains(&path.as_str()) || !observed.contains(&path.as_str()) {
                        return Err(format!("{finding_id}: 原生源码目标不在本义务覆盖范围内"));
                    }
                }
                native_identities.insert(finding_id.to_owned(), identity);
            } else if item.contains_key("native_identity") {
                return Err(format!("{finding_id}: 旧报告不能声明原生精确身份"));
            }
            let impact = string(item, "gate_impact", "发现")?;
            match impact {
                "blocking" => {
                    blocking.insert(finding_id.to_owned());
                    if completion == "complete" {
                        complete_blocking.insert(finding_id.to_owned());
                    }
                }
                "non_blocking" => {}
                "undetermined" => {
                    incomplete.insert(id.to_owned());
                }
                _ => return Err(format!("{finding_id}: 非法 gate_impact")),
            }
            if findings.contains(&finding_id.to_owned()) {
                return Err(format!("重复发现 ID：{finding_id}"));
            }
            findings.push(finding_id.to_owned());
        }
    }
    unique_strings(root, "warnings", "运行报告")?;
    unique_strings(root, "next_actions", "运行报告")?;
    if result_ids.is_empty() && request.language != "all" {
        return Err("显式语言请求没有义务结果".into());
    }
    let whitelisted = validate_dispositions(
        root,
        &blocking,
        &complete_blocking,
        &native_identities,
        require_dispositions,
        require_native_identity,
    )?;
    let active_blocking: BTreeSet<_> = blocking.difference(&whitelisted).cloned().collect();
    let gate = object(
        root.get("delivery_gate").ok_or("缺 delivery_gate")?,
        "delivery_gate",
    )?;
    keys(
        gate,
        &[
            "decision",
            "eligible",
            "blocking_finding_ids",
            "whitelisted_false_positive_ids",
            "active_blocking_finding_ids",
            "incomplete_obligation_ids",
            "coverage_mismatch_ids",
            "invalid_ledger_ids",
        ],
        "delivery_gate",
    )?;
    let decision = string(gate, "decision", "delivery_gate")?;
    if !matches!(
        decision,
        "allow"
            | "allow_with_exceptions"
            | "deny"
            | "incomplete"
            | "not_applicable"
            | "not_evaluated"
    ) {
        return Err("非法交付判定".into());
    }
    if decision == "allow_with_exceptions" && !require_dispositions {
        return Err("旧协议不支持带例外交付判定".into());
    }
    let eligible = gate
        .get("eligible")
        .and_then(Value::as_bool)
        .ok_or("delivery_gate.eligible 非法")?;
    let declared_blocking = unique_strings(gate, "blocking_finding_ids", "delivery_gate")?;
    if declared_blocking.into_iter().collect::<BTreeSet<_>>()
        != blocking.iter().map(String::as_str).collect()
    {
        return Err("交付报告遗漏或伪造阻断发现".into());
    }
    if require_dispositions {
        let declared_whitelisted =
            unique_strings(gate, "whitelisted_false_positive_ids", "delivery_gate")?;
        let declared_active = unique_strings(gate, "active_blocking_finding_ids", "delivery_gate")?;
        if declared_whitelisted.into_iter().collect::<BTreeSet<_>>()
            != whitelisted.iter().map(String::as_str).collect()
            || declared_active.into_iter().collect::<BTreeSet<_>>()
                != active_blocking.iter().map(String::as_str).collect()
        {
            return Err("交付报告的误报处置与活跃阻断集合不一致".into());
        }
    } else if gate.contains_key("whitelisted_false_positive_ids")
        || gate.contains_key("active_blocking_finding_ids")
    {
        return Err("旧协议不接受白名单门禁字段".into());
    }
    let declared_incomplete = unique_strings(gate, "incomplete_obligation_ids", "delivery_gate")?;
    let declared_coverage = unique_strings(gate, "coverage_mismatch_ids", "delivery_gate")?;
    let invalid_ledger = unique_strings(gate, "invalid_ledger_ids", "delivery_gate")?;
    let declared_incomplete_set: BTreeSet<_> = declared_incomplete.into_iter().collect();
    let declared_coverage_set: BTreeSet<_> = declared_coverage.into_iter().collect();
    if !incomplete
        .iter()
        .all(|id| declared_incomplete_set.contains(id.as_str()))
        || !coverage_gap
            .iter()
            .all(|id| declared_coverage_set.contains(id.as_str()))
    {
        return Err("交付报告遗漏未完成或覆盖缺口".into());
    }
    let has_gap = !incomplete.is_empty()
        || !coverage_gap.is_empty()
        || !declared_incomplete_set.is_empty()
        || !declared_coverage_set.is_empty()
        || !invalid_ledger.is_empty();
    if has_gap && status == "complete" {
        return Err("未完成证据不能声明运行完成".into());
    }
    if !has_gap && status == "incomplete" {
        return Err("运行未完成缺具体义务或账本原因".into());
    }
    if !active_blocking.is_empty() && status == "complete" && exit_code != 1 {
        return Err("阻断发现与退出码矛盾".into());
    }
    if active_blocking.is_empty() && status == "complete" && exit_code == 1 {
        return Err("退出码 1 缺阻断发现".into());
    }
    match decision {
        "allow" => {
            if operation != "check"
                || request.language != "all"
                || !eligible
                || status != "complete"
                || exit_code != 0
                || has_gap
                || !blocking.is_empty()
                || !whitelisted.is_empty()
                || result_ids.is_empty()
                || all_not_applicable
            {
                return Err("交付 allow 与请求/证据不一致".into());
            }
        }
        "allow_with_exceptions" => {
            if operation != "check"
                || request.language != "all"
                || !eligible
                || status != "complete"
                || exit_code != 0
                || has_gap
                || whitelisted.is_empty()
                || !active_blocking.is_empty()
                || result_ids.is_empty()
            {
                return Err("带批准例外交付与请求/证据不一致".into());
            }
        }
        "deny" => {
            if operation != "check"
                || request.language != "all"
                || !eligible
                || status != "complete"
                || active_blocking.is_empty()
                || exit_code != 1
                || has_gap
            {
                return Err("交付 deny 与证据不一致".into());
            }
        }
        "incomplete" => {
            if eligible || !has_gap {
                return Err("交付 incomplete 缺证据缺口".into());
            }
        }
        "not_applicable" => {
            if eligible
                || operation != "check"
                || request.language != "all"
                || !all_not_applicable
                || has_gap
                || !blocking.is_empty()
                || status != "complete"
                || exit_code != 0
            {
                return Err("交付不适用与证据不一致".into());
            }
        }
        "not_evaluated" => {
            if eligible || (operation == "check" && request.language == "all") {
                return Err("局部请求不能取得交付资格".into());
            }
        }
        _ => unreachable!("已验证交付枚举"),
    }
    Ok(RunReport {
        request_id,
        run_id,
        exit_code,
        finding_ids: findings,
        whitelisted_finding_ids: whitelisted.into_iter().collect(),
        active_blocking_finding_ids: active_blocking.into_iter().collect(),
        delivery_decision: decision.to_owned(),
        document,
    })
}

fn validate_dispositions(
    root: &Map<String, Value>,
    raw_blocking: &BTreeSet<String>,
    complete_blocking: &BTreeSet<String>,
    native_identities: &BTreeMap<String, FalsePositiveIdentity>,
    required: bool,
    require_native_identity: bool,
) -> Result<BTreeSet<String>, String> {
    if !required {
        return if root.contains_key("dispositions") {
            Err("旧报告协议不接受白名单处置".into())
        } else {
            Ok(BTreeSet::new())
        };
    }
    let policy = object(&root["identities"]["policy"], "政策身份")?;
    let revision = string(policy, "revision", "政策身份")?;
    let policy_digest = string(policy, "digest", "政策身份")?;
    let mut seen = BTreeSet::new();
    for item in array(root, "dispositions", "运行报告")? {
        let disposition = object(item, "误报处置")?;
        keys(
            disposition,
            &[
                "finding_id",
                "kind",
                "decision_id",
                "approval_ref",
                "approved_policy_revision",
                "policy_digest",
                "match_basis_digest",
                "expires_at",
                "decision_identity",
            ],
            "误报处置",
        )?;
        let id = string(disposition, "finding_id", "误报处置")?;
        if !raw_blocking.contains(id)
            || !complete_blocking.contains(id)
            || !seen.insert(id.to_owned())
        {
            return Err(format!("{id}: 误报处置未绑定唯一且完整的原始阻断发现"));
        }
        if string(disposition, "kind", "误报处置")? != "whitelisted_false_positive" {
            return Err(format!("{id}: 非法误报处置类别"));
        }
        for key in ["decision_id", "approval_ref", "approved_policy_revision"] {
            if !safe_public_reference(string(disposition, key, "误报处置")?) {
                return Err(format!("{id}: 非法公开批准引用 {key}"));
            }
        }
        if string(disposition, "approved_policy_revision", "误报处置")? != revision {
            return Err(format!("{id}: 误报处置策略修订不一致"));
        }
        digest(disposition, "policy_digest", "误报处置")?;
        digest(disposition, "match_basis_digest", "误报处置")?;
        if string(disposition, "policy_digest", "误报处置")? != policy_digest {
            return Err(format!("{id}: 误报处置策略摘要不一致"));
        }
        if disposition
            .get("expires_at")
            .and_then(Value::as_u64)
            .filter(|expires| *expires > 0)
            .is_none()
        {
            return Err(format!("{id}: 误报处置缺有效期限"));
        }
        if require_native_identity {
            let decision: FalsePositiveIdentity = serde_json::from_value(
                disposition
                    .get("decision_identity")
                    .ok_or_else(|| format!("{id}: 缺精确批准身份"))?
                    .clone(),
            )
            .map_err(|_| format!("{id}: 精确批准身份无效"))?;
            let native = native_identities
                .get(id)
                .ok_or_else(|| format!("{id}: 原生发现身份缺失"))?;
            match_false_positive_identity(native, &decision)
                .map_err(|_| format!("{id}: 批准身份与原生发现不一致"))?;
        } else if disposition.contains_key("decision_identity") {
            return Err(format!("{id}: 旧报告不能声明精确批准身份"));
        }
    }
    Ok(seen)
}

fn validate_native_finding_identity(
    root: &Map<String, Value>,
    finding: &Map<String, Value>,
    finding_id: &str,
) -> Result<FalsePositiveIdentity, String> {
    let identity: FalsePositiveIdentity = serde_json::from_value(
        finding
            .get("native_identity")
            .ok_or_else(|| format!("{finding_id}: 缺本轮原生精确身份"))?
            .clone(),
    )
    .map_err(|_| format!("{finding_id}: 原生精确身份无效"))?;
    if !valid_false_positive_identity(&identity)
        || identity.finding_id != finding_id
        || finding["native_rule_id"].as_str() != Some(identity.native_rule_id.as_str())
    {
        return Err(format!("{finding_id}: 原生精确身份与发现不一致"));
    }
    let tool_id = string(finding, "tool_id", "发现")?;
    let tools = root["identities"]["tools"].as_array().ok_or("缺工具身份")?;
    if !tools.iter().any(|tool| {
        tool["id"].as_str() == Some(tool_id)
            && tool["binary_sha256"].as_str() == Some(identity.tool_sha256.as_str())
    }) {
        return Err(format!("{finding_id}: 原生工具摘要与报告身份不一致"));
    }
    let checker_statuses = root["checker_statuses"].as_array().ok_or("缺检查器状态")?;
    let matching_checker_count = checker_statuses
        .iter()
        .filter(|checker| {
            checker["checker_id"].as_str() == Some(identity.checker_id.as_str())
                && checker["category"].as_str() == Some(identity.category.as_str())
                && checker["configuration"] == "configured"
                && checker["run_status"] == "findings"
        })
        .count();
    if matching_checker_count != 1 {
        return Err(format!("{finding_id}: 原生检查器身份缺失或歧义"));
    }
    let primary = finding["locations"]
        .as_array()
        .and_then(|locations| locations.first())
        .ok_or_else(|| format!("{finding_id}: 缺原生主定位"))?;
    let primary_matches = match &identity.target {
        AllowlistTarget::Source { path, .. } => {
            matches!(primary["kind"].as_str(), Some("source" | "project"))
                && primary["path"].as_str() == Some(path.as_str())
        }
        AllowlistTarget::Dependency {
            component, version, ..
        } => {
            primary["kind"] == "dependency"
                && primary["component"].as_str() == Some(component.as_str())
                && primary["version"].as_str() == Some(version.as_str())
        }
    };
    if !primary_matches {
        return Err(format!("{finding_id}: 原生精确身份与主目标不一致"));
    }
    Ok(identity)
}

fn safe_public_reference(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/' | b'#' | b'@')
        })
}

fn validate_locations(
    finding: &Map<String, Value>,
    finding_id: &str,
    required: bool,
) -> Result<(), String> {
    let Some(locations) = finding.get("locations") else {
        return if required {
            Err(format!("{finding_id}: 新协议发现缺定位"))
        } else {
            Ok(())
        };
    };
    let locations = locations
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| format!("{finding_id}: 定位必须是非空数组"))?;
    for location in locations {
        let location = object(location, "发现定位")?;
        keys(
            location,
            &["kind", "path", "line", "column", "component", "version"],
            "发现定位",
        )?;
        let kind = string(location, "kind", "发现定位")?;
        if !matches!(kind, "source" | "dependency" | "project") {
            return Err(format!("{finding_id}: 未知定位类别"));
        }
        if matches!(kind, "source" | "project") {
            let path = string(location, "path", "发现定位")?;
            if !safe_relative_path(path) {
                return Err(format!("{finding_id}: 定位路径必须是项目内相对路径"));
            }
        }
        if kind == "dependency" && location.contains_key("path") {
            let path = string(location, "path", "发现定位")?;
            if !safe_relative_path(path) {
                return Err(format!("{finding_id}: 依赖清单路径必须是项目内相对路径"));
            }
        }
        if kind == "dependency" {
            string(location, "component", "发现定位")?;
        }
        for key in ["line", "column"] {
            if location.contains_key(key)
                && location[key]
                    .as_u64()
                    .filter(|n| (1..=u32::MAX as u64).contains(n))
                    .is_none()
            {
                return Err(format!("{finding_id}: 非法定位 {key}"));
            }
        }
        for key in ["path", "component", "version"] {
            if location.contains_key(key) {
                let value = string(location, key, "发现定位")?;
                if value.chars().any(char::is_control) {
                    return Err(format!("{finding_id}: 定位含控制字符"));
                }
            }
        }
    }
    Ok(())
}

fn safe_relative_path(path: &str) -> bool {
    if path.starts_with('/') || path.starts_with('\\') || path.contains('\\') {
        return false;
    }
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return false;
    }
    path.split('/')
        .all(|component| !matches!(component, "" | ".."))
}

fn validate_checker_statuses(
    root: &Map<String, Value>,
    require_build_root: bool,
) -> Result<(), String> {
    let Some(items) = root.get("checker_statuses") else {
        if root["schema_version"] == "1.0" {
            return Ok(());
        }
        return Err("运行报告缺 checker_statuses".into());
    };
    let items = items.as_array().ok_or("checker_statuses 必须是数组")?;
    let mut ids = BTreeSet::new();
    for item in items {
        let item = object(item, "检查器状态")?;
        keys(
            item,
            &[
                "checker_id",
                "build_root",
                "category",
                "configuration",
                "configuration_ref",
                "run_status",
                "summary",
                "next_action",
            ],
            "检查器状态",
        )?;
        let id = string(item, "checker_id", "检查器状态")?;
        let build_root = match item.get("build_root") {
            Some(_) => string(item, "build_root", "检查器状态")?,
            None if require_build_root => return Err(format!("{id}: 缺构建根")),
            None => ".",
        };
        if !safe_relative_path(build_root) {
            return Err(format!("{id}: 构建根必须是项目内相对路径"));
        }
        if !ids.insert((build_root, id)) {
            return Err(format!("重复检查器状态：{build_root}/{id}"));
        }
        let category = string(item, "category", "检查器状态")?;
        if !CHECK_CATEGORIES.contains(&category) {
            return Err(format!("{id}: 未知检查类别"));
        }
        let configuration = string(item, "configuration", "检查器状态")?;
        if !matches!(
            configuration,
            "configured" | "missing" | "invalid" | "unknown"
        ) {
            return Err(format!("{id}: 非法配置状态"));
        }
        let run_status = string(item, "run_status", "检查器状态")?;
        if !matches!(run_status, "not_run" | "passed" | "findings" | "tool_error") {
            return Err(format!("{id}: 非法运行状态"));
        }
        if configuration != "configured" && run_status != "not_run" {
            return Err(format!("{id}: 未配置检查器不能声称运行结果"));
        }
        string(item, "summary", "检查器状态")?;
        if item.contains_key("configuration_ref") {
            string(item, "configuration_ref", "检查器状态")?;
        }
        if configuration != "configured" || item.contains_key("next_action") {
            string(item, "next_action", "检查器状态")?;
        }
    }
    Ok(())
}

fn validate_identities(root: &Map<String, Value>) -> Result<BTreeSet<String>, String> {
    let identities = object(root.get("identities").ok_or("缺 identities")?, "identities")?;
    keys(identities, &["content", "policy", "tools"], "identities")?;
    let content = object(identities.get("content").ok_or("缺内容身份")?, "内容身份")?;
    keys(content, &["kind", "digest"], "内容身份")?;
    if !matches!(
        string(content, "kind", "内容身份")?,
        "working_tree" | "git_index" | "git_ref" | "ci"
    ) {
        return Err("未知内容身份类别".into());
    }
    digest(content, "digest", "内容身份")?;
    let policy = object(identities.get("policy").ok_or("缺政策身份")?, "政策身份")?;
    keys(policy, &["source", "revision", "digest"], "政策身份")?;
    string(policy, "source", "政策身份")?;
    string(policy, "revision", "政策身份")?;
    digest(policy, "digest", "政策身份")?;
    let mut tool_ids = BTreeSet::new();
    for item in array(identities, "tools", "identities")? {
        let item = object(item, "工具身份")?;
        keys(item, &["id", "version", "binary_sha256"], "工具身份")?;
        let id = string(item, "id", "工具身份")?;
        string(item, "version", "工具身份")?;
        digest(item, "binary_sha256", "工具身份")?;
        if !tool_ids.insert(id.to_owned()) {
            return Err(format!("重复工具身份：{id}"));
        }
    }
    Ok(tool_ids)
}

fn version(root: &Map<String, Value>) -> Result<u32, String> {
    let version = string(root, "schema_version", "运行报告")?;
    let Some((major, minor)) = version.split_once('.') else {
        return Err("报告协议版本格式错误".into());
    };
    if major != "1" || minor.is_empty() {
        return Err("未知报告协议版本".into());
    }
    let minor = minor.parse::<u32>().map_err(|_| "未知报告协议版本")?;
    if minor > 4 {
        return Err("未知报告协议版本".into());
    }
    Ok(minor)
}

fn extensions(value: Option<&Value>) -> Result<(), String> {
    if let Some(value) = value {
        let object = object(value, "extensions")?;
        if object
            .keys()
            .any(|key| !key.starts_with("x-") || key.len() < 3)
        {
            return Err("extensions 只能使用 x- 前缀".into());
        }
    }
    Ok(())
}

fn digest(object: &Map<String, Value>, key: &str, label: &str) -> Result<(), String> {
    let value = string(object, key, label)?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label}.{key} 必须是小写 SHA-256"));
    }
    Ok(())
}

fn unique_strings<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<Vec<&'a str>, String> {
    let items = array(object, key, label)?;
    let mut found = BTreeSet::new();
    let mut output = Vec::with_capacity(items.len());
    for value in items {
        let text = value
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| format!("{label}.{key} 必须为非空字符串数组"))?;
        if !found.insert(text) {
            return Err(format!("{label}.{key} 含重复项"));
        }
        output.push(text);
    }
    Ok(output)
}

fn array<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a Vec<Value>, String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.{key} 必须是数组"))
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} 必须是对象"))
}

fn keys(object: &Map<String, Value>, allowed: &[&str], label: &str) -> Result<(), String> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{label} 包含未知字段：{key}"));
        }
    }
    Ok(())
}

fn string<'a>(object: &'a Map<String, Value>, key: &str, label: &str) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{label} 缺少非空 {key}"))
}
