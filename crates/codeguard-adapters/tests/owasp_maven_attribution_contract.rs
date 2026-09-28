use codeguard_adapters::{
    OwaspArtifactBindingState, OwaspAttributionState, attribute_owasp_advisories_to_maven_graph,
    bind_owasp_advisories_to_artifact_digests, parse_maven_dependency_tree_json,
    parse_owasp_dependency_check_json,
};

fn graph() -> codeguard_adapters::MavenDependencyTree {
    parse_maven_dependency_tree_json(br#"{"groupId":"demo","artifactId":"app","version":"1","type":"jar","scope":"","classifier":"","optional":"false","children":[{"groupId":"org.demo","artifactId":"lib","version":"2.0","type":"jar","scope":"compile","classifier":"","optional":"false"}]}"#).unwrap()
}

#[test]
fn same_coordinate_cannot_certify_different_or_missing_artifact_bytes() {
    let mut parsed = report(&["pkg:maven/org.demo/lib@2.0"], false);
    let graph = graph();
    let good = "a".repeat(64);
    let bad = "b".repeat(64);
    parsed.advisories[0].dependency_sha256 = Some(good.clone());
    assert_eq!(
        bind_owasp_advisories_to_artifact_digests(&parsed, &graph, &[None, Some(good)])[0],
        OwaspArtifactBindingState::ExactDigestCandidate
    );
    assert_eq!(
        bind_owasp_advisories_to_artifact_digests(&parsed, &graph, &[None, Some(bad)])[0],
        OwaspArtifactBindingState::DigestMismatch
    );
    assert_eq!(
        bind_owasp_advisories_to_artifact_digests(&parsed, &graph, &[None, None])[0],
        OwaspArtifactBindingState::MavenDigestMissing
    );
    assert_eq!(
        bind_owasp_advisories_to_artifact_digests(&parsed, &graph, &[None, Some("invalid".into())])
            [0],
        OwaspArtifactBindingState::DigestEvidenceInvalid
    );
    let wrong_report = report(&["pkg:maven/org.demo/lib@1.9"], false);
    assert_eq!(
        bind_owasp_advisories_to_artifact_digests(
            &wrong_report,
            &graph,
            &[None, Some("a".repeat(64))]
        )[0],
        OwaspArtifactBindingState::AttributionUnavailable
    );
    parsed.advisories[0].dependency_sha256 = None;
    assert_eq!(
        bind_owasp_advisories_to_artifact_digests(&parsed, &graph, &[None, Some("a".repeat(64))])
            [0],
        OwaspArtifactBindingState::OwaspDigestMissing
    );
    assert_eq!(
        bind_owasp_advisories_to_artifact_digests(&parsed, &graph, &[])[0],
        OwaspArtifactBindingState::OwaspDigestMissing
    );
}

fn report(
    package_ids: &[&str],
    suppressed: bool,
) -> codeguard_adapters::OwaspDependencyCheckReport {
    let packages: Vec<_> = package_ids
        .iter()
        .map(|id| serde_json::json!({"id":id}))
        .collect();
    let key = if suppressed {
        "suppressedVulnerabilities"
    } else {
        "vulnerabilities"
    };
    let raw = serde_json::json!({
        "reportSchema":"1.1", "scanInfo":{"engineVersion":"12.1.0","dataSource":[]},
        "projectInfo":{"name":"demo","reportDate":"2026-09-26T00:00:00Z"},
        "dependencies":[{"isVirtual":false,"fileName":"lib.jar","packages":packages,
            key:[{"source":"NVD","name":"CVE-2026-1234","cvssv3":{"baseScore":8.0}}]}]
    });
    parse_owasp_dependency_check_json(&serde_json::to_vec(&raw).unwrap()).unwrap()
}

#[test]
fn only_exact_maven_coordinate_is_a_candidate_and_suppression_remains_visible() {
    let parsed = report(&["pkg:maven/org.demo/lib@2.0"], false);
    let attributions = attribute_owasp_advisories_to_maven_graph(&parsed, &graph());
    assert_eq!(attributions.len(), 1);
    assert_eq!(
        attributions[0].state,
        OwaspAttributionState::ExactCoordinateCandidate
    );
    assert_eq!(attributions[0].graph_node_index, Some(1));
    let suppressed = report(&["pkg:maven/org.demo/lib@2.0"], true);
    let attributions = attribute_owasp_advisories_to_maven_graph(&suppressed, &graph());
    assert_eq!(
        attributions[0].state,
        OwaspAttributionState::ExactCoordinateCandidate
    );
    assert!(suppressed.advisories[0].suppressed_by_native_tool);
}

#[test]
fn old_version_other_group_missing_or_broad_identity_cannot_match() {
    for (ids, expected) in [
        (
            vec!["pkg:maven/org.demo/lib@1.9"],
            OwaspAttributionState::NotInGraph,
        ),
        (
            vec!["pkg:maven/other/lib@2.0"],
            OwaspAttributionState::NotInGraph,
        ),
        (vec![], OwaspAttributionState::MissingMavenPurl),
        (
            vec!["pkg:maven/org.demo/lib"],
            OwaspAttributionState::UnsupportedPurl,
        ),
        (
            vec!["pkg:maven/org.demo/lib@2.0?repository_url=https:%2F%2Fprivate.example"],
            OwaspAttributionState::UnsupportedPurl,
        ),
        (
            vec!["pkg:maven/org.demo/lib@2.0", "pkg:maven/org.demo/lib@1.9"],
            OwaspAttributionState::ConflictingPurls,
        ),
    ] {
        let attributions =
            attribute_owasp_advisories_to_maven_graph(&report(&ids, false), &graph());
        assert_eq!(attributions[0].state, expected, "{ids:?}");
        assert_eq!(attributions[0].graph_node_index, None);
    }
}

#[test]
fn artifact_type_classifier_and_duplicate_graph_nodes_prevent_false_attribution() {
    for id in [
        "pkg:maven/org.demo/lib@2.0?type=pom",
        "pkg:maven/org.demo/lib@2.0?classifier=sources",
    ] {
        let attributions =
            attribute_owasp_advisories_to_maven_graph(&report(&[id], false), &graph());
        assert_eq!(attributions[0].state, OwaspAttributionState::NotInGraph);
    }
    let duplicate = parse_maven_dependency_tree_json(br#"{"groupId":"demo","artifactId":"app","version":"1","type":"jar","scope":"","classifier":"","optional":"false","children":[{"groupId":"org.demo","artifactId":"lib","version":"2.0","type":"jar","scope":"compile","classifier":"","optional":"false"},{"groupId":"org.demo","artifactId":"lib","version":"2.0","type":"jar","scope":"test","classifier":"","optional":"false"}]}"#).unwrap();
    let attributions = attribute_owasp_advisories_to_maven_graph(
        &report(&["pkg:maven/org.demo/lib@2.0"], false),
        &duplicate,
    );
    assert_eq!(
        attributions[0].state,
        OwaspAttributionState::AmbiguousGraphCoordinate
    );
    assert_eq!(attributions[0].graph_node_index, None);
}
