//! Exact protected mappings into existing GuardEngine relations; no native qualification.
use super::{
    reader::NativeEvidence,
    scope::{FrozenObligations, ScopeAssessment, assess_scope},
};
use guardengine::integration::FactBudget;
use guardengine::{
    API_VERSION, AnalyzerIdentity, Completeness, GuardAssertion, GuardContract, GuardFacts,
    GuardReport, GuardSubject,
};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Source {
    Finding {
        tool_id: String,
        native_rule_id: String,
    },
    Gap {
        detail: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    source: Source,
    rule_id: String,
}
/// Caller must obtain this mapping and the contract from protected policy, not the candidate.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedMapping {
    version: String,
    entries: Vec<Entry>,
}
impl ProtectedMapping {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 1024 * 1024 {
            return Err("mapping exceeds limit");
        }
        let value =
            codeguard_adapters::parse_unique_json(bytes).map_err(|_| "invalid mapping JSON")?;
        let mapping: Self = serde_json::from_value(value).map_err(|_| "invalid mapping fields")?;
        mapping.validate()?;
        Ok(mapping)
    }
    pub(super) fn validate_ruff_scope(&self, contract: &GuardContract) -> Result<(), &'static str> {
        if contract.spec.rules.len() != 1 || self.entries.len() != 2
            || self.entries.iter().any(|entry| {
                entry.rule_id != contract.spec.rules[0].id
                    || !matches!(&entry.source,
                        Source::Finding {tool_id, native_rule_id} if tool_id == "ruff" && native_rule_id == "F401")
                       && !matches!(&entry.source, Source::Gap {detail} if detail == "ruff_f401_scope_incomplete")
            }) {
            return Err("contract exceeds qualified Ruff F401 scope");
        }
        Ok(())
    }
    pub(super) fn validate_references(&self, contract: &GuardContract) -> Result<(), &'static str> {
        if self.entries.iter().any(|entry| {
            !contract
                .spec
                .rules
                .iter()
                .any(|rule| rule.id == entry.rule_id)
        }) {
            return Err("mapping references unknown contract rule");
        }
        Ok(())
    }
    pub(super) fn validate(&self) -> Result<(), &'static str> {
        if self.version != "codeguard.mapping/v1alpha1" || self.entries.is_empty() {
            return Err("unsupported mapping");
        }
        let mut seen = BTreeSet::new();
        for entry in &self.entries {
            let source_fields = match &entry.source {
                Source::Finding {
                    tool_id,
                    native_rule_id,
                } => vec![tool_id, native_rule_id],
                Source::Gap { detail } => vec![detail],
            };
            if !exact(&entry.rule_id)
                || source_fields.iter().any(|s| !exact(s))
                || !seen.insert(&entry.source)
            {
                return Err("mapping must be exact and unique");
            }
        }
        Ok(())
    }
}
pub(super) fn exact(s: &str) -> bool {
    !s.trim().is_empty() && !s.contains(['*', '?', '[', ']'])
}
/// Immutable result; native bytes remain separate from the engine wire.
pub struct Projection {
    pub(crate) required_targets: std::collections::BTreeMap<String, Vec<String>>,
    pub(crate) native_run_id: String,
    pub(crate) facts: GuardFacts,
    pub(crate) report: GuardReport,
    pub(crate) contract_bytes: Vec<u8>,
    pub(crate) domain: Vec<u8>,
    pub(crate) required_scopes: Vec<String>,
}
impl Projection {
    pub fn facts(&self) -> &GuardFacts {
        &self.facts
    }
    pub fn report(&self) -> &GuardReport {
        &self.report
    }
    pub fn domain_bytes(&self) -> &[u8] {
        &self.domain
    }
}
pub fn project(
    evidence: &NativeEvidence,
    obligations: &FrozenObligations,
    mapping: &ProtectedMapping,
    contract: &GuardContract,
    subject: GuardSubject,
) -> Result<Projection, &'static str> {
    mapping.validate()?;
    if matches!(evidence.report().exit_code, 4 | 130) {
        return Err("failed native execution cannot be evaluated");
    }
    if subject.snapshot_digest
        != format!(
            "sha256:{}",
            evidence.report().document()["identities"]["content"]["digest"]
                .as_str()
                .ok_or("missing native snapshot")?
        )
    {
        return Err("native snapshot differs from projection subject");
    }
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
    if mapping.entries.iter().any(|entry| {
        !contract
            .spec
            .rules
            .iter()
            .any(|rule| rule.id == entry.rule_id)
    }) {
        return Err("mapping references unknown contract rule");
    }
    let ScopeAssessment::Partial { gaps } = assess_scope(evidence, obligations);
    let mut budget = FactBudget::new();
    for result in evidence.report().document()["results"]
        .as_array()
        .ok_or("missing results")?
    {
        for finding in result["findings"].as_array().ok_or("missing findings")? {
            let tool = finding["tool_id"].as_str().ok_or("missing tool")?;
            let native_rule = finding["native_rule_id"].as_str().ok_or("missing rule")?;
            let id = finding["id"].as_str().ok_or("missing finding")?;
            push_mapped_relation(
                &mut budget,
                mapping,
                contract,
                |source| {
                    matches!(source, Source::Finding { tool_id, native_rule_id }
                    if tool_id == tool && native_rule_id == native_rule)
                },
                &["native:", id],
            )?;
        }
    }
    for gap in &gaps {
        push_mapped_relation(
            &mut budget,
            mapping,
            contract,
            |source| matches!(source, Source::Gap { detail } if detail == gap),
            &["scope:", gap],
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
        completeness: Completeness::Partial,
        facts,
        diagnostics: gaps,
    };
    let report = guardengine::integration::evaluate_bounded(contract, &facts)
        .map_err(|_| "engine evaluation rejected projection")?;
    // JSON is also valid YAML, accepted by the engine's strict contract loader.
    let contract_bytes = serde_json::to_vec(contract).map_err(|_| "cannot serialize contract")?;
    Ok(Projection {
        required_targets: obligations.frozen_targets(),
        native_run_id: evidence.report().run_id.clone(),
        facts,
        report,
        contract_bytes,
        domain: evidence.raw_bytes().to_vec(),
        required_scopes: obligations.scope_ids(),
    })
}

// Mapping and contract fields stay borrowed until the shared builder accepts their cost.
pub(super) fn push_mapped_relation(
    budget: &mut FactBudget,
    mapping: &ProtectedMapping,
    contract: &GuardContract,
    matches_source: impl Fn(&Source) -> bool,
    source_parts: &[&str],
) -> Result<(), &'static str> {
    let entry = mapping
        .entries
        .iter()
        .find(|entry| matches_source(&entry.source))
        .ok_or("unmapped native finding or scope gap")?;
    let rule = contract
        .spec
        .rules
        .iter()
        .find(|rule| rule.id == entry.rule_id)
        .ok_or("mapping references unknown contract rule")?;
    let GuardAssertion::ForbidRelation {
        subject,
        predicate,
        object,
    } = &rule.assertion;
    budget
        .push_relation_with_source_parts(subject, predicate, object, source_parts)
        .map_err(|_| "fact construction rejected projection")
}
