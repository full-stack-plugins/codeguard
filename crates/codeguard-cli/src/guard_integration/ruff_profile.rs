//! One-file F401 observations from the real, selected-file Ruff feedback producer.
//! This profile never changes the aggregate command's native exit3 or gate status.
use super::scope::FrozenObligations;
use guardengine::Completeness;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const PROFILE: &str = "codeguard.ruff-f401/linux-x86_64/v1alpha1";
pub const CONFIG: &str =
    "[tool.ruff]\ntarget-version = \"py311\"\n[tool.ruff.lint]\nselect = [\"F401\"]\n";
/// Official PyPI Ruff0.16.8 Linux x86_64 executable, recorded wheel provenance.
pub const TOOL_SHA256: &str = "f3eb080f0173e1a5884fabcc2c836f5a0ebea35fccf252e718fc71b17f821ef7";

/// Protected caller inputs. Fields stay private; there is no qualification flag.
pub struct RuffF401Policy {
    target: String,
    source_sha256: String,
    producer_sha256: String,
    scope_id: String,
}
impl RuffF401Policy {
    pub fn new(
        target: &str,
        source_sha256: &str,
        producer_sha256: &str,
    ) -> Result<Self, &'static str> {
        if target.len() > 256
            || !target.ends_with(".py")
            || target.starts_with('/')
            || target
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || target.chars().any(|c| c.is_control() || c == '\\')
            || !digest(source_sha256)
            || !digest(producer_sha256)
        {
            return Err("invalid frozen Ruff inputs");
        }
        let identity = serde_json::to_vec(&(
            PROFILE,
            target,
            source_sha256,
            producer_sha256,
            TOOL_SHA256,
            CONFIG,
        ))
        .map_err(|_| "invalid policy identity")?;
        Ok(Self {
            target: target.into(),
            source_sha256: source_sha256.into(),
            producer_sha256: producer_sha256.into(),
            scope_id: format!("python/ruff-f401/{:x}", Sha256::digest(identity)),
        })
    }
    pub fn obligations(&self) -> FrozenObligations {
        FrozenObligations::new(BTreeMap::from([(
            self.scope_id.clone(),
            vec![self.target.clone()],
        )]))
        .expect("bounded fixed profile identity")
    }
    pub fn source_digest(&self) -> String {
        format!("sha256:{}", self.source_sha256)
    }
}
/// Strict observations, still dependent on caller-captured provenance and policy.
pub struct RuffEvidence {
    raw: Vec<u8>,
    run_id: String,
    source_sha256: String,
    target: String,
    scope_id: String,
    findings: Vec<String>,
    gaps: Vec<String>,
}
impl RuffEvidence {
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw
    }
    pub fn native_exit(&self) -> u8 {
        3
    }
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }
    pub fn gaps(&self) -> &[String] {
        &self.gaps
    }
    pub fn completeness(&self) -> Completeness {
        if self.gaps.is_empty() {
            Completeness::Complete
        } else {
            Completeness::Partial
        }
    }
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn keys(value: &Value, allowed: &[&str]) -> Result<(), &'static str> {
    let object = value.as_object().ok_or("expected feedback object")?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("unknown feedback field");
    }
    Ok(())
}
fn exact_keys(value: &Value, allowed: &[&str]) -> Result<(), &'static str> {
    keys(value, allowed)?;
    if value.as_object().unwrap().len() != allowed.len() {
        return Err("missing feedback field");
    }
    Ok(())
}
fn strings(value: &Value) -> Result<Vec<&str>, &'static str> {
    value
        .as_array()
        .ok_or("expected string array")?
        .iter()
        .map(|item| item.as_str().ok_or("expected string array"))
        .collect()
}
fn optional_digest(value: &Value) -> bool {
    value.is_null() || value.as_str().is_some_and(digest)
}

/// Consume an actual `lint python --file TARGET --format=json` 0.13 report.
/// Caller supplies independently captured exit/run/producer identities. No I/O,
/// process execution, authentication, or candidate-driven policy selection occurs.
pub fn read_ruff_feedback(
    bytes: &[u8],
    policy: &RuffF401Policy,
    run_id: &str,
    process_exit: u8,
    producer_sha256: &str,
) -> Result<RuffEvidence, &'static str> {
    if bytes.len() > 1024 * 1024
        || process_exit != 3
        || run_id.is_empty()
        || run_id.len() > 128
        || !run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || producer_sha256 != policy.producer_sha256
    {
        return Err("unsupported Ruff invocation");
    }
    let root =
        codeguard_adapters::parse_unique_json(bytes).map_err(|_| "invalid Ruff feedback JSON")?;
    exact_keys(
        &root,
        &[
            "adapter_sha256",
            "backlog_status",
            "backlog_sync",
            "checker_configurations",
            "command_status",
            "delivery_decision",
            "execution_budget",
            "exit_code",
            "files",
            "incomplete_reasons",
            "language",
            "local_scan_complete",
            "native_tool_version",
            "next",
            "operation",
            "repair_brief_reason",
            "repair_brief_status",
            "report_type",
            "requested_paths",
            "rulepack_approval",
            "run_id",
            "scan_scope",
            "schema_version",
            "scope",
            "tool_approval",
            "workspace_binding",
            "workspace_id",
        ],
    )?;
    if root["schema_version"] != "0.13.0"
        || root["report_type"] != "python_lint_feedback"
        || root["language"] != "python"
        || root["operation"] != "lint"
        || root["run_id"] != run_id
        || root["exit_code"] != 3
        || root["command_status"] != "incomplete"
        || root["delivery_decision"] != "not_evaluated"
        || root["scan_scope"] != "selected_files"
        || root["scope"] != "local_native_scan_only"
        || strings(&root["requested_paths"])? != [policy.target.as_str()]
        || root["backlog_status"] != "not_synced_scoped"
        || !root["backlog_sync"].is_null()
        || root["repair_brief_status"] != "unavailable"
        || root["repair_brief_reason"] != "not_synced_scoped"
        || !root["next"].is_null()
        || root["tool_approval"] != "unverified"
        || root["rulepack_approval"] != "unverified"
        || root["workspace_binding"] != "uninitialized"
        || !root["workspace_id"].is_null()
        || !optional_digest(&root["adapter_sha256"])
    {
        return Err("unsupported Ruff feedback profile");
    }
    exact_keys(
        &root["execution_budget"],
        &["enforcement", "source", "timeout_ms"],
    )?;
    if root["execution_budget"]["enforcement"] != "native_execution_only"
        || !root["execution_budget"]["source"].is_string()
        || !root["execution_budget"]["timeout_ms"]
            .as_u64()
            .is_some_and(|n| n > 0)
    {
        return Err("invalid execution budget");
    }
    if !(root["native_tool_version"].is_null() || root["native_tool_version"].is_string()) {
        return Err("invalid native tool version");
    }
    let mut gaps = Vec::new();
    let mut gap = |name: &str| {
        gaps.push(name.to_owned());
    };
    if root["adapter_sha256"]
        .as_str()
        .is_some_and(|hash| hash != producer_sha256)
    {
        gap("producer identity mismatch");
    }
    if root["native_tool_version"] != "ruff 0.16.8" {
        gap("Ruff tool version unavailable");
    }
    let complete = root["local_scan_complete"]
        .as_bool()
        .ok_or("invalid local completion")?;
    let reasons = strings(&root["incomplete_reasons"])?;
    if !complete || !reasons.is_empty() {
        gap("native scan incomplete");
    }
    let checkers = root["checker_configurations"]
        .as_array()
        .ok_or("invalid checker list")?;
    if checkers.len() != 1 {
        return Err("unsupported checker scope");
    }
    let checker = &checkers[0];
    exact_keys(
        checker,
        &[
            "build_root",
            "checker_id",
            "configuration",
            "configuration_ref",
            "next_action",
        ],
    )?;
    if checker["checker_id"] != "python.ruff"
        || checker["build_root"] != "."
        || checker["configuration_ref"] != "pyproject.toml"
        || !checker["next_action"].is_string()
    {
        return Err("unsupported checker identity");
    }
    if checker["configuration"] != "configured" {
        gap("native configuration unavailable");
    }
    let files = root["files"].as_array().ok_or("invalid file list")?;
    if files.len() != 1 {
        return Err("unsupported target scope");
    }
    let file = &files[0];
    exact_keys(
        file,
        &[
            "config_sha256",
            "configuration_ref",
            "findings",
            "path",
            "reason",
            "recheck_argv",
            "recheck_cwd",
            "rule_settings",
            "run_status",
            "source_sha256",
            "suppression_audit",
            "tool_sha256",
        ],
    )?;
    if file["path"] != policy.target
        || file["configuration_ref"] != "pyproject.toml"
        || strings(&file["recheck_argv"])? != ["ruff", "check", policy.target.as_str()]
        || !file["recheck_cwd"].is_string()
        || !(file["reason"].is_null() || file["reason"].is_string())
        || ["config_sha256", "source_sha256", "tool_sha256"]
            .iter()
            .any(|key| !optional_digest(&file[key]))
    {
        return Err("invalid native file identity");
    }
    if file["config_sha256"] != format!("{:x}", Sha256::digest(CONFIG.as_bytes())) {
        gap("frozen configuration mismatch");
    }
    if file["source_sha256"] != policy.source_sha256 {
        gap("frozen source mismatch");
    }
    if file["tool_sha256"] != TOOL_SHA256 {
        gap("frozen tool mismatch");
    }
    if !file["reason"].is_null() {
        gap("native file incomplete");
    }
    let settings = &file["rule_settings"];
    if settings.is_null() {
        gap("rule settings unavailable");
    } else {
        exact_keys(
            settings,
            &[
                "coverage_proven",
                "globally_enabled_mapped_rules",
                "per_file_ignores_present",
                "settings_sha256",
            ],
        )?;
        if settings["coverage_proven"] != false
            || !settings["settings_sha256"].as_str().is_some_and(digest)
            || !settings["per_file_ignores_present"].is_boolean()
        {
            return Err("invalid rule settings");
        }
        if strings(&settings["globally_enabled_mapped_rules"])? != ["F401"] {
            gap("required F401 disabled");
        }
        if settings["per_file_ignores_present"] != false {
            gap("per-file ignores unsupported");
        }
    }
    let audit = &file["suppression_audit"];
    if audit.is_null() {
        gap("suppression audit unavailable");
    } else {
        exact_keys(
            audit,
            &[
                "audit_sha256",
                "scope",
                "suppressed_diagnostic_count",
                "suppressed_rule_ids",
            ],
        )?;
        if !audit["audit_sha256"].as_str().is_some_and(digest)
            || audit["scope"] != "source_comments_only"
        {
            return Err("invalid suppression audit");
        }
        let count = audit["suppressed_diagnostic_count"]
            .as_u64()
            .ok_or("invalid suppression count")?;
        let suppressed = strings(&audit["suppressed_rule_ids"])?;
        if (count == 0) != suppressed.is_empty() {
            return Err("inconsistent suppression audit");
        }
        if count > 0 {
            gap("source suppression unsupported");
        }
    }
    let findings = file["findings"].as_array().ok_or("invalid findings")?;
    let mut ids = std::collections::BTreeSet::new();
    for finding in findings {
        exact_keys(
            finding,
            &[
                "codeguard_rule_id",
                "column",
                "finding_fingerprint",
                "finding_id",
                "line",
                "next_action",
                "path",
                "repair_hint",
                "rule_id",
                "rule_summary",
                "rulepack_sha256",
                "rulepack_status",
            ],
        )?;
        if finding["rule_id"] != "F401"
            || finding["codeguard_rule_id"] != "python.ruff.F401"
            || finding["path"] != policy.target
            || !finding["line"].as_u64().is_some_and(|n| n > 0)
            || !finding["column"].as_u64().is_some_and(|n| n > 0)
            || !finding["next_action"].is_string()
            || !finding["rule_summary"].is_string()
            || !finding["finding_fingerprint"].as_str().is_some_and(digest)
            || !finding["rulepack_sha256"].as_str().is_some_and(digest)
            || finding["rulepack_status"] != "candidate_unapproved"
        {
            return Err("unsupported finding identity");
        }
        let id = finding["finding_id"].as_str().ok_or("missing finding ID")?;
        if id.is_empty() || id.len() > 128 || !ids.insert(id.to_owned()) {
            return Err("duplicate or invalid finding ID");
        }
        keys(
            &finding["repair_hint"],
            &[
                "allowed_path",
                "closure_condition",
                "source_sha256",
                "status",
                "step",
            ],
        )?;
        if finding["repair_hint"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| !v.is_string())
        {
            return Err("invalid repair hint");
        }
    }
    let status = file["run_status"].as_str().ok_or("invalid file status")?;
    if !matches!(status, "passed" | "findings" | "suppressed" | "incomplete")
        || (status == "passed" && !findings.is_empty())
        || (status == "findings" && findings.is_empty())
    {
        return Err("inconsistent file status");
    }
    if !matches!(status, "passed" | "findings") {
        gaps.push("native file not complete".into());
    }
    if complete && status == "incomplete" {
        return Err("contradictory local completion");
    }
    gaps.sort();
    gaps.dedup();
    Ok(RuffEvidence {
        raw: bytes.to_vec(),
        run_id: run_id.into(),
        source_sha256: policy.source_sha256.clone(),
        target: policy.target.clone(),
        scope_id: policy.scope_id.clone(),
        findings: ids.into_iter().collect(),
        gaps,
    })
}

/// Project only the fixed one-file F401 profile. Aggregate native exit3 is retained
/// in the untouched domain artifact and never interpreted as engine approval.
pub fn project_ruff(
    evidence: &RuffEvidence,
    mapping: &super::projection::ProtectedMapping,
    contract: &guardengine::GuardContract,
    subject: guardengine::GuardSubject,
) -> Result<super::projection::Projection, &'static str> {
    use super::projection::{Projection, Source, exact, push_mapped_relation};
    use guardengine::integration::{FactBudget, evaluate_bounded};
    use guardengine::{API_VERSION, AnalyzerIdentity, GuardAssertion, GuardFacts};
    if subject.snapshot_digest != format!("sha256:{}", evidence.source_sha256) {
        return Err("Ruff projection differs from frozen source");
    }
    mapping.validate()?;
    mapping.validate_references(contract)?;
    mapping.validate_ruff_scope(contract)?;
    contract
        .validate()
        .map_err(|_| "invalid protected contract")?;
    for rule in &contract.spec.rules {
        let GuardAssertion::ForbidRelation {
            subject,
            predicate,
            object,
        } = &rule.assertion;
        if ![subject, predicate, object].iter().all(|s| exact(s)) {
            return Err("unsupported wildcard relation");
        }
    }
    let mut budget = FactBudget::new();
    for id in &evidence.findings {
        push_mapped_relation(
            &mut budget,
            mapping,
            contract,
            |source| matches!(source,Source::Finding{tool_id,native_rule_id} if tool_id=="ruff" && native_rule_id=="F401"),
            &["native:", id],
        )?;
    }
    if !evidence.gaps.is_empty() {
        push_mapped_relation(
            &mut budget,
            mapping,
            contract,
            |source| matches!(source,Source::Gap{detail} if detail=="ruff_f401_scope_incomplete"),
            &["scope:ruff_f401_scope_incomplete"],
        )?;
    }
    let mut facts = budget
        .finish()
        .map_err(|_| "fact construction rejected projection")?;
    facts.sort();
    facts.dedup();
    let facts = GuardFacts {
        api_version: API_VERSION.into(),
        kind: "GuardFacts".into(),
        analyzer: AnalyzerIdentity {
            id: "codeguard.native-projection".into(),
            version: "0.1.0".into(),
        },
        subject,
        completeness: evidence.completeness(),
        facts,
        diagnostics: evidence.gaps.clone(),
    };
    let report =
        evaluate_bounded(contract, &facts).map_err(|_| "engine evaluation rejected projection")?;
    Ok(Projection {
        required_targets: BTreeMap::from([(
            evidence.scope_id.clone(),
            vec![evidence.target.clone()],
        )]),
        native_run_id: evidence.run_id.clone(),
        required_scopes: vec![evidence.scope_id.clone()],
        facts,
        report,
        contract_bytes: serde_json::to_vec(contract).map_err(|_| "cannot serialize contract")?,
        domain: evidence.raw.clone(),
    })
}
