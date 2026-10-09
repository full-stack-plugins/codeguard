#![cfg(target_os = "linux")]
#[path = "guard_integration_freshness.rs"]
mod fixture;
use codeguard_cli::guard_integration::{
    audit::{
        AuditEvidence,
        access::{Administration, AuditAccess, RetentionPolicy},
        attempts::{AttemptHistory, AttemptWork, ImportOutcome},
    },
    consumer::ExpectedConsumption,
    envelope::{EnvelopeOutput, FrozenRun},
    projection::ProtectedMapping,
    ruff_profile::{RuffF401Policy, project_ruff, read_ruff_feedback},
};
use codeguard_runtime::private_artifact_store::StoreLimits;
use guardengine::{GuardContract, GuardSubject, integration::RunBinding};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
struct Observer;
thread_local! { static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; static MAX_ALLOC: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
fn observe(size: usize) {
    let _ = TRACK.try_with(|on| {
        if on.get() {
            let _ = MAX_ALLOC.try_with(|n| n.set(n.get().max(size)));
        }
    });
}
unsafe impl std::alloc::GlobalAlloc for Observer {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        observe(l.size());
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        observe(n);
        unsafe { std::alloc::GlobalAlloc::realloc(&std::alloc::System, p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Observer = Observer;
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cg-attempts-{}-{}",
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
    fn open(&self) -> (AuditAccess, Administration) {
        AuditAccess::open(
            &self.0,
            StoreLimits {
                max_entries: 64,
                max_total_bytes: 8 * 1024 * 1024,
            },
            "fixture-namespace",
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
fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../tests/fixtures/guard-integration/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
// Explicit fixture invocation labels and binding identities, not a new actual Git/native capture.
fn output(
    case: &str,
    run: &str,
    task: &str,
    worktree: &str,
    requirement: &str,
    head: char,
    whitespace: bool,
) -> (EnvelopeOutput, ExpectedConsumption) {
    let capture: serde_json::Value =
        serde_json::from_slice(&read_fixture("ruff-f401/capture.json")).unwrap();
    let producer = capture["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        capture["cases"][case]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let mut native: serde_json::Value =
        serde_json::from_slice(&read_fixture(&format!("ruff-f401/{case}.json"))).unwrap();
    native["run_id"] = run.into();
    let mut raw = serde_json::to_vec(&native).unwrap();
    if whitespace {
        raw.push(b'\n');
    }
    let evidence = read_ruff_feedback(&raw, &policy, run, 3, producer).unwrap();
    let vocab: serde_json::Value = serde_json::from_slice(&read_fixture("relations.json")).unwrap();
    let mapping = ProtectedMapping::parse(&serde_json::to_vec(&vocab["mapping"]).unwrap()).unwrap();
    let contract: GuardContract = serde_json::from_value(vocab["contract"].clone()).unwrap();
    let binding = RunBinding {
        repo_id: "repo".into(),
        task_id: task.into(),
        worktree_id: worktree.into(),
        requirement_ids: vec![requirement.into()],
        candidate_oid: head.to_string().repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: policy.source_digest(),
        baseline_digest: None,
    };
    let projection = project_ruff(
        &evidence,
        &mapping,
        &contract,
        GuardSubject {
            id: "repo".into(),
            snapshot_digest: policy.source_digest(),
        },
    )
    .unwrap();
    let result = FrozenRun::new(
        run,
        binding,
        &policy.obligations(),
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap()
    .complete(projection)
    .unwrap();
    let envelope = result.envelope();
    let expected = ExpectedConsumption {
        policy: guardengine::integration::eligibility::EligibilityPolicy {
            binding: envelope.binding.clone(),
            producer: envelope.producer.clone(),
            required_scopes: envelope.coverage.required_scopes.clone(),
            contract_digest: envelope.artifacts.contract.as_ref().unwrap().digest.clone(),
            action: "merge".into(),
            producer_principals: BTreeSet::from(["fixture-producer".into()]),
            approval_principals: BTreeMap::new(),
        },
        run_id: run.into(),
        mapping_digest: mapping.digest(),
        envelope_digest: digest(&serde_json::to_vec(envelope).unwrap()),
        raw_domain_digest: digest(&raw),
    };
    (result, expected)
}
fn work(expected: &ExpectedConsumption) -> AttemptWork {
    AttemptWork::freeze(
        &expected.policy.binding,
        &expected.policy.producer,
        &expected.policy.required_scopes,
        &expected.policy.contract_digest,
        &expected.mapping_digest,
    )
    .unwrap()
}
#[test]
fn late_head1_success_cannot_overwrite_current_head2_block() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let (old, old_expected) = output("clean", "old", "task", "worktree", "R1", 'a', false);
    let (new, new_expected) = output("bad", "new", "task", "worktree", "R1", 'c', false);
    let old_work = work(&old_expected);
    let target = old_work.target().clone();
    let old_ticket = history.register(&admin, "old", old_work, 0).unwrap();
    let new_ticket = history
        .register(&admin, "new", work(&new_expected), 1)
        .unwrap();
    let new_receipt = store
        .publish(&admin, AuditEvidence::Output(&new), fixture::NOW, 30)
        .unwrap();
    assert_eq!(
        history
            .import(&admin, &raw, &new_ticket, &new_receipt, fixture::NOW)
            .unwrap(),
        ImportOutcome::Inserted
    );
    history
        .publish(&admin, &raw, &new_ticket, fixture::NOW)
        .unwrap();
    let old_receipt = store
        .publish(&admin, AuditEvidence::Output(&old), fixture::NOW, 30)
        .unwrap();
    history
        .import(&admin, &raw, &old_ticket, &old_receipt, fixture::NOW)
        .unwrap();
    assert!(
        history
            .publish(&admin, &raw, &old_ticket, fixture::NOW)
            .is_err()
    );
    assert_eq!(
        history.current(&admin, &target).unwrap().unwrap().run_id,
        "new"
    );
    assert_eq!(history.history(&admin, &target).unwrap().len(), 2);
    let result = history
        .consume_current(
            &raw,
            &new_expected,
            &fixture::FixtureProvider::new(),
            fixture::NOW,
            None,
        )
        .unwrap();
    assert_eq!(
        result.eligibility.technical_decision,
        Some(guardengine::Decision::Block)
    );
    assert!(!result.eligibility.eligible);
    assert!(
        history
            .consume_current(
                &raw,
                &old_expected,
                &fixture::FixtureProvider::new(),
                fixture::NOW,
                None
            )
            .is_err()
    );
}
#[test]
fn same_attempt_same_bytes_replay_and_changed_bytes_conflict_without_history_mutation() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let (first, expected) = output("clean", "same", "task", "worktree", "R1", 'a', false);
    let (different, _) = output("clean", "same", "task", "worktree", "R1", 'a', true);
    let frozen = work(&expected);
    let target = frozen.target().clone();
    let ticket = history.register(&admin, "same", frozen, 0).unwrap();
    let original = first.domain_bytes().to_vec();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&first), fixture::NOW, 30)
        .unwrap();
    let changed = store
        .publish(&admin, AuditEvidence::Output(&different), fixture::NOW, 30)
        .unwrap();
    assert_eq!(
        history
            .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
            .unwrap(),
        ImportOutcome::Inserted
    );
    assert_eq!(
        history
            .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
            .unwrap(),
        ImportOutcome::IdenticalReplay
    );
    assert!(
        history
            .import(&admin, &raw, &ticket, &changed, fixture::NOW)
            .is_err()
    );
    assert_eq!(history.history(&admin, &target).unwrap().len(), 1);
    history
        .publish(&admin, &raw, &ticket, fixture::NOW)
        .unwrap();
    assert!(
        history
            .consume_current(
                &raw,
                &expected,
                &fixture::FixtureProvider::new(),
                fixture::NOW,
                None
            )
            .unwrap()
            .eligibility
            .eligible
    );
    assert_eq!(first.domain_bytes(), original);
}
#[test]
fn two_tasks_two_worktrees_and_two_requirements_keep_attempts_separate() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let mut targets = Vec::new();
    for task in ["task-one", "task-two"] {
        for tree in ["worktree-one", "worktree-two"] {
            for requirement in ["R1", "R2"] {
                let run = format!("{task}-{tree}-{requirement}");
                let (output, expected) = output("clean", &run, task, tree, requirement, 'a', false);
                let frozen = work(&expected);
                let target = frozen.target().clone();
                let ticket = history.register(&admin, &run, frozen, 0).unwrap();
                assert_eq!(ticket.generation(), 1);
                let receipt = store
                    .publish(&admin, AuditEvidence::Output(&output), fixture::NOW, 30)
                    .unwrap();
                history
                    .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
                    .unwrap();
                history
                    .publish(&admin, &raw, &ticket, fixture::NOW)
                    .unwrap();
                targets.push((target, run, expected));
            }
        }
    }
    for (target, run, expected) in &targets {
        assert_eq!(
            history.current(&admin, target).unwrap().unwrap().run_id,
            run
        );
        let records = history.history(&admin, target).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].run_id, run);
        assert!(
            history
                .consume_current(
                    &raw,
                    expected,
                    &fixture::FixtureProvider::new(),
                    fixture::NOW,
                    None
                )
                .unwrap()
                .eligibility
                .eligible
        );
    }
    let (_, different) = output(
        "clean",
        &targets[0].1,
        "third-task",
        "third-worktree",
        "R3",
        'a',
        false,
    );
    assert!(
        history
            .register(&admin, &targets[0].1, work(&different), 0)
            .is_err()
    );
    assert_eq!(
        history
            .current(&admin, &targets[0].0)
            .unwrap()
            .unwrap()
            .run_id,
        targets[0].1
    );
}
#[test]
fn registering_new_generation_clears_prior_current_even_before_completion() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let (first, expected) = output("clean", "first", "task", "tree", "R1", 'a', false);
    let frozen = work(&expected);
    let target = frozen.target().clone();
    let first_ticket = history.register(&admin, "first", frozen, 0).unwrap();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&first), fixture::NOW, 30)
        .unwrap();
    history
        .import(&admin, &raw, &first_ticket, &receipt, fixture::NOW)
        .unwrap();
    history
        .publish(&admin, &raw, &first_ticket, fixture::NOW)
        .unwrap();
    assert!(
        history
            .register(&admin, "wrong-generation", work(&expected), 0)
            .is_err()
    );
    assert!(history.current(&admin, &target).unwrap().is_some());
    let second = history
        .register(&admin, "pending", work(&expected), 1)
        .unwrap();
    assert_eq!(second.generation(), 2);
    assert!(history.current(&admin, &target).unwrap().is_none());
    assert!(
        history
            .publish(&admin, &raw, &first_ticket, fixture::NOW)
            .is_err()
    );
    assert!(
        history
            .consume_current(
                &raw,
                &expected,
                &fixture::FixtureProvider::new(),
                fixture::NOW,
                None
            )
            .is_err()
    );
    assert_eq!(history.history(&admin, &target).unwrap().len(), 1);
}
#[test]
fn foreign_ticket_and_wrong_actual_run_fail_without_append_or_publication() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let mut foreign = AttemptHistory::new(&store, &admin).unwrap();
    let (output, expected) = output("clean", "actual", "task", "tree", "R1", 'a', false);
    let frozen = work(&expected);
    let target = frozen.target().clone();
    let wrong = history.register(&admin, "wrong", frozen, 0).unwrap();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), fixture::NOW, 30)
        .unwrap();
    assert!(
        history
            .import(&admin, &raw, &wrong, &receipt, fixture::NOW)
            .is_err()
    );
    assert!(history.history(&admin, &target).unwrap().is_empty());
    assert!(history.publish(&admin, &raw, &wrong, fixture::NOW).is_err());
    let ticket = history
        .register(&admin, "actual", work(&expected), 1)
        .unwrap();
    foreign
        .register(&admin, "actual", work(&expected), 0)
        .unwrap();
    assert!(
        foreign
            .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
            .is_err()
    );
    assert!(foreign.history(&admin, &target).unwrap().is_empty());
    assert_eq!(
        history
            .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
            .unwrap(),
        ImportOutcome::Inserted
    );
}
#[test]
fn raw_namespace_permissions_and_retention_are_not_bypassed_by_import_or_current() {
    for condition in ["expiry", "purge", "missing"] {
        let root = Root::new();
        let (store, admin) = root.open();
        let other_root = Root::new();
        let (other, other_admin) = other_root.open();
        let raw = store.raw_access(&admin).unwrap();
        let foreign = other.raw_access(&other_admin).unwrap();
        assert!(AttemptHistory::new(&store, &other_admin).is_err());
        let mut history = AttemptHistory::new(&store, &admin).unwrap();
        let (output, expected) = output("clean", "one", "task", "tree", "R1", 'a', false);
        let frozen = work(&expected);
        let target = frozen.target().clone();
        let ticket = history.register(&admin, "one", frozen, 0).unwrap();
        let receipt = store
            .publish(&admin, AuditEvidence::Output(&output), fixture::NOW, 30)
            .unwrap();
        assert!(
            history
                .import(&admin, &foreign, &ticket, &receipt, fixture::NOW)
                .is_err()
        );
        assert!(history.current(&other_admin, &target).is_err());
        assert!(history.history(&other_admin, &target).is_err());
        history
            .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
            .unwrap();
        history
            .publish(&admin, &raw, &ticket, fixture::NOW)
            .unwrap();
        assert!(
            history
                .consume_current(
                    &foreign,
                    &expected,
                    &fixture::FixtureProvider::new(),
                    fixture::NOW,
                    None
                )
                .is_err()
        );
        let now = if condition == "expiry" {
            fixture::NOW + 30
        } else {
            fixture::NOW
        };
        if condition == "purge" {
            store.purge(&admin, &receipt, now).unwrap();
        }
        if condition == "missing" {
            std::fs::remove_file(root.0.join(receipt.key()).join("facts.json")).unwrap();
        }
        assert!(
            history
                .import(&admin, &raw, &ticket, &receipt, now)
                .is_err(),
            "replay {condition}"
        );
        assert!(
            history.publish(&admin, &raw, &ticket, now).is_err(),
            "publish {condition}"
        );
        assert!(
            history
                .consume_current(&raw, &expected, &fixture::FixtureProvider::new(), now, None)
                .is_err(),
            "consume {condition}"
        );
        assert_eq!(history.history(&admin, &target).unwrap().len(), 1);
    }
}
#[test]
fn caller_mutex_serializes_real_thread_cas_with_one_expected_generation_winner() {
    use std::sync::{Arc, Barrier, Mutex};
    let root = Root::new();
    let (store, admin) = root.open();
    let history = Mutex::new(AttemptHistory::new(&store, &admin).unwrap());
    let (_, expected) = output("clean", "fixture", "task", "tree", "R1", 'a', false);
    let barrier = Arc::new(Barrier::new(2));
    let target = work(&expected).target().clone();
    let outcomes = std::thread::scope(|scope| {
        let first_work = work(&expected);
        let second_work = work(&expected);
        let a = barrier.clone();
        let b = barrier.clone();
        let h = &history;
        let admin = &admin;
        let first = scope.spawn(move || {
            a.wait();
            h.lock().unwrap().register(admin, "race-one", first_work, 0)
        });
        let second = scope.spawn(move || {
            b.wait();
            h.lock()
                .unwrap()
                .register(admin, "race-two", second_work, 0)
        });
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|r| r.is_err()).count(), 1);
    assert!(
        history
            .lock()
            .unwrap()
            .current(&admin, &target)
            .unwrap()
            .is_none()
    );
    assert!(
        history
            .lock()
            .unwrap()
            .history(&admin, &target)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn current_approved_review_requeries_revocation_and_preserves_technical_bytes() {
    use guardengine::integration::{GuardRunEnvelope, eligibility::*};
    use std::cell::Cell;
    struct Authority {
        expected: ExpectedConsumption,
        revoked: Cell<bool>,
        unavailable: Cell<bool>,
        calls: Cell<usize>,
    }
    impl AuthorityProvider for Authority {
        fn verify_producer(
            &self,
            e: &GuardRunEnvelope,
            d: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            if d != self.expected.envelope_digest || e.producer != self.expected.policy.producer {
                return Err(AuthorityError::Untrusted);
            }
            Ok(ProducerRecord {
                principal: "fixture-producer".into(),
                producer: self.expected.policy.producer.clone(),
                envelope_digest: self.expected.envelope_digest.clone(),
                validity: Validity {
                    issued_at: 0,
                    expires_at: 2_000_000_000,
                    revoked: false,
                },
            })
        }
        fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
            self.calls.set(self.calls.get() + 1);
            if self.unavailable.get() {
                return Err(AuthorityError::Unavailable);
            }
            if r != "approval:one" {
                return Err(AuthorityError::Untrusted);
            }
            Ok(ApprovalRecord {
                principal: "fixture-reviewer".into(),
                purpose: "review".into(),
                action: self.expected.policy.action.clone(),
                binding: self.expected.policy.binding.clone(),
                contract_digest: self.expected.policy.contract_digest.clone(),
                validity: Validity {
                    issued_at: 0,
                    expires_at: 2_000_000_000,
                    revoked: self.revoked.get(),
                },
            })
        }
    }
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let (original, mut expected) = fixture::sample(true);
    let report = original.report_bytes().unwrap().to_vec();
    let native = original.domain_bytes().to_vec();
    let attached = original
        .with_approval_refs(&["approval:one".into()])
        .unwrap();
    expected.envelope_digest = digest(&serde_json::to_vec(attached.envelope()).unwrap());
    let ticket = history
        .register(&admin, &expected.run_id, work(&expected), 0)
        .unwrap();
    let receipt = store
        .publish(&admin, AuditEvidence::Attached(&attached), fixture::NOW, 30)
        .unwrap();
    history
        .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
        .unwrap();
    history
        .publish(&admin, &raw, &ticket, fixture::NOW)
        .unwrap();
    let authority = Authority {
        expected: expected.clone(),
        revoked: Cell::new(false),
        unavailable: Cell::new(false),
        calls: Cell::new(0),
    };
    let first = history
        .consume_current(&raw, &expected, &authority, fixture::NOW, None)
        .unwrap();
    assert!(first.eligibility.eligible);
    assert_eq!(
        first.eligibility.technical_decision,
        Some(guardengine::Decision::RequireApproval)
    );
    authority.revoked.set(true);
    let revoked = history
        .consume_current(&raw, &expected, &authority, fixture::NOW + 1, None)
        .unwrap();
    assert!(!revoked.eligibility.eligible);
    assert_eq!(
        revoked.eligibility.technical_decision,
        first.eligibility.technical_decision
    );
    authority.unavailable.set(true);
    assert!(
        !history
            .consume_current(&raw, &expected, &authority, fixture::NOW + 1, None)
            .unwrap()
            .eligibility
            .eligible
    );
    assert_eq!(authority.calls.get(), 3);
    let mut changed = expected.clone();
    changed.policy.binding.candidate_oid = "d".repeat(40);
    assert!(
        history
            .consume_current(&raw, &changed, &authority, fixture::NOW + 1, None)
            .is_err()
    );
    assert_eq!(authority.calls.get(), 3);
    assert_eq!(original.report_bytes().unwrap(), report);
    assert_eq!(original.domain_bytes(), native);
}
#[test]
fn attempt_capacity_and_invalid_frozen_inputs_fail_before_state_change() {
    let root = Root::new();
    let (store, admin) = root.open();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let (_, expected) = output("clean", "fixture", "task", "tree", "R1", 'a', false);
    let target = work(&expected).target().clone();
    for i in 0..256 {
        assert_eq!(
            history
                .register(&admin, &format!("attempt-{i}"), work(&expected), i)
                .unwrap()
                .generation(),
            i + 1
        );
    }
    assert!(
        history
            .register(&admin, "overflow", work(&expected), 256)
            .is_err()
    );
    assert!(history.current(&admin, &target).unwrap().is_none());
    assert!(history.history(&admin, &target).unwrap().is_empty());
    for case in 0..4 {
        let mut changed = expected.clone();
        match case {
            0 => changed.policy.binding.candidate_oid = "invalid".into(),
            1 => changed.policy.binding.requirement_ids = vec!["R2".into(), "R1".into()],
            2 => changed.policy.required_scopes = vec!["scope".into(); 65],
            _ => changed.mapping_digest = "not-a-digest".into(),
        }
        assert!(
            AttemptWork::freeze(
                &changed.policy.binding,
                &changed.policy.producer,
                &changed.policy.required_scopes,
                &changed.policy.contract_digest,
                &changed.mapping_digest
            )
            .is_err()
        );
    }
}
#[test]
fn borrowed_work_budget_rejects_large_binding_before_owned_clone_or_hash() {
    let (_, mut expected) = output("clean", "fixture", "task", "tree", "R1", 'a', false);
    expected.policy.binding.task_id = "s".repeat(17 * 1024 * 1024);
    MAX_ALLOC.with(|n| n.set(0));
    TRACK.with(|on| on.set(true));
    let rejected = AttemptWork::freeze(
        &expected.policy.binding,
        &expected.policy.producer,
        &expected.policy.required_scopes,
        &expected.policy.contract_digest,
        &expected.mapping_digest,
    )
    .is_err();
    TRACK.with(|on| on.set(false));
    assert!(rejected);
    assert!(MAX_ALLOC.with(|n| n.get()) < 4096);
}
#[test]
fn independently_changed_full_work_keys_do_not_reuse_current_observation() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let (output, expected) = output("clean", "one", "task", "tree", "R1", 'a', false);
    let ticket = history.register(&admin, "one", work(&expected), 0).unwrap();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&output), fixture::NOW, 30)
        .unwrap();
    history
        .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
        .unwrap();
    history
        .publish(&admin, &raw, &ticket, fixture::NOW)
        .unwrap();
    for case in 0..17 {
        let mut changed = expected.clone();
        match case {
            0 => changed.policy.binding.repo_id = "different".into(),
            1 => changed.policy.binding.task_id = "different".into(),
            2 => changed.policy.binding.worktree_id = "different".into(),
            3 => changed.policy.binding.requirement_ids = vec!["different".into()],
            4 => changed.policy.binding.candidate_oid = "c".repeat(40),
            5 => changed.policy.binding.base_oid = "d".repeat(40),
            6 => changed.policy.binding.merge_group_id = Some("other-queue".into()),
            7 => {
                changed.policy.binding.source_snapshot_digest = format!("sha256:{}", "c".repeat(64))
            }
            8 => {
                changed.policy.binding.baseline_digest = Some(format!("sha256:{}", "d".repeat(64)))
            }
            9 => changed.policy.producer.analyzer_version = "different".into(),
            10 => changed.policy.required_scopes = vec!["different".into()],
            11 => changed.policy.contract_digest = format!("sha256:{}", "d".repeat(64)),
            12 => changed.mapping_digest = format!("sha256:{}", "d".repeat(64)),
            13 => changed.run_id = "different".into(),
            14 => changed.envelope_digest = format!("sha256:{}", "d".repeat(64)),
            15 => changed.raw_domain_digest = format!("sha256:{}", "d".repeat(64)),
            _ => changed.policy.producer.version = "different".into(),
        }
        assert!(
            history
                .consume_current(
                    &raw,
                    &changed,
                    &fixture::FixtureProvider::new(),
                    fixture::NOW,
                    None
                )
                .is_err(),
            "case {case}"
        );
    }
    assert!(
        history
            .consume_current(
                &raw,
                &expected,
                &fixture::FixtureProvider::new(),
                fixture::NOW,
                None
            )
            .unwrap()
            .eligibility
            .eligible
    );
}
#[test]
fn partial_current_remains_blocked_and_never_reuses_prior_complete_evidence() {
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let (partial, expected) = output("missing-tool", "partial", "task", "tree", "R1", 'a', false);
    let ticket = history
        .register(&admin, "partial", work(&expected), 0)
        .unwrap();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&partial), fixture::NOW, 30)
        .unwrap();
    history
        .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
        .unwrap();
    history
        .publish(&admin, &raw, &ticket, fixture::NOW)
        .unwrap();
    let result = history
        .consume_current(
            &raw,
            &expected,
            &fixture::FixtureProvider::new(),
            fixture::NOW,
            None,
        )
        .unwrap();
    assert!(!result.eligibility.eligible);
    assert_eq!(
        result.eligibility.technical_decision,
        Some(guardengine::Decision::Block)
    );
    assert_eq!(
        result.eligibility.code,
        guardengine::integration::eligibility::EligibilityCode::Incomplete
    );
}
#[path = "support/guard_integration.rs"]
mod native_fixture;
#[test]
fn bound_failed_attempt_can_be_current_metadata_without_becoming_engine_evidence() {
    use codeguard_cli::guard_integration::{
        profile::InvocationDescriptor, reader::read_native, scope::FrozenObligations,
    };
    let mut native = native_fixture::valid();
    native["exit_code"] = 4.into();
    native["command_status"] = "internal_error".into();
    let mut invocation = serde_json::to_value(native_fixture::invocation()).unwrap();
    invocation["process_exit"] = 4.into();
    let invocation =
        InvocationDescriptor::parse(&serde_json::to_vec(&invocation).unwrap()).unwrap();
    let bytes = serde_json::to_vec(&native).unwrap();
    let evidence = read_native(&bytes, &invocation).unwrap();
    let (_, mut expected) = fixture::sample(false);
    expected.policy.binding.source_snapshot_digest = format!("sha256:{}", "a".repeat(64));
    let scope = FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    let failed = FrozenRun::new(
        "native-001",
        expected.policy.binding.clone(),
        &scope,
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap()
    .failure(&evidence)
    .unwrap();
    expected.run_id = "native-001".into();
    expected.envelope_digest = digest(&serde_json::to_vec(failed.envelope()).unwrap());
    expected.raw_domain_digest = digest(&bytes);
    expected.policy.required_scopes = failed.envelope().coverage.required_scopes.clone();
    let root = Root::new();
    let (store, admin) = root.open();
    let raw = store.raw_access(&admin).unwrap();
    let mut history = AttemptHistory::new(&store, &admin).unwrap();
    let frozen = work(&expected);
    let target = frozen.target().clone();
    let ticket = history.register(&admin, "native-001", frozen, 0).unwrap();
    let receipt = store
        .publish(&admin, AuditEvidence::Output(&failed), fixture::NOW, 30)
        .unwrap();
    history
        .import(&admin, &raw, &ticket, &receipt, fixture::NOW)
        .unwrap();
    history
        .publish(&admin, &raw, &ticket, fixture::NOW)
        .unwrap();
    assert_eq!(
        history.current(&admin, &target).unwrap().unwrap().run_id,
        "native-001"
    );
    assert!(
        history
            .consume_current(
                &raw,
                &expected,
                &fixture::FixtureProvider::new(),
                fixture::NOW,
                None
            )
            .is_err()
    );
    assert_eq!(failed.domain_bytes(), bytes);
    assert!(failed.report_bytes().is_none());
}
