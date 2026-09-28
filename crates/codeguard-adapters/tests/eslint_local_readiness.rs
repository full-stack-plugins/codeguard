use codeguard_adapters::{EslintConfigState, inspect_eslint_local_candidate};
use codeguard_core::ObservedPathKind;

const ROOT: &[u8] = br#"{"name":"demo","devDependencies":{"eslint":"^10.0.0"}}"#;
const LOCAL: &[u8] = br#"{"name":"eslint","version":"10.1.0","bin":{"eslint":"bin/eslint.js"}}"#;

#[test]
fn project_local_eslint_is_a_candidate_even_without_path_lookup() {
    let candidate = inspect_eslint_local_candidate(
        Some(ROOT),
        Some(LOCAL),
        Some(ObservedPathKind::File),
        EslintConfigState::Unknown,
    );
    assert_eq!(candidate.state, "local_candidate_requires_native_probe");
    assert_eq!(candidate.declared_spec.as_deref(), Some("^10.0.0"));
    assert_eq!(candidate.observed_version.as_deref(), Some("10.1.0"));
    assert!(!candidate.next_action.contains("安装"));
}

#[test]
fn broken_config_or_package_is_not_a_missing_tool_instruction() {
    let broken_config = inspect_eslint_local_candidate(
        Some(ROOT),
        Some(LOCAL),
        Some(ObservedPathKind::File),
        EslintConfigState::Invalid,
    );
    assert_eq!(broken_config.state, "configuration_invalid");
    assert!(broken_config.next_action.contains("配置"));
    assert!(!broken_config.next_action.contains("安装"));

    let malformed = inspect_eslint_local_candidate(
        Some(br#"{"devDependencies":{"eslint":"10","eslint":"9"}}"#),
        Some(LOCAL),
        Some(ObservedPathKind::File),
        EslintConfigState::Unknown,
    );
    assert_eq!(malformed.state, "project_manifest_invalid");
    assert!(!malformed.next_action.contains("安装"));

    let unsafe_entry = inspect_eslint_local_candidate(
        Some(ROOT),
        Some(LOCAL),
        Some(ObservedPathKind::Symlink),
        EslintConfigState::Unknown,
    );
    assert_eq!(unsafe_entry.state, "local_entry_untrusted");
}

#[test]
fn declarations_versions_and_missing_local_files_remain_distinct() {
    let declared =
        inspect_eslint_local_candidate(Some(ROOT), None, None, EslintConfigState::Unknown);
    assert_eq!(declared.state, "declared_local_entry_unobserved");

    let unsupported = inspect_eslint_local_candidate(
        Some(ROOT),
        Some(br#"{"name":"eslint","version":"9.1.0","bin":{"eslint":"bin/eslint.js"}}"#),
        Some(ObservedPathKind::File),
        EslintConfigState::Unknown,
    );
    assert_eq!(unsupported.state, "local_version_not_supported_by_adapter");
    assert!(!unsupported.next_action.contains("安装"));

    let unclaimed = inspect_eslint_local_candidate(None, None, None, EslintConfigState::Unknown);
    assert_eq!(unclaimed.state, "unknown");

    let conflict = inspect_eslint_local_candidate(
        Some(br#"{"devDependencies":{"eslint":"9.1.0"}}"#),
        Some(LOCAL),
        Some(ObservedPathKind::File),
        EslintConfigState::Observed,
    );
    assert_eq!(conflict.state, "local_version_conflicts_with_declaration");
    assert!(!conflict.next_action.contains("安装"));

    let damaged_local = inspect_eslint_local_candidate(
        Some(ROOT),
        Some(br#"{"name":"other","version":"10.1.0","bin":{"eslint":"../escape.js"}}"#),
        Some(ObservedPathKind::File),
        EslintConfigState::Unknown,
    );
    assert_eq!(damaged_local.state, "local_package_identity_invalid");
}
