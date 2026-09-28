//! 两份同轮原生局部观察的精确坐标与制品摘要候选归属；不判定漏洞库时效。

use codeguard_adapters::{
    MavenDependencyNode, MavenDependencyTree, OwaspAdvisoryObservation, OwaspArtifactBindingState,
    OwaspAttributionState, OwaspDependencyCheckReport, attribute_owasp_advisories_to_maven_graph,
    bind_owasp_advisories_to_artifact_digests,
};
use serde_json::{Value, json};

/// 对每个 CVE 构建根寻找相同 POM、工具、JDK 和离线仓库身份的原生依赖图。
pub(crate) fn attach_candidates(cve: &mut Value, dependencies: &Value) {
    let Some(cve_probes) = cve["probes"].as_array() else {
        return;
    };
    let dependency_probes = dependencies["probes"].as_array();
    let candidates: Vec<_> = cve_probes
        .iter()
        .map(|probe| {
            let matching: Vec<_> = dependency_probes
                .into_iter()
                .flatten()
                .filter(|entry| {
                    entry["build_root"] == probe["build_root"]
                        && entry["configuration_ref"] == probe["configuration_ref"]
                })
                .collect();
            let (status, bindings) = match matching.as_slice() {
                [] => ("dependency_graph_unavailable", Vec::new()),
                [entry] => evaluate(&probe["observation"], &entry["observation"]),
                _ => ("dependency_graph_ambiguous", Vec::new()),
            };
            json!({"build_root":probe["build_root"],"status":status,"bindings":bindings})
        })
        .collect();
    cve["attribution_probes"] = json!(candidates);
}

fn evaluate(cve: &Value, dependency: &Value) -> (&'static str, Vec<Value>) {
    if cve["native_status"] != "findings_observed_untrusted"
        || dependency["native_status"] != "graph_observed_untrusted"
    {
        return ("native_reports_unavailable", Vec::new());
    }
    for key in [
        "native_plan_sha256",
        "maven_tool_sha256",
        "java_runtime_sha256",
        "dependency_closure_sha256",
    ] {
        let (Some(left), Some(right)) = (cve[key].as_str(), dependency[key].as_str()) else {
            return ("native_identity_unavailable", Vec::new());
        };
        if left.len() != 64 || !left.bytes().all(|byte| byte.is_ascii_hexdigit()) || left != right {
            return ("native_identity_mismatch", Vec::new());
        }
    }
    let Some(advisories) = cve["advisories"].as_array() else {
        return ("native_report_invalid", Vec::new());
    };
    let Some(nodes) = dependency["nodes"].as_array() else {
        return ("native_report_invalid", Vec::new());
    };
    let Some(graph_nodes) = nodes.iter().map(project_node).collect::<Option<Vec<_>>>() else {
        return ("native_report_invalid", Vec::new());
    };
    let Some(observations) = advisories
        .iter()
        .map(project_advisory)
        .collect::<Option<Vec<_>>>()
    else {
        return ("native_report_invalid", Vec::new());
    };
    let report = OwaspDependencyCheckReport {
        engine_version: String::new(),
        project_name: String::new(),
        report_date: String::new(),
        data_sources: Vec::new(),
        dependency_count: 0,
        advisories: observations,
    };
    let graph = MavenDependencyTree {
        nodes: graph_nodes,
        edges: Vec::new(),
    };
    let digests = nodes
        .iter()
        .map(|node| node["artifact_sha256"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    let positions = attribute_owasp_advisories_to_maven_graph(&report, &graph);
    let artifact_states = bind_owasp_advisories_to_artifact_digests(&report, &graph, &digests);
    let bindings = positions
        .iter()
        .zip(artifact_states)
        .map(|(position, artifact)| {
            json!({
                "observation_index":position.observation_index,
                "coordinate_state":coordinate_state(position.state),
                "graph_node_index":position.graph_node_index,
                "artifact_state":artifact_state(artifact)
            })
        })
        .collect();
    ("candidate_evaluated_untrusted", bindings)
}

fn project_node(value: &Value) -> Option<MavenDependencyNode> {
    Some(MavenDependencyNode {
        group_id: value["group_id"].as_str()?.to_owned(),
        artifact_id: value["artifact_id"].as_str()?.to_owned(),
        version: value["version"].as_str()?.to_owned(),
        artifact_type: value["type"].as_str()?.to_owned(),
        scope: value["scope"].as_str()?.to_owned(),
        classifier: value["classifier"].as_str()?.to_owned(),
        optional: value["optional"].as_bool()?,
    })
}

fn project_advisory(value: &Value) -> Option<OwaspAdvisoryObservation> {
    Some(OwaspAdvisoryObservation {
        source: value["source"].as_str()?.to_owned(),
        advisory_id: value["advisory_id"].as_str()?.to_owned(),
        score: value["score"].as_f64(),
        package_ids: value["package_ids"]
            .as_array()?
            .iter()
            .map(|item| {
                item.as_str().map(|id| {
                    if id.starts_with("redacted-sha256:") {
                        // 脱敏前可能是冲突 Maven PURL；给原归属器一个必拒绝的占位标记。
                        "pkg:maven/%redacted".to_owned()
                    } else {
                        id.to_owned()
                    }
                })
            })
            .collect::<Option<Vec<_>>>()?,
        dependency_sha256: value["dependency_sha256"].as_str().map(str::to_owned),
        suppressed_by_native_tool: value["suppressed_by_native_tool"].as_bool()?,
    })
}

fn coordinate_state(value: OwaspAttributionState) -> &'static str {
    match value {
        OwaspAttributionState::ExactCoordinateCandidate => "exact_coordinate_candidate",
        OwaspAttributionState::MissingMavenPurl => "missing_maven_purl",
        OwaspAttributionState::UnsupportedPurl => "unsupported_purl",
        OwaspAttributionState::ConflictingPurls => "conflicting_purls",
        OwaspAttributionState::NotInGraph => "not_in_graph",
        OwaspAttributionState::AmbiguousGraphCoordinate => "ambiguous_graph_coordinate",
    }
}

fn artifact_state(value: OwaspArtifactBindingState) -> &'static str {
    match value {
        OwaspArtifactBindingState::AttributionUnavailable => "attribution_unavailable",
        OwaspArtifactBindingState::OwaspDigestMissing => "owasp_digest_missing",
        OwaspArtifactBindingState::MavenDigestMissing => "maven_digest_missing",
        OwaspArtifactBindingState::DigestEvidenceInvalid => "digest_evidence_invalid",
        OwaspArtifactBindingState::DigestMismatch => "digest_mismatch",
        OwaspArtifactBindingState::ExactDigestCandidate => "exact_digest_candidate",
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use serde_json::json;

    #[test]
    fn a_different_pom_or_incomplete_graph_cannot_reuse_an_advisory() {
        let digest_a = "a".repeat(64);
        let digest_b = "b".repeat(64);
        let cve = json!({
            "native_status":"findings_observed_untrusted",
            "native_plan_sha256":digest_a,
            "maven_tool_sha256":digest_a,
            "java_runtime_sha256":digest_a,
            "dependency_closure_sha256":digest_a,
            "advisories":[{"source":"NVD","advisory_id":"CVE-2026-1234","score":8.0,"package_ids":["pkg:maven/org.demo/lib@2.0"],"dependency_sha256":digest_a,"suppressed_by_native_tool":false}]
        });
        let mut graph = json!({
            "native_status":"graph_observed_untrusted",
            "native_plan_sha256":digest_b,
            "maven_tool_sha256":digest_a,
            "java_runtime_sha256":digest_a,
            "dependency_closure_sha256":digest_a,
            "nodes":[]
        });
        assert_eq!(evaluate(&cve, &graph).0, "native_identity_mismatch");
        graph["native_plan_sha256"] = json!(digest_a);
        graph["native_status"] = json!("incomplete");
        assert_eq!(evaluate(&cve, &graph).0, "native_reports_unavailable");
    }
}
