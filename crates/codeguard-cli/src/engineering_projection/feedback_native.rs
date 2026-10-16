use super::ProjectionContext;
use codeguard_core::{Completion, Finding, FindingLocation, ObligationEvidence, ObligationResult};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
/// 只解释显式已支持的原生报告字段；目标/规则集合全部来自独立上下文。
pub(super) fn extract(
    document: &Value,
    context: &ProjectionContext,
) -> (
    Vec<ObligationEvidence>,
    Vec<String>,
    Vec<super::UnboundNativeDiagnostic>,
) {
    let mut evidence = Vec::new();
    let mut issues = Vec::new();
    let mut unbound = Vec::new();
    for pin in &context.checker_pins {
        if matches!(
            pin.binding.checker_id.as_str(),
            "java.maven.javadoc" | "java.maven.p3c"
        ) {
            let (row, java_issues, java_unbound) =
                super::feedback_java::extract(document, context, pin);
            evidence.push(row);
            issues.extend(java_issues);
            unbound.extend(java_unbound);
            continue;
        }
        let profile = match pin.binding.checker_id.as_str() {
            "rust.cargo_clippy" => ("rust_lint", "rust_clippy_local_observation", "0.2.0", false),
            "rust.cargo_rustdoc" => ("rust_comments", "rustdoc_local_observation", "0.4.0", false),
            "rust.cargo_audit" => ("rust_cve", "rust_cve_local_observation", "0.1.0", true),
            _ => {
                issues.push(format!(
                    "native checker profile unsupported: {}",
                    pin.binding.checker_id
                ));
                continue;
            }
        };
        let native = &document["native_results"][profile.0];
        if native.is_null() {
            issues.push(format!(
                "native checker report missing: {}",
                pin.binding.checker_id
            ));
            continue;
        }
        if native["checker_id"] != pin.binding.checker_id
            || native["report_type"] != profile.1
            || native["schema_version"] != profile.2
        {
            issues.push(format!(
                "native checker/profile identity mismatch: {}",
                pin.binding.checker_id
            ));
            continue;
        }
        if native["authority"] != "local_unverified"
            || native["delivery_decision"] != "not_evaluated"
            || native["tool_sha256"]
                != pin
                    .artifact_digest
                    .as_str()
                    .strip_prefix("sha256:")
                    .unwrap_or("")
        {
            issues.push(format!(
                "native checker artifact/authority mismatch: {}",
                pin.binding.checker_id
            ));
            continue;
        }
        if let Some(version) = native.get("tool_version").and_then(Value::as_str) {
            if version != pin.version.as_str() {
                issues.push(format!(
                    "native checker version mismatch: {}",
                    pin.binding.checker_id
                ));
                continue;
            }
        } else {
            issues.push(format!(
                "native version not reported; artifact pin only: {}",
                pin.binding.checker_id
            ));
        }
        let Some(obligation) = context
            .plan
            .obligations
            .iter()
            .find(|o| o.obligation_id.as_str() == pin.binding.obligation_id)
        else {
            issues.push("native checker obligation mapping missing".into());
            continue;
        };
        let Some(rows) = native["findings"].as_array() else {
            issues.push("native findings collection missing".into());
            continue;
        };
        let mut findings = Vec::new();
        let mut observed = BTreeSet::new();
        for row in rows {
            let rule = row[if profile.3 { "advisory_id" } else { "rule_id" }]
                .as_str()
                .unwrap_or("");
            let Some(mapping) = pin
                .native_rules
                .iter()
                .find(|mapping| mapping.native_rule_id.as_str() == rule)
            else {
                issues.push(format!(
                    "native finding retained privately; approved rule mapping missing: {rule}"
                ));
                continue;
            };
            let path = if profile.3 {
                let Some(path) = pin.feedback_target.as_ref() else {
                    issues.push("dependency report has no lock path; explicit frozen feedback_target required".into());
                    continue;
                };
                match String::from_utf8(path.to_bytes()) {
                    Ok(path) => path,
                    Err(_) => {
                        issues.push("native dependency path encoding unsupported".into());
                        continue;
                    }
                }
            } else {
                let Some(path) = row["path"].as_str() else {
                    issues.push("native source finding path missing".into());
                    continue;
                };
                path.into()
            };
            let Some(target) = obligation
                .targets
                .iter()
                .find(|target| target.path.to_bytes() == path.as_bytes())
            else {
                issues.push(format!("native finding outside frozen targets: {path}"));
                continue;
            };
            let source = if profile.3 {
                native["lock_sha256"].as_str()
            } else {
                row["source_sha256"].as_str()
            };
            if source != target.input_digest.as_str().strip_prefix("sha256:") {
                issues.push(format!(
                    "native finding source digest differs from captured target: {path}"
                ));
                continue;
            }
            if !profile.3 && native["local_scan_complete"] != true {
                issues.push(
                    "native scan incomplete; already bound positive diagnostic retained".into(),
                );
            }
            let id = if let Some(id) = row["finding_id"].as_str().or_else(|| row["id"].as_str()) {
                id.to_owned()
            } else {
                let encoded = match serde_json::to_vec(row) {
                    Ok(encoded) => encoded,
                    Err(error) => {
                        issues.push(format!("native finding identity encoding failed: {error}"));
                        continue;
                    }
                };
                format!("CG-native-{:x}", Sha256::digest(encoded))
            };
            let severity = row["severity"]
                .as_str()
                .or_else(|| row["level"].as_str())
                .unwrap_or("UNKNOWN")
                .to_owned();
            let location = if profile.3 {
                let Some(component) = row["package_name"].as_str() else {
                    issues.push("native advisory component missing".into());
                    continue;
                };
                FindingLocation::Dependency {
                    component: component.into(),
                    version: row["package_version"].as_str().map(str::to_owned),
                    path: Some(path.clone()),
                }
            } else {
                FindingLocation::Source {
                    path: path.clone(),
                    line: row["line"].as_u64().and_then(|n| u32::try_from(n).ok()),
                    column: row["column"].as_u64().and_then(|n| u32::try_from(n).ok()),
                }
            };
            findings.push(Finding {
                id,
                native_rule_id: rule.into(),
                tool_id: pin.binding.tool_id.clone(),
                severity,
                gate_impact: mapping.gate_impact,
                message: row["message"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("原生诊断 {rule}")),
                obligation_id: pin.binding.obligation_id.clone(),
                native_identity: None,
                locations: vec![location],
            });
            observed.insert(path);
        }
        // 原生0.x只给局部观察，没有事前义务全集/完整目标覆盖证明，绝不转Complete。
        evidence.push(ObligationEvidence {
            result: ObligationResult {
                id: pin.binding.obligation_id.clone(),
                completion: Completion::Incomplete,
                reason: Some("native_feedback_frozen_coverage_not_proven".into()),
                findings,
            },
            observed_targets: observed.into_iter().collect(),
        });
        issues.push(format!(
            "native coverage remains partial/unknown: {}",
            pin.binding.checker_id
        ));
        if profile.3 {
            issues.push(
                "native advisory database freshness/provenance not independently verified".into(),
            );
        }
    }
    (evidence, issues, unbound)
}
