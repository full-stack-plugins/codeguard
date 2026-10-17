use super::{ProjectionContext, ProjectionResult};
use codeguard_core::{
    Completion, DeliveryInput, Finding, GateImpact, ObligationEvidence, ObligationResult,
    ObligationSpec, evaluate_delivery,
};
use guard_protocol::{
    Assessment, Coverage, DomainOwner, ExecutionStatus, Extensions, GuardResult, Identifier,
    ProtocolDocument, Remediation, Verdict,
};
use serde_json::Value;
use sha2::{Digest as Hasher, Sha256};
use std::collections::BTreeSet;
fn ident(v: &str) -> Result<Identifier, String> {
    Identifier::new(v)
}
fn rows(document: &Value) -> Result<Vec<ObligationEvidence>, String> {
    let mut output = Vec::new();
    for row in document["results"]
        .as_array()
        .ok_or("missing obligation results")?
    {
        let id = row["obligation_id"]
            .as_str()
            .ok_or("missing obligation ID")?;
        let mut findings = Vec::new();
        for f in row["findings"].as_array().ok_or("missing findings")? {
            findings.push(serde_json::from_value::<Finding>(f.clone()).map_err(|e| e.to_string())?);
        }
        let completion = serde_json::from_value::<Completion>(row["completion"].clone())
            .map_err(|e| e.to_string())?;
        let observed_targets = row["coverage"]["observed"]
            .as_array()
            .ok_or("missing observed targets")?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or("invalid observed target".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        output.push(ObligationEvidence {
            result: ObligationResult {
                id: id.into(),
                completion,
                reason: row["reason"].as_str().map(str::to_owned),
                findings,
            },
            observed_targets,
        });
    }
    Ok(output)
}
fn expected(context: &ProjectionContext) -> Result<Vec<ObligationSpec>, String> {
    context
        .plan
        .obligations
        .iter()
        .map(|o| {
            let targets = o
                .targets
                .iter()
                .map(|t| {
                    String::from_utf8(t.path.to_bytes()).map_err(|_| {
                        "legacy string targets cannot represent raw Git path".to_string()
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ObligationSpec {
                id: o.obligation_id.as_str().into(),
                expected_targets: targets,
            })
        })
        .collect()
}
/// 双读原始报告与公共结果；上下文必须由控制端独立提供，不从报告复制批准范围。
/// 未知版本/重复键/坏字段/缺运行与冻结输入均明确incomplete，原件始终保持原字节。
pub fn project_raw<'a>(
    bytes: &'a [u8],
    context: Option<&ProjectionContext>,
) -> ProjectionResult<'a> {
    let digest = guard_protocol::Digest::new(format!("sha256:{:x}", Sha256::digest(bytes)))
        .expect("fixed SHA256 representation");
    let mut input = DeliveryInput {
        approval_scope: None,
        full_project: false,
        discovery_complete: false,
        trusted_bindings_verified: false,
        gate_time_unix: None,
        policy_revision: None,
        frozen_obligation_ids: Vec::new(),
        obligations: Vec::new(),
        evidence: Vec::new(),
        allowlist_dispositions: Vec::new(),
        native_checker_bindings: Vec::new(),
    };
    let mut projected = ProjectionResult {
        raw: bytes,
        raw_digest: digest,
        gate: evaluate_delivery(&input),
        findings: Vec::new(),
        results: Vec::new(),
        issues: Vec::new(),
        unbound: Vec::new(),
    };
    let parsed = (|| -> Result<Value, String> {
        let canonical = guard_protocol::canonicalize(bytes)?;
        let document: Value = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
        let version = document["schema_version"]
            .as_str()
            .ok_or("missing source schema version")?;
        match document["report_type"].as_str() {
            Some("run_report") => {
                if !["1.0", "1.1", "1.2", "1.3", "1.4"].contains(&version) {
                    return Err("unknown exact run_report version".into());
                }
                let report = crate::run_report::parse_run_report(bytes)?;
                input.evidence = rows(report.document())?;
                input.full_project = document["request"]["selection"]["language"] == "all";
            }
            Some("check_feedback") => {
                super::feedback_schema::validate(&document, version)?;
                if document["authority"] != "local_unverified"
                    || !["incomplete", "not_evaluated"]
                        .contains(&document["delivery_decision"].as_str().unwrap_or(""))
                {
                    return Err("feedback cannot claim authority or delivery allow".into());
                }
                input.full_project = document["selection"] == "all";
                projected.issues.push("native_feedback_lacks_frozen_obligation_evidence; native_results retained in private original".into());
            }
            _ => return Err("unknown report type; projection incomplete".into()),
        }
        Ok(document)
    })();
    let malformed = parsed.is_err();
    projected.findings = input
        .evidence
        .iter()
        .flat_map(|e| e.result.findings.clone())
        .collect();
    match parsed {
        Err(error) => projected.issues.push(error),
        Ok(document) => {
            if let Some(context) = context {
                let projection = (|| -> Result<Vec<GuardResult>, String> {
                    if context.raw_evidence.digest != projected.raw_digest
                        || context.raw_evidence.size != bytes.len() as u64
                    {
                        return Err("private original evidence digest/size mismatch".into());
                    }
                    if document["report_type"] == "check_feedback" {
                        let (evidence, issues, unbound) =
                            super::feedback_native::extract(&document, context);
                        projected.unbound = unbound;
                        input.evidence = evidence;
                        projected.issues.extend(issues);
                    }
                    input.obligations = expected(context)?;
                    input.frozen_obligation_ids =
                        input.obligations.iter().map(|o| o.id.clone()).collect();
                    let declared: BTreeSet<_> = input
                        .evidence
                        .iter()
                        .map(|o| o.result.id.as_str())
                        .collect();
                    let frozen: BTreeSet<_> = input
                        .frozen_obligation_ids
                        .iter()
                        .map(String::as_str)
                        .collect();
                    if !declared.is_subset(&frozen) {
                        return Err("source result outside frozen obligation set".into());
                    }
                    if declared != frozen {
                        projected
                            .issues
                            .push("required frozen obligation results missing".into());
                    }
                    let mut results = Vec::new();
                    for obligation in &context.plan.obligations {
                        let id = obligation.obligation_id.as_str();
                        if document["report_type"] == "run_report" {
                            let pin = context
                                .checker_pins
                                .iter()
                                .find(|pin| {
                                    pin.verifier_id == obligation.verifier_id
                                        && pin.binding.obligation_id == id
                                })
                                .ok_or("native checker pin missing")?;
                            let native = document["identities"]["tools"]
                                .as_array()
                                .ok_or("native tool identities missing")?
                                .iter()
                                .find(|tool| tool["id"] == pin.binding.tool_id)
                                .ok_or("approved native tool not declared")?;
                            if native["version"] != pin.version.as_str()
                                || native["binary_sha256"]
                                    != pin
                                        .artifact_digest
                                        .as_str()
                                        .strip_prefix("sha256:")
                                        .ok_or("invalid native tool digest")?
                            {
                                return Err(
                                    "native tool version/artifact differs from independent pin"
                                        .into(),
                                );
                            }
                            if input
                                .evidence
                                .iter()
                                .find(|e| e.result.id == id)
                                .is_some_and(|e| {
                                    e.result
                                        .findings
                                        .iter()
                                        .any(|f| f.tool_id != pin.binding.tool_id)
                                })
                            {
                                return Err(
                                    "finding tool differs from approved obligation checker".into(),
                                );
                            }
                        }

                        let rule = context
                            .rules
                            .iter()
                            .find(|r| r.rule_id == obligation.rule_id)
                            .ok_or("approved rule missing")?;
                        let row = input.evidence.iter().find(|e| e.result.id == id);
                        let observed = row
                            .map(|r| {
                                r.observed_targets
                                    .iter()
                                    .map(String::as_str)
                                    .collect::<BTreeSet<_>>()
                            })
                            .unwrap_or_default();
                        let expected = input
                            .obligations
                            .iter()
                            .find(|o| o.id == id)
                            .ok_or("missing frozen targets")?
                            .expected_targets
                            .iter()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>();
                        let source_row = document["results"]
                            .as_array()
                            .and_then(|rs| rs.iter().find(|r| r["obligation_id"] == id));
                        let source_expected = source_row
                            .and_then(|r| r["coverage"]["expected"].as_array())
                            .map(|v| v.iter().filter_map(Value::as_str).collect::<BTreeSet<_>>())
                            .unwrap_or_default();
                        let complete = row
                            .is_some_and(|r| r.result.completion == Completion::Complete)
                            && !expected.is_empty()
                            && observed == expected
                            && source_expected == expected;
                        if !complete {
                            projected
                                .issues
                                .push(format!("{id}: frozen target coverage not fully proven"));
                        }
                        let findings = row.map(|r| r.result.findings.as_slice()).unwrap_or(&[]);
                        let known_violation = findings
                            .iter()
                            .any(|f| f.gate_impact == GateImpact::Blocking);
                        let finding_ids = findings
                            .iter()
                            .map(|f| ident(&f.id))
                            .collect::<Result<Vec<_>, _>>()?;
                        let mut execution = context.run.execution.clone();
                        if row.is_none() {
                            execution.status = ExecutionStatus::NotRun;
                            execution.started_at = None;
                            execution.finished_at = None;
                            execution.exit_code = None;
                        }
                        let verdict = if known_violation {
                            Verdict::Violation
                        } else if complete
                            && findings.is_empty()
                            && execution.status == ExecutionStatus::Completed
                        {
                            Verdict::Satisfied
                        } else {
                            Verdict::Unknown
                        };
                        let coverage = if complete {
                            Coverage::Complete
                        } else if observed.is_empty() {
                            Coverage::Unknown
                        } else {
                            Coverage::Partial
                        };
                        let value = GuardResult {
                            schema_version: context.plan.schema_version.clone(),
                            result_id: ident(&format!(
                                "cg-{}-{}",
                                id,
                                &projected.raw_digest.as_str()[7..23]
                            ))?,
                            run_id: context.run.run_id.clone(),
                            operation_id: context.run.operation_id.clone(),
                            task_id: context.task.task_id.clone(),
                            subject_digest: context.plan.subject_digest.clone(),
                            contract_digest: context.plan.contract_digest.clone(),
                            policy_digest: context.plan.policy_digest.clone(),
                            producer: context.producer.clone(),
                            guard: DomainOwner::Codeguard,
                            rule_id: rule.rule_id.clone(),
                            mode: rule.mode,
                            severity: rule.severity,
                            execution,
                            assessment: Assessment {
                                verdict,
                                coverage,
                                applicability_evidence_refs: Vec::new(),
                                finding_ids,
                            },
                            evidence_refs: {
                                let mut refs = vec![context.raw_evidence.clone()];
                                for native in &context.java_executions {
                                    if context.checker_pins.iter().any(|pin| {
                                        pin.binding.obligation_id == id
                                            && pin.binding.checker_id == native.checker_id.as_str()
                                    }) {
                                        refs.push(native.evidence.clone());
                                        refs.extend(
                                            native
                                                .sources
                                                .iter()
                                                .map(|source| source.evidence.clone()),
                                        );
                                    }
                                }
                                refs
                            },
                            remediation: Remediation {
                                summary: ident(
                                    "Review original CodeGuard findings and re-run the approved verifier",
                                )?,
                                argv: Vec::new(),
                            },
                            limitations: {
                                let mut limitations = vec![
                                    ident("local_unverified")?,
                                    ident("source_report_does_not_authorize_delivery")?,
                                ];
                                if document["report_type"] == "check_feedback" {
                                    limitations
                                        .push(ident("native_feedback_frozen_coverage_not_proven")?);
                                    limitations
                                        .push(ident("native_tool_version_may_not_be_reported")?);
                                    if context.checker_pins.iter().any(|pin| {
                                        pin.binding.obligation_id == id
                                            && pin.binding.checker_id == "rust.cargo_audit"
                                    }) {
                                        limitations.push(ident(
                                            "native_advisory_database_provenance_unverified",
                                        )?);
                                    }
                                }
                                limitations
                            },
                            extensions: Extensions::default(),
                        };
                        guard_protocol::encode(&ProtocolDocument::GuardResult(value.clone()))?;
                        results.push(value);
                    }
                    Ok(results)
                })();
                match projection {
                    Ok(results) => projected.results = results,
                    Err(error) => projected.issues.push(error),
                }
            } else {
                projected
                    .issues
                    .push("independent frozen plan and actual execution context missing".into());
            }
        }
    }
    // 永远不从旧trusted字段、策略source字符串或parse成功设置身份核验true。
    projected.findings = input
        .evidence
        .iter()
        .flat_map(|e| e.result.findings.clone())
        .collect();
    projected.gate = evaluate_delivery(&input);
    if malformed {
        projected.gate.decision = codeguard_core::DeliveryDecision::Incomplete;
    }
    projected
}
