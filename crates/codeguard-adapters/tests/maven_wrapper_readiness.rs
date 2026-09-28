use codeguard_adapters::inspect_maven_wrapper_candidate;
use codeguard_core::ObservedPathKind;

const PROPERTIES: &[u8] = b"distributionUrl=https\\://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.9/apache-maven-3.9.9-bin.zip\n";

#[test]
fn local_wrapper_and_version_are_candidates_without_path_lookup() {
    let candidate = inspect_maven_wrapper_candidate(Some(ObservedPathKind::File), Some(PROPERTIES));
    assert_eq!(candidate.state, "wrapper_candidate_requires_native_probe");
    assert_eq!(candidate.observed_version.as_deref(), Some("3.9.9"));
    assert!(!candidate.next_action.contains("安装"));
}

#[test]
fn invalid_or_unsafe_wrapper_is_not_a_missing_maven_instruction() {
    let duplicate = b"distributionUrl=https\\://example.test/apache-maven-3.9.9-bin.zip\ndistributionUrl=https\\://example.test/apache-maven-3.9.9-bin.zip\n";
    let invalid = inspect_maven_wrapper_candidate(Some(ObservedPathKind::File), Some(duplicate));
    assert_eq!(invalid.state, "wrapper_configuration_invalid");
    assert!(!invalid.next_action.contains("安装"));

    let unsafe_script =
        inspect_maven_wrapper_candidate(Some(ObservedPathKind::Symlink), Some(PROPERTIES));
    assert_eq!(unsafe_script.state, "wrapper_script_untrusted");
}

#[test]
fn partial_wrapper_evidence_stays_unknown_and_maven_four_needs_adapter() {
    assert_eq!(
        inspect_maven_wrapper_candidate(None, Some(PROPERTIES)).state,
        "wrapper_script_unobserved"
    );
    assert_eq!(
        inspect_maven_wrapper_candidate(Some(ObservedPathKind::File), None).state,
        "wrapper_configuration_unobserved"
    );
    assert_eq!(inspect_maven_wrapper_candidate(None, None).state, "unknown");
    let next = b"distributionUrl=https\\://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/4.0.0/apache-maven-4.0.0-bin.zip\n";
    assert_eq!(
        inspect_maven_wrapper_candidate(Some(ObservedPathKind::File), Some(next)).state,
        "wrapper_version_not_supported_by_adapter"
    );
}
