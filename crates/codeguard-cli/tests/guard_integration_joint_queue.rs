//! Retained real four-candidate native captures; fresh local projection/authority only.
#![cfg(target_os = "linux")]
use codeguard_cli::guard_integration::{
    audit::{
        AuditEvidence,
        access::{Administration, AuditAccess, RetentionPolicy},
        attempts::{AttemptHistory, AttemptWork},
    },
    consumer::{BaselineRequirement, ExpectedConsumption, PreparedConsumption},
    envelope::{EnvelopeOutput, FrozenRun},
    projection::ProtectedMapping,
    ruff_profile::{RuffF401Policy, project_ruff, read_ruff_feedback},
};
use codeguard_runtime::private_artifact_store::StoreLimits;
use guardengine::{
    GuardContract, GuardSubject,
    integration::{GuardRunEnvelope, RunBinding, eligibility::*},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Barrier, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
const NOW: i64 = 2_000_000_000;
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn digest(b: &[u8]) -> String {
    format!("sha256:{}", sha(b))
}
fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/guard-integration/joint")
}
fn read(p: impl AsRef<Path>) -> Vec<u8> {
    std::fs::read(p).unwrap()
}
fn value(p: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&read(p)).unwrap()
}
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cg-joint-{}-{}",
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
    fn store(&self) -> (AuditAccess, Administration) {
        let p = self.0.join("store");
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        AuditAccess::open(
            &p,
            StoreLimits {
                max_entries: 64,
                max_total_bytes: 8 * 1024 * 1024,
            },
            "joint",
            RetentionPolicy {
                max_age_seconds: 600,
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
fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let o = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    o.stdout
}
struct Group {
    output: EnvelopeOutput,
    expected: ExpectedConsumption,
    prepared: PreparedConsumption,
    approval: ApprovalRecord,
}
fn group(name: &str, repo: &Path, baseline: &str) -> Group {
    let path = corpus().join(name).join("codeguard");
    let context = value(path.join("context.json"));
    let original: RunBinding = serde_json::from_value(context["binding"].clone()).unwrap();
    let candidate = value(corpus().join(name).join("gitguard/candidate.json"));
    assert_eq!(candidate["candidate_oid"], original.candidate_oid);
    assert_eq!(candidate["base_oid"], original.base_oid);
    assert_eq!(
        candidate["requirement_ids"],
        json!(original.requirement_ids)
    );
    assert_eq!(candidate["merge_group_id"], json!(original.merge_group_id));
    assert_eq!(
        String::from_utf8(git(repo, &["cat-file", "-t", &original.candidate_oid]))
            .unwrap()
            .trim(),
        "commit"
    );
    let source = git(
        repo,
        &["show", &format!("{}:app.py", original.candidate_oid)],
    );
    assert_eq!(digest(&source), original.source_snapshot_digest);
    let raw = read(path.join("native.json"));
    let native: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(native["schema_version"], "0.13.0");
    assert_eq!(native["run_id"], context["runId"]);
    let producer = context["producerSha256"].as_str().unwrap();
    let run = context["runId"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        original
            .source_snapshot_digest
            .strip_prefix("sha256:")
            .unwrap(),
        producer,
    )
    .unwrap();
    let native = read_ruff_feedback(&raw, &policy, run, 3, producer).unwrap();
    let mapping = ProtectedMapping::parse(&read(path.join("mapping.json"))).unwrap();
    let contract: GuardContract =
        serde_json::from_slice(&read(path.join("contract.json"))).unwrap();
    let project = || {
        project_ruff(
            &native,
            &mapping,
            &contract,
            GuardSubject {
                id: original.repo_id.clone(),
                snapshot_digest: original.source_snapshot_digest.clone(),
            },
        )
        .unwrap()
    };
    let original_output = FrozenRun::new(
        run,
        original.clone(),
        &policy.obligations(),
        context["startedAt"].as_str().unwrap(),
        context["finishedAt"].as_str().unwrap(),
    )
    .unwrap()
    .complete(project())
    .unwrap();
    assert_eq!(
        serde_json::to_value(original_output.envelope()).unwrap(),
        value(path.join("envelope.json"))
    );
    assert_eq!(original_output.domain_bytes(), raw);
    // Fresh supplemental baseline context over the unchanged original native bytes/IDs.
    // It is NOT the original producer's authenticated envelope or historical approval.
    let mut binding = original.clone();
    binding.baseline_digest = Some(format!("sha256:{baseline}"));
    let output = FrozenRun::new(
        run,
        binding,
        &policy.obligations(),
        context["startedAt"].as_str().unwrap(),
        context["finishedAt"].as_str().unwrap(),
    )
    .unwrap()
    .complete(project())
    .unwrap();
    let e = output.envelope();
    let policy = EligibilityPolicy {
        binding: e.binding.clone(),
        producer: e.producer.clone(),
        required_scopes: e.coverage.required_scopes.clone(),
        contract_digest: e.artifacts.contract.as_ref().unwrap().digest.clone(),
        action: "merge".into(),
        producer_principals: BTreeSet::from(["fixture-producer".into()]),
        approval_principals: BTreeMap::new(),
    };
    let expected = ExpectedConsumption {
        policy: policy.clone(),
        run_id: run.into(),
        mapping_digest: mapping.digest(),
        envelope_digest: digest(&serde_json::to_vec(e).unwrap()),
        raw_domain_digest: digest(&raw),
    };
    let mut bp = policy.clone();
    bp.approval_principals = BTreeMap::from([(
        "baseline".into(),
        BTreeSet::from(["fixture-baseline".into()]),
    )]);
    let required = BaselineRequirement {
        reference: format!("fixture:baseline:{name}"),
        purpose: "baseline".into(),
        policy: bp,
    };
    let prepared = PreparedConsumption::freeze(&expected, Some(&required)).unwrap();
    let approval = ApprovalRecord {
        principal: "fixture-baseline".into(),
        purpose: "baseline".into(),
        action: policy.action,
        binding: policy.binding,
        contract_digest: policy.contract_digest,
        validity: Validity {
            issued_at: NOW - 10,
            expires_at: NOW + 100,
            revoked: false,
        },
    };
    Group {
        output,
        expected,
        prepared,
        approval,
    }
}
struct Authority {
    records: BTreeMap<String, ProducerRecord>,
    approvals: BTreeMap<String, ApprovalRecord>,
    revoke_a: AtomicBool,
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.records
            .get(d)
            .cloned()
            .ok_or(AuthorityError::Untrusted)
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        let mut a = self
            .approvals
            .get(r)
            .cloned()
            .ok_or(AuthorityError::Untrusted)?;
        if r == "fixture:baseline:A" {
            a.validity.revoked = self.revoke_a.load(Ordering::SeqCst);
        }
        Ok(a)
    }
}
fn work(e: &ExpectedConsumption) -> AttemptWork {
    AttemptWork::freeze(
        &e.policy.binding,
        &e.policy.producer,
        &e.policy.required_scopes,
        &e.policy.contract_digest,
        &e.mapping_digest,
    )
    .unwrap()
}
#[test]
fn real_queue_parallel_consumption_rechecks_baseline_and_late_current() {
    let corpus = corpus();
    let transport = read(corpus.join("TRANSPORT.json"));
    assert_eq!(
        sha(&transport),
        "6581f212c0b5b5afc4c84f184592fe128ad09cc66f6301784cb630a83db79b7d"
    );
    let manifest: Value = serde_json::from_slice(&transport).unwrap();
    for (name, h) in manifest["files"].as_object().unwrap() {
        assert_eq!(sha(&read(corpus.join(name))), h.as_str().unwrap());
    }
    let root = Root::new();
    git(
        &root.0,
        &[
            "clone",
            "--bare",
            "-q",
            corpus.join("source.bundle").to_str().unwrap(),
            "repo",
        ],
    );
    let repo = root.0.join("repo");
    let producer = value(corpus.join("producer-manifest.json"));
    let baseline = producer["project_digest"].as_str().unwrap();
    let groups: BTreeMap<_, _> = ["A", "B", "M1", "M2"]
        .into_iter()
        .map(|n| (n, group(n, &repo, baseline)))
        .collect();
    let p = &producer["profiles"];
    let m1 = String::from_utf8(git(
        &repo,
        &[
            "show",
            "-s",
            "--format=%P",
            p["M1"]["candidate"].as_str().unwrap(),
        ],
    ))
    .unwrap();
    assert_eq!(
        m1.trim(),
        format!(
            "{} {}",
            p["A"]["candidate"].as_str().unwrap(),
            p["B"]["candidate"].as_str().unwrap()
        )
    );
    assert_ne!(p["M1"]["base"], p["M2"]["base"]);
    assert_ne!(p["M1"]["members"], p["M2"]["members"]);
    let authority = Authority {
        records: groups
            .values()
            .map(|g| {
                (
                    g.expected.envelope_digest.clone(),
                    ProducerRecord {
                        principal: "fixture-producer".into(),
                        producer: g.expected.policy.producer.clone(),
                        envelope_digest: g.expected.envelope_digest.clone(),
                        validity: Validity {
                            issued_at: NOW - 10,
                            expires_at: NOW + 100,
                            revoked: false,
                        },
                    },
                )
            })
            .collect(),
        approvals: groups
            .iter()
            .map(|(n, g)| (format!("fixture:baseline:{n}"), g.approval.clone()))
            .collect(),
        revoke_a: AtomicBool::new(false),
    };
    let (store, admin) = root.store();
    let raw = store.raw_access(&admin).unwrap();
    let history = Mutex::new(AttemptHistory::new(&store, &admin).unwrap());
    let mut tickets = BTreeMap::new();
    let mut receipts = BTreeMap::new();
    for n in ["A", "B", "M1", "M2"] {
        let g = &groups[n];
        let t = history
            .lock()
            .unwrap()
            .register(
                &admin,
                &g.expected.run_id,
                work(&g.expected),
                if n == "M2" { 1 } else { 0 },
            )
            .unwrap();
        tickets.insert(n, t);
        receipts.insert(
            n,
            store
                .publish(&admin, AuditEvidence::Output(&g.output), NOW, 100)
                .unwrap(),
        );
    }
    for n in ["A", "B", "M2"] {
        let mut h = history.lock().unwrap();
        h.import(&admin, &raw, &tickets[n], &receipts[n], NOW)
            .unwrap();
        h.publish(&admin, &raw, &tickets[n], NOW).unwrap();
    }
    let barrier = Barrier::new(2);
    std::thread::scope(|s| {
        let handles: Vec<_> = ["A", "B"]
            .into_iter()
            .map(|n| {
                let barrier = &barrier;
                let g = &groups[n];
                let history = &history;
                let raw = &raw;
                let authority = &authority;
                s.spawn(move || {
                    barrier.wait();
                    let h = history.lock().unwrap();
                    // Controller-owned Mutex serializes complete current-state observations.
                    let result = h
                        .consume_current_prepared(raw, &g.prepared, authority, NOW, None)
                        .unwrap();
                    assert!(result.eligibility.eligible);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });

    let mut h = history.lock().unwrap();
    let mut observed = BTreeMap::new();
    for n in ["A", "B", "M2"] {
        let g = &groups[n];
        let result = h
            .consume_current_prepared(&raw, &g.prepared, &authority, NOW, None)
            .unwrap();
        assert!(result.eligibility.eligible);
        observed.insert(n,json!({"envelope":g.output.envelope(),"native_sha256":sha(g.output.domain_bytes()),"expected":g.expected,"receipt":receipts[n],"eligibility":result.eligibility.eligible,"audit":result.eligibility.audit}));
        // The old source-only entry remains explicitly unsupported for non-null baseline.
        assert!(
            h.consume_current(&raw, &g.expected, &authority, NOW, None)
                .is_err()
        );
    }
    authority.revoke_a.store(true, Ordering::SeqCst);
    let revoked = h
        .consume_current_prepared(&raw, &groups["A"].prepared, &authority, NOW, None)
        .unwrap();
    assert_eq!(revoked.eligibility.code, EligibilityCode::InvalidApproval);
    assert!(!revoked.eligibility.eligible);
    assert!(
        h.consume_current_prepared(&raw, &groups["B"].prepared, &authority, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    authority.revoke_a.store(false, Ordering::SeqCst);
    // M2 is current before old M1 finishes. Store retains M1 but cannot publish it.
    h.import(&admin, &raw, &tickets["M1"], &receipts["M1"], NOW)
        .unwrap();
    assert!(h.publish(&admin, &raw, &tickets["M1"], NOW).is_err());
    assert!(
        h.consume_current_prepared(&raw, &groups["M1"].prepared, &authority, NOW, None)
            .is_err()
    );
    assert!(
        h.consume_current_prepared(&raw, &groups["M2"].prepared, &authority, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    let queue_target = work(&groups["M2"].expected).target().clone();
    assert_eq!(
        h.current(&admin, &queue_target).unwrap().unwrap().run_id,
        groups["M2"].expected.run_id
    );
    assert_eq!(h.history(&admin, &queue_target).unwrap().len(), 2);
    for (old, new) in [("A", "B"), ("A", "M1"), ("M1", "M2")] {
        assert!(
            groups[new]
                .prepared
                .consume(&groups[old].output, &authority, NOW, None)
                .is_err()
        );
    }
    // New base/source/current expectations cannot reuse the old stored receipt.
    let mut stale = groups["M2"].expected.clone();
    stale.policy.binding.base_oid = groups["M1"].expected.policy.binding.base_oid.clone();
    let mut baseline_policy = stale.policy.clone();
    baseline_policy.approval_principals = BTreeMap::from([(
        "baseline".into(),
        BTreeSet::from(["fixture-baseline".into()]),
    )]);
    let changed = PreparedConsumption::freeze(
        &stale,
        Some(&BaselineRequirement {
            reference: "fixture:baseline:M2".into(),
            purpose: "baseline".into(),
            policy: baseline_policy,
        }),
    )
    .unwrap();
    assert!(
        h.consume_current_prepared(&raw, &changed, &authority, NOW, None)
            .is_err()
    );
    for n in ["A", "B", "M1", "M2"] {
        let retained = store.read_raw(&raw, &receipts[n], NOW).unwrap();
        assert_eq!(
            retained.domain_bytes(),
            read(corpus.join(n).join("codeguard/native.json"))
        );
        assert_eq!(
            serde_json::to_vec(retained.envelope()).unwrap(),
            serde_json::to_vec(groups[n].output.envelope()).unwrap()
        );
    }
    let m1 = &groups["M1"];
    let m1_consumed = m1
        .prepared
        .consume(&m1.output, &authority, NOW, None)
        .unwrap();
    assert!(m1_consumed.eligibility.eligible);
    observed.insert("M1", json!({"envelope":m1.output.envelope(),"native_sha256":sha(m1.output.domain_bytes()),"expected":m1.expected,"receipt":receipts["M1"],"standalone_eligible":true,"current_eligible":false,"audit":m1_consumed.eligibility.audit}));
    assert!(
        h.consume_current_prepared(&raw, &groups["M2"].prepared, &authority, NOW + 100, None)
            .is_err()
    );
    for (name, hash) in manifest["files"].as_object().unwrap() {
        assert_eq!(sha(&read(corpus.join(name))), hash.as_str().unwrap());
    }
    let result = json!({"profile":"codeguard.local-joint-queue/v1","production_authenticated":false,"native_rerun":false,"original_native_bytes_and_ids_unchanged":true,"transport_sha256":sha(&transport),"groups":observed,"m1_retained_receipt":receipts["M1"],"parallel_A_B_consumption":true,"scan_then_A_baseline_revoked":revoked.eligibility.code,"B_unaffected":true,"cross_context_rejected":["A-to-B","A-to-M1","M1-to-M2"],"M1_late_publication_refused":true,"M2_new_base_requires_new_expectation":true,"current_after_late":h.current(&admin,&queue_target).unwrap().unwrap().run_id,"history_count":2,"retention_expiry_refused":true,"additional_baseline_context":"new controller fixture projection; original native report remains unchanged","execution_authorized":false});
    if let Ok(path) = std::env::var("CODEGUARD_JOINT_OUTPUT") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &result).unwrap();
    }
}
