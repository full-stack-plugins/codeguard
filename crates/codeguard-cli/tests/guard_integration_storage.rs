#![cfg(target_os = "linux")]
#[path = "guard_integration_freshness.rs"]
mod fixture;
use codeguard_cli::guard_integration::audit::{AuditEvidence, AuditStore};
use codeguard_runtime::private_artifact_store::StoreLimits;
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cg-audit-domain-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn store_requires_explicit_publication_and_rechecks_every_actual_blob() {
    let root = Root::new();
    let store = AuditStore::open(
        &root.0,
        StoreLimits {
            max_entries: 8,
            max_total_bytes: 4 * 1024 * 1024,
        },
    )
    .unwrap();
    let (output, _) = fixture::sample(false);
    let raw = output.domain_bytes().to_vec();
    let staged = store.stage(AuditEvidence::Output(&output)).unwrap();
    let receipt = staged.receipt().clone();
    assert!(store.read(&receipt).is_err());
    drop(staged);
    assert!(!root.0.join(receipt.key()).exists());
    let receipt = store
        .stage(AuditEvidence::Output(&output))
        .unwrap()
        .publish()
        .unwrap();
    let loaded = store.read(&receipt).unwrap();
    assert_eq!(loaded.domain_bytes(), raw);
    assert_eq!(loaded.report_bytes(), output.report_bytes());
    assert!(
        store
            .stage(AuditEvidence::Output(&output))
            .unwrap()
            .publish()
            .is_err()
    );
    std::fs::write(root.0.join(receipt.key()).join("native.json"), b"tampered").unwrap();
    assert!(store.read(&receipt).is_err());
    assert_eq!(output.domain_bytes(), raw);
}

fn store(root: &Root) -> AuditStore {
    AuditStore::open(
        &root.0,
        StoreLimits {
            max_entries: 32,
            max_total_bytes: 8 * 1024 * 1024,
        },
    )
    .unwrap()
}
#[test]
fn missing_truncated_and_oversized_required_blobs_never_restore_consumable_evidence() {
    for leaf in [
        "manifest.json",
        "envelope.json",
        "native.json",
        "contract.json",
        "facts.json",
        "report.json",
    ] {
        for missing in [true, false] {
            let root = Root::new();
            let store = store(&root);
            let (output, _) = fixture::sample(false);
            let report = output.report_bytes().unwrap().to_vec();
            let receipt = store
                .stage(AuditEvidence::Output(&output))
                .unwrap()
                .publish()
                .unwrap();
            let path = root.0.join(receipt.key()).join(leaf);
            if missing {
                std::fs::remove_file(&path).unwrap();
            } else {
                let old = std::fs::read(&path).unwrap();
                std::fs::write(&path, &old[..old.len() - 1]).unwrap();
            }
            assert!(store.read(&receipt).is_err(), "{leaf} missing={missing}");
            assert_eq!(output.report_bytes().unwrap(), report);
        }
    }
    let root = Root::new();
    let store = store(&root);
    let (output, _) = fixture::sample(false);
    let receipt = store
        .stage(AuditEvidence::Output(&output))
        .unwrap()
        .publish()
        .unwrap();
    std::fs::write(
        root.0.join(receipt.key()).join("native.json"),
        vec![b'x'; 1_048_577],
    )
    .unwrap();
    assert!(matches!(
        store.read(&receipt),
        Err(
            codeguard_cli::guard_integration::audit::AuditError::Storage(
                codeguard_runtime::private_artifact_store::StoreError::Quota
            )
        )
    ));
}
#[test]
fn required_artifact_uris_are_never_paths_and_receipts_are_not_trusted_size_claims() {
    use codeguard_cli::guard_integration::audit::Receipt;
    use sha2::{Digest, Sha256};
    let root = Root::new();
    let store = store(&root);
    let (output, _) = fixture::sample(false);
    let receipt = store
        .stage(AuditEvidence::Output(&output))
        .unwrap()
        .publish()
        .unwrap();
    let mut value = serde_json::to_value(&receipt).unwrap();
    value["key"] = "../../user-file".into();
    assert!(Receipt::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    value["key"] = "artifact://run/native.json".into();
    assert!(Receipt::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let manifest_path = root.0.join(receipt.key()).join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest["files"]["native.json"]["size"] = serde_json::json!(u64::MAX);
    let bytes = serde_json::to_vec(&manifest).unwrap();
    std::fs::write(&manifest_path, &bytes).unwrap();
    let mut replacement = serde_json::to_value(&receipt).unwrap();
    replacement["manifest_digest"] = format!("sha256:{:x}", Sha256::digest(&bytes)).into();
    let replacement = Receipt::from_json(&serde_json::to_vec(&replacement).unwrap()).unwrap();
    assert!(store.read(&replacement).is_err());
}
#[test]
fn quota_permission_and_user_file_collision_keep_native_findings_in_memory() {
    let root = Root::new();
    let (output, _) = fixture::sample(true);
    let raw = output.domain_bytes().to_vec();
    let tiny = AuditStore::open(
        &root.0,
        StoreLimits {
            max_entries: 1,
            max_total_bytes: 8,
        },
    )
    .unwrap();
    assert!(tiny.stage(AuditEvidence::Output(&output)).is_err());
    assert_eq!(output.domain_bytes(), raw);
    let store = store(&root);
    let stage = store.stage(AuditEvidence::Output(&output)).unwrap();
    let receipt = stage.receipt().clone();
    drop(stage);
    let collision = root.0.join(receipt.key());
    std::fs::write(&collision, b"existing user report").unwrap();
    std::fs::set_permissions(&collision, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        store
            .stage(AuditEvidence::Output(&output))
            .unwrap()
            .publish()
            .is_err()
    );
    assert_eq!(std::fs::read(&collision).unwrap(), b"existing user report");
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o500)).unwrap();
    assert!(store.stage(AuditEvidence::Output(&output)).is_err());
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(output.domain_bytes(), raw);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&raw).unwrap()["files"][0]["findings"][0]["rule_id"],
        "F401"
    );
}
#[test]
fn persisted_approval_attachment_and_partial_output_preserve_semantics_without_issuing_authority() {
    use codeguard_cli::guard_integration::consumer::consume;
    use sha2::{Digest, Sha256};
    let root = Root::new();
    let store = store(&root);
    let (output, mut expected) = fixture::sample(true);
    let attached = output.with_approval_refs(&["approval:one".into()]).unwrap();
    let receipt = store
        .stage(AuditEvidence::Attached(&attached))
        .unwrap()
        .publish()
        .unwrap();
    let loaded = store.read(&receipt).unwrap();
    assert_eq!(loaded.envelope(), attached.envelope());
    assert_eq!(loaded.domain_bytes(), output.domain_bytes());
    assert_eq!(loaded.report_bytes(), output.report_bytes());
    assert_eq!(loaded.mapping_digest(), output.mapping_digest());
    expected.envelope_digest = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(loaded.envelope()).unwrap())
    );
    let result = consume(
        &loaded,
        &expected,
        &fixture::FixtureProvider::new(),
        fixture::NOW,
        None,
    )
    .unwrap();
    assert_eq!(
        result.eligibility.code,
        guardengine::integration::eligibility::EligibilityCode::InvalidApproval
    );
    assert_eq!(
        result.eligibility.technical_decision,
        Some(guardengine::Decision::RequireApproval)
    );
    let (partial, expected) = fixture::sample_native("missing-tool", false);
    let receipt = store
        .stage(AuditEvidence::Output(&partial))
        .unwrap()
        .publish()
        .unwrap();
    let loaded = store.read(&receipt).unwrap();
    assert_eq!(
        consume(
            &loaded,
            &expected,
            &fixture::FixtureProvider::new(),
            fixture::NOW,
            None
        )
        .unwrap()
        .eligibility
        .code,
        guardengine::integration::eligibility::EligibilityCode::Incomplete
    );
}

#[path = "support/guard_integration.rs"]
mod native_fixture;
#[test]
fn failed_native_bytes_are_auditable_but_never_restored_as_completed_engine_evidence() {
    use codeguard_cli::guard_integration::{
        consumer::consume, envelope::FrozenRun, profile::InvocationDescriptor, reader::read_native,
        scope::FrozenObligations,
    };
    let mut value = native_fixture::valid();
    value["exit_code"] = serde_json::json!(4);
    value["command_status"] = serde_json::json!("internal_error");
    let mut invocation = serde_json::to_value(native_fixture::invocation()).unwrap();
    invocation["process_exit"] = serde_json::json!(4);
    let invocation =
        InvocationDescriptor::parse(&serde_json::to_vec(&invocation).unwrap()).unwrap();
    let raw = serde_json::to_vec(&value).unwrap();
    let evidence = read_native(&raw, &invocation).unwrap();
    let (_, expected) = fixture::sample(false);
    let mut binding = expected.policy.binding.clone();
    binding.source_snapshot_digest = format!(
        "sha256:{}",
        value["identities"]["content"]["digest"].as_str().unwrap()
    );
    let scope = FrozenObligations::new(std::collections::BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    let output = FrozenRun::new(
        value["run_id"].as_str().unwrap(),
        binding,
        &scope,
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap()
    .failure(&evidence)
    .unwrap();
    let root = Root::new();
    let store = store(&root);
    let receipt = store
        .stage(AuditEvidence::Output(&output))
        .unwrap()
        .publish()
        .unwrap();
    let loaded = store.read(&receipt).unwrap();
    assert_eq!(loaded.domain_bytes(), raw);
    assert!(loaded.envelope().decision.is_none());
    assert!(loaded.report_bytes().is_none());
    assert_eq!(
        loaded.envelope().run_status,
        guardengine::integration::RunStatus::Error
    );
    assert!(
        consume(
            &loaded,
            &expected,
            &fixture::FixtureProvider::new(),
            fixture::NOW,
            None
        )
        .is_err()
    );
}
