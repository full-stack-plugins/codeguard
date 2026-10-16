use super::{ProjectionCheckerPin, ProjectionContext, UnboundNativeDiagnostic};
use codeguard_core::{Completion, Finding, FindingLocation, ObligationEvidence, ObligationResult};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
/// Java原生专业格式映射；只复用既有原生解析器，不实现新的质量规则。
pub(super) fn extract(
    document: &Value,
    context: &ProjectionContext,
    pin: &ProjectionCheckerPin,
) -> (
    ObligationEvidence,
    Vec<String>,
    Vec<UnboundNativeDiagnostic>,
) {
    let javadoc = pin.binding.checker_id == "java.maven.javadoc";
    let native = &document["native_results"][if javadoc { "java_javadoc" } else { "java_p3c" }];
    let mut issues = vec!["Java native frozen coverage/production identity not proven".into()];
    let mut findings = Vec::new();
    let mut unbound = Vec::new();
    let mut observed = BTreeSet::new();
    let root_valid = native["checker_id"] == pin.binding.checker_id
        && native["authority"] == "local_unverified"
        && native["delivery_decision"] == "not_evaluated"
        && if javadoc {
            native["report_type"] == "java_javadoc_project_probe"
                && native["schema_version"] == "0.5.0"
        } else {
            native["report_type"] == "java_p3c_project_observation"
                && native["schema_version"] == "0.2.0"
        };
    if !root_valid {
        issues.push("Java checker/profile mismatch or missing".into());
    }
    let execution = context
        .java_executions
        .iter()
        .find(|e| e.checker_id.as_str() == pin.binding.checker_id);
    let sources = execution
        .map(|e| {
            e.sources
                .iter()
                .map(|s| (s.native_path.clone(), s.bytes.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    // 只枚举公开契约中明确的Java层级，不能递归搜索任意名为findings的数组。
    let observations: Vec<&Value> = if javadoc {
        native["maven_multifile_probes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| &p["observation"])
            .collect()
    } else {
        native["files"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| &p["observation"])
            .collect()
    };
    for observation in observations {
        let artifact = observation["maven_tool_sha256"].as_str()
            == pin.artifact_digest.as_str().strip_prefix("sha256:");
        let profile = if javadoc {
            observation["report_type"] == "maven_javadoc_multifile_probe"
                && observation["schema_version"] == "0.2.0"
        } else {
            observation["report_type"] == "java_p3c_local_feedback"
                && observation["schema_version"] == "0.2.0"
                && (observation["local_status"] == "findings_observed_untrusted"
                    || (observation["local_status"] == "incomplete"
                        && observation["reason"] == "native_execution_failed"))
        };
        let javadoc_parsed = if javadoc {
            execution.map(|e| {
                codeguard_adapters::parse_detailed_maven_javadoc_output(
                    &e.log,
                    e.native_exit,
                    e.parser_version.as_str(),
                    &sources,
                )
            })
        } else {
            None
        };
        let pmd_parsed = if !javadoc {
            execution.map(|e| codeguard_adapters::parse_pmd_xml(&e.log, e.parser_version.as_str()))
        } else {
            None
        };
        for row in observation["findings"].as_array().into_iter().flatten() {
            let path = row["path"]
                .as_str()
                .or_else(|| observation["path"].as_str())
                .unwrap_or("");
            let rule = row["rule_id"].as_str().unwrap_or("");
            let line = row["line"].as_u64().and_then(|n| u32::try_from(n).ok());
            let column = row["column"].as_u64().and_then(|n| u32::try_from(n).ok());
            let severity = row["severity"]
                .as_str()
                .or_else(|| row["level"].as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    row["native_priority"]
                        .as_u64()
                        .map(|p| format!("native_priority:{p}"))
                        .unwrap_or("UNKNOWN".into())
                });
            let obligation = context
                .plan
                .obligations
                .iter()
                .find(|o| o.obligation_id.as_str() == pin.binding.obligation_id);
            let source = execution.and_then(|e| {
                e.sources.iter().find(|s| {
                    s.target.path.to_bytes() == path.as_bytes()
                        && obligation.is_some_and(|o| o.targets.contains(&s.target))
                })
            });
            let mapping = pin
                .native_rules
                .iter()
                .find(|m| m.native_rule_id.as_str() == rule);
            let parsed_match = if javadoc {
                javadoc_parsed.as_ref().is_some_and(|p| {
                    p.diagnostics.iter().any(|d| {
                        source.is_some_and(|s| d.path == s.native_path)
                            && d.rule_id == rule
                            && Some(d.line) == line
                            && Some(d.column) == column
                    })
                })
            } else {
                pmd_parsed.as_ref().is_some_and(|p| {
                    // 版本不符否定解析器身份，不能把该版本保留的审计诊断当确认发现；
                    // 同版本处理失败仍保留已核对的前序阳性，不一刀切拒绝Incomplete。
                    p.reason != Some("pmd_version_mismatch")
                        && p.diagnostics.iter().any(|d| {
                            source.is_some_and(|s| d.filename == s.native_path)
                                && d.rule == rule
                                && Some(d.beginline) == line
                                && Some(d.begincolumn) == column
                        })
                })
            };
            let selected = observation["declared_rulesets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            let p3c_rule_valid = javadoc
                || codeguard_adapters::p3c_rule_in_selected_rulesets(
                    &selected,
                    row["ruleset"].as_str().unwrap_or(""),
                    rule,
                );
            let source_digest_match = javadoc
                || source.is_some_and(|s| {
                    observation["source_sha256"].as_str()
                        == s.target.input_digest.as_str().strip_prefix("sha256:")
                });
            let bound = root_valid
                && artifact
                && profile
                && source.is_some()
                && mapping.is_some()
                && parsed_match
                && source_digest_match
                && p3c_rule_valid;
            if !bound {
                unbound.push(UnboundNativeDiagnostic {
                    checker_id: pin.binding.checker_id.clone(),
                    native_rule_id: rule.into(),
                    path: path.into(),
                    line,
                    column,
                    severity,
                    reason: "independent_source_log_rule_checker_binding_missing_or_mismatched"
                        .into(),
                });
                issues.push(format!(
                    "unbound Java diagnostic retained: {rule} at {path}"
                ));
                continue;
            }
            let id = format!(
                "CG-java-{:x}",
                Sha256::digest(format!(
                    "{}\0{}\0{}\0{:?}\0{:?}",
                    pin.binding.checker_id, rule, path, line, column
                ))
            );
            findings.push(Finding {
                id,
                native_rule_id: rule.into(),
                tool_id: pin.binding.tool_id.clone(),
                severity,
                gate_impact: mapping.expect("bound mapping").gate_impact,
                message: row["rule_summary"]
                    .as_str()
                    .unwrap_or("原生Java诊断；参见私有日志与规则说明")
                    .into(),
                obligation_id: pin.binding.obligation_id.clone(),
                native_identity: None,
                locations: vec![FindingLocation::Source {
                    path: path.into(),
                    line,
                    column,
                }],
            });
            observed.insert(path.to_string());
        }
    }
    (
        ObligationEvidence {
            result: ObligationResult {
                id: pin.binding.obligation_id.clone(),
                completion: Completion::Incomplete,
                reason: Some("java_native_frozen_coverage_not_proven".into()),
                findings,
            },
            observed_targets: observed.into_iter().collect(),
        },
        issues,
        unbound,
    )
}
