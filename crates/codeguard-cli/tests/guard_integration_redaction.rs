#![cfg(target_os = "linux")]
#[path = "guard_integration_freshness.rs"]
mod fixture;
use codeguard_cli::guard_integration::audit::{
    AuditEvidence,
    access::{AuditAccess, RetentionPolicy},
};
use codeguard_runtime::private_artifact_store::StoreLimits;
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
// Real allocator observation is confined to the calling test thread.
struct CountingAllocator;
thread_local! {
    static TRACK_ALLOCATIONS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LARGEST_ALLOCATION: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        observe_allocation(layout.size());
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        observe_allocation(new_size);
        unsafe { std::alloc::GlobalAlloc::realloc(&std::alloc::System, ptr, layout, new_size) }
    }
}
fn observe_allocation(size: usize) {
    let _ = TRACK_ALLOCATIONS.try_with(|track| {
        if track.get() {
            let _ = LARGEST_ALLOCATION.try_with(|largest| largest.set(largest.get().max(size)));
        }
    });
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cg-redaction-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
    fn open(
        &self,
    ) -> (
        AuditAccess,
        codeguard_cli::guard_integration::audit::access::Administration,
    ) {
        AuditAccess::open(
            &self.0,
            StoreLimits {
                max_entries: 32,
                max_total_bytes: 8 * 1024 * 1024,
            },
            "tenant-A",
            RetentionPolicy {
                max_age_seconds: 60,
            },
        )
        .unwrap()
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn namespace_and_raw_permissions_are_separate_and_expiry_survives_reopen() {
    let root = Root::new();
    let other_root = Root::new();
    let (store, admin) = root.open();
    let (other, other_admin) = other_root.open();
    let view = store.view_access(&admin).unwrap();
    let raw = store.raw_access(&admin).unwrap();
    let foreign = other.raw_access(&other_admin).unwrap();
    let (output, _) = fixture::sample(false);
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    assert!(store.read_raw(&foreign, &receipt, 101).is_err());
    assert_eq!(
        store.read_raw(&raw, &receipt, 101).unwrap().domain_bytes(),
        output.domain_bytes()
    );
    let public = serde_json::to_value(store.view(&view, &receipt, 102).unwrap()).unwrap();
    let keys: Vec<_> = public
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["availability", "decision", "run_status", "version"]);
    assert!(store.read_raw(&raw, &receipt, 101).is_err()); // persisted clock cannot go backward
    assert!(store.read_raw(&raw, &receipt, 130).is_err()); // expiry boundary is exclusive
    let (reopened, reopened_admin) = root.open();
    assert!(reopened.raw_access(&admin).is_err());
    let new_raw = reopened.raw_access(&reopened_admin).unwrap();
    assert!(reopened.read_raw(&new_raw, &receipt, 129).is_err());
    assert!(reopened.read_raw(&new_raw, &receipt, 131).is_err());
}
#[test]
fn purge_disables_future_restoration_and_keeps_original_memory_unchanged() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let (output, _) = fixture::sample(false);
    let original = output.domain_bytes().to_vec();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    store.purge(&admin, &receipt, 110).unwrap();
    assert!(store.read_raw(&raw, &receipt, 111).is_err());
    assert_eq!(output.domain_bytes(), original);
}
#[test]
fn controlled_consumption_reloads_required_artifacts_on_every_call() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let (output, expected) = fixture::sample(false);
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), fixture::NOW, 30)
        .unwrap();
    let provider = fixture::FixtureProvider::new();
    assert!(
        store
            .consume(&raw, &receipt, &expected, &provider, fixture::NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    std::fs::remove_file(root.0.join(receipt.key()).join("report.json")).unwrap();
    assert!(
        store
            .consume(&raw, &receipt, &expected, &provider, fixture::NOW + 1, None)
            .is_err()
    );
}
#[test]
fn namespace_policy_and_missing_control_cannot_reset_persisted_time() {
    let root = Root::new();
    let (store, admin) = root.open();
    let (output, _) = fixture::sample(false);
    let _receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    assert!(
        AuditAccess::open(
            &root.0,
            StoreLimits {
                max_entries: 32,
                max_total_bytes: 8 * 1024 * 1024
            },
            "tenant-B",
            RetentionPolicy {
                max_age_seconds: 60
            }
        )
        .is_err()
    );
    assert!(
        AuditAccess::open(
            &root.0,
            StoreLimits {
                max_entries: 32,
                max_total_bytes: 8 * 1024 * 1024
            },
            "tenant-A",
            RetentionPolicy {
                max_age_seconds: 600
            }
        )
        .is_err()
    );
    std::fs::remove_file(root.0.join(".audit-retention-v1.json")).unwrap();
    assert!(
        AuditAccess::open(
            &root.0,
            StoreLimits {
                max_entries: 32,
                max_total_bytes: 8 * 1024 * 1024
            },
            "tenant-A",
            RetentionPolicy {
                max_age_seconds: 60
            }
        )
        .is_err()
    );
}
#[path = "support/guard_integration.rs"]
mod native_fixture;
#[test]
fn public_view_drops_token_environment_source_and_native_instructions() {
    use codeguard_cli::guard_integration::{
        envelope::FrozenRun, profile::InvocationDescriptor, reader::read_native,
        scope::FrozenObligations,
    };
    let root = Root::new();
    let (store, admin) = root.open();
    let sentinel = root.0.join("MUST_NOT_EXECUTE");
    let secret = format!(
        "TOKEN=secret-42 AWS_SECRET_ACCESS_KEY=environment-secret source: private_fn() native instruction: touch {}",
        sentinel.display()
    );
    let mut native = native_fixture::valid();
    native["results"][0]["findings"][0]["message"] = secret.clone().into();
    native["next_actions"] = serde_json::json!([secret]);
    native["exit_code"] = 4.into();
    native["command_status"] = "internal_error".into();
    let mut invoke = serde_json::to_value(native_fixture::invocation()).unwrap();
    invoke["process_exit"] = 4.into();
    let invoke = InvocationDescriptor::parse(&serde_json::to_vec(&invoke).unwrap()).unwrap();
    let bytes = serde_json::to_vec(&native).unwrap();
    let evidence = read_native(&bytes, &invoke).unwrap();
    let (_, expected) = fixture::sample(false);
    let mut binding = expected.policy.binding;
    binding.source_snapshot_digest = format!("sha256:{}", "a".repeat(64));
    let scope = FrozenObligations::new(std::collections::BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    let output = FrozenRun::new(
        "native-001",
        binding,
        &scope,
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap()
    .failure(&evidence)
    .unwrap();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    let public = serde_json::to_string(
        &store
            .view(&store.view_access(&admin).unwrap(), &receipt, 101)
            .unwrap(),
    )
    .unwrap();
    for text in [
        "TOKEN",
        "secret-42",
        "AWS",
        "environment-secret",
        "private_fn",
        "touch",
        "MUST_NOT_EXECUTE",
    ] {
        assert!(!public.contains(text), "{text}");
    }
    assert!(!sentinel.exists());
    assert_eq!(
        store
            .read_raw(&store.raw_access(&admin).unwrap(), &receipt, 102)
            .unwrap()
            .domain_bytes(),
        bytes
    );
}
#[test]
fn loss_of_checkpoint_after_last_purge_does_not_reinitialize_namespace() {
    let root = Root::new();
    let (store, admin) = root.open();
    let (output, _) = fixture::sample(false);
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    store.purge(&admin, &receipt, 120).unwrap();
    std::fs::remove_file(root.0.join(".audit-retention-v1.json")).unwrap();
    assert!(
        AuditAccess::open(
            &root.0,
            StoreLimits {
                max_entries: 32,
                max_total_bytes: 8 * 1024 * 1024
            },
            "tenant-A",
            RetentionPolicy {
                max_age_seconds: 60
            }
        )
        .is_err()
    );
}
#[test]
fn persisted_clock_failure_does_not_return_evidence_or_overwrite_checkpoint() {
    let root = Root::new();
    let (store, admin) = root.open();
    let (output, _) = fixture::sample(false);
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    let raw = store.raw_access(&admin).unwrap();
    let control = root.0.join(".audit-retention-v1.json");
    let original = std::fs::read(&control).unwrap();
    std::fs::set_permissions(&control, std::fs::Permissions::from_mode(0o400)).unwrap();
    assert!(store.read_raw(&raw, &receipt, 110).is_err());
    assert_eq!(std::fs::read(&control).unwrap(), original);
    std::fs::set_permissions(&control, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(store.read_raw(&raw, &receipt, 110).is_ok());
    assert!(store.read_raw(&raw, &receipt, 109).is_err());
}
#[test]
fn whole_root_lease_blocks_purge_while_fresh_provider_is_consulted() {
    use guardengine::integration::{GuardRunEnvelope, eligibility::*};
    use std::cell::Cell;
    let root = Root::new();
    let (store, admin) = root.open();
    let (output, expected) = fixture::sample(false);
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), fixture::NOW, 30)
        .unwrap();
    struct Provider<'a> {
        store: &'a AuditAccess,
        admin: &'a codeguard_cli::guard_integration::audit::access::Administration,
        receipt: &'a codeguard_cli::guard_integration::audit::Receipt,
        inner: fixture::FixtureProvider,
        attempted: Cell<bool>,
    }
    impl AuthorityProvider for Provider<'_> {
        fn verify_producer(
            &self,
            e: &GuardRunEnvelope,
            d: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            self.attempted.set(true);
            assert!(matches!(
                self.store.purge(self.admin, self.receipt, fixture::NOW),
                Err(
                    codeguard_cli::guard_integration::audit::access::AccessError::Evidence(
                        codeguard_cli::guard_integration::audit::AuditError::Storage(
                            codeguard_runtime::private_artifact_store::StoreError::Busy
                        )
                    )
                )
            ));
            self.inner.verify_producer(e, d)
        }
        fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
            self.inner.verify_approval(r)
        }
    }
    let provider = Provider {
        store: &store,
        admin: &admin,
        receipt: &receipt,
        inner: fixture::FixtureProvider::new(),
        attempted: Cell::new(false),
    };
    let raw = store.raw_access(&admin).unwrap();
    assert!(
        store
            .consume(&raw, &receipt, &expected, &provider, fixture::NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    assert!(provider.attempted.get());
    store.purge(&admin, &receipt, fixture::NOW + 1).unwrap();
    assert!(
        store
            .consume(&raw, &receipt, &expected, &provider, fixture::NOW + 2, None)
            .is_err()
    );
}
#[test]
fn forged_receipt_and_oversized_checkpoint_fail_closed() {
    let root = Root::new();
    let (store, admin) = root.open();
    let (output, _) = fixture::sample(false);
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    let raw = store.raw_access(&admin).unwrap();
    let mut forged = serde_json::to_value(&receipt).unwrap();
    forged["manifest_digest"] = "secret".repeat(350_000).into();
    let forged = serde_json::from_value(forged).unwrap();
    LARGEST_ALLOCATION.with(|n| n.set(0));
    TRACK_ALLOCATIONS.with(|enabled| enabled.set(true));
    let rejected = store.read_raw(&raw, &forged, 101).is_err();
    TRACK_ALLOCATIONS.with(|enabled| enabled.set(false));
    assert!(rejected);
    assert!(
        LARGEST_ALLOCATION.with(|n| n.get()) < 1024,
        "forged field was copied before validation"
    );
    let control = root.0.join(".audit-retention-v1.json");
    std::fs::OpenOptions::new()
        .write(true)
        .open(control)
        .unwrap()
        .set_len(128 * 1024 + 1)
        .unwrap();
    assert!(store.read_raw(&raw, &receipt, 101).is_err());
}
#[test]
fn failed_physical_purge_leaves_tombstone_that_rejects_restored_old_bytes() {
    let root = Root::new();
    let (store, admin) = root.open();
    let (output, _) = fixture::sample(false);
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), 100, 30)
        .unwrap();
    let directory = root.0.join(receipt.key());
    let saved: Vec<_> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        })
        .collect();
    // Missing bundle forces physical deletion failure after the durable tombstone step.
    std::fs::remove_dir_all(&directory).unwrap();
    assert!(store.purge(&admin, &receipt, 110).is_err());
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    for (name, bytes) in saved {
        let path = directory.join(name);
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let (reopened, admin) = root.open();
    let raw = reopened.raw_access(&admin).unwrap();
    assert!(matches!(
        reopened.read_raw(&raw, &receipt, 111),
        Err(codeguard_cli::guard_integration::audit::access::AccessError::Purged)
    ));
    let public = serde_json::to_value(
        reopened
            .view(&reopened.view_access(&admin).unwrap(), &receipt, 111)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(public["availability"], "unavailable");
    assert!(public["decision"].is_null());
}
#[test]
fn retained_consumption_requeries_revocation_without_changing_technical_report() {
    use guardengine::integration::{GuardRunEnvelope, eligibility::*};
    use std::cell::Cell;
    struct Provider {
        calls: Cell<usize>,
        revoked: Cell<bool>,
        inner: fixture::FixtureProvider,
    }
    impl AuthorityProvider for Provider {
        fn verify_producer(
            &self,
            e: &GuardRunEnvelope,
            d: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            self.calls.set(self.calls.get() + 1);
            let mut record = self.inner.verify_producer(e, d)?;
            record.validity.revoked = self.revoked.get();
            Ok(record)
        }
        fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
            self.inner.verify_approval(r)
        }
    }
    let root = Root::new();
    let (store, admin) = root.open();
    let (output, expected) = fixture::sample(false);
    let report = output.report_bytes().unwrap().to_vec();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), fixture::NOW, 30)
        .unwrap();
    let raw = store.raw_access(&admin).unwrap();
    let provider = Provider {
        calls: Cell::new(0),
        revoked: Cell::new(false),
        inner: fixture::FixtureProvider::new(),
    };
    let first = store
        .consume(&raw, &receipt, &expected, &provider, fixture::NOW, None)
        .unwrap();
    assert!(first.eligibility.eligible);
    provider.revoked.set(true);
    let second = store
        .consume(&raw, &receipt, &expected, &provider, fixture::NOW + 1, None)
        .unwrap();
    assert!(!second.eligibility.eligible);
    assert_eq!(
        first.eligibility.technical_decision,
        second.eligibility.technical_decision
    );
    assert_eq!(provider.calls.get(), 2);
    assert_eq!(
        store
            .read_raw(&raw, &receipt, fixture::NOW + 1)
            .unwrap()
            .report_bytes()
            .unwrap(),
        report
    );
    assert!(
        store
            .consume(
                &raw,
                &receipt,
                &expected,
                &provider,
                fixture::NOW + 30,
                None
            )
            .is_err()
    );
    assert_eq!(provider.calls.get(), 2);
    assert_eq!(output.report_bytes().unwrap(), report);
}
