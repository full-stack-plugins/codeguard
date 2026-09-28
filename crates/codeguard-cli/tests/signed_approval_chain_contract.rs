use codeguard_cli::approval_snapshot::{SnapshotResolution, verify_and_bind_replacement_chain};
use codeguard_cli::false_positive_decision::parse_false_positive_decision_candidate;
use codeguard_cli::{ApprovalTrustKey, ApprovalVerificationContext, SignedPriorApprovalInput};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

struct Chain {
    decisions: Vec<Vec<u8>>,
    snapshots: Vec<Vec<u8>>,
    envelopes: Vec<Vec<u8>>,
    trust: ApprovalTrustKey,
}
fn context(index: usize) -> ApprovalVerificationContext<'static> {
    ApprovalVerificationContext {
        workspace_id: "workspace-one",
        policy_revision: (["r1", "r2", "r3"][index]),
        baseline_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        now_unix: Some((index as u64 + 1) * 100 + 50),
        minimum_sequence: index as u64 + 1,
        max_lifetime_seconds: 100,
    }
}
fn sign(
    pair: &Ed25519KeyPair,
    snapshot: &[u8],
    index: usize,
    changes: Option<(&str, Value)>,
) -> Vec<u8> {
    let mut payload = json!({"schema_version":"1.0","workspace_id":"workspace-one","policy_revision":(["r1","r2","r3"][index]),"baseline_commit":"a".repeat(40),"revision_sequence":index+1,"issued_at":(index+1)*100,"expires_at":(index+2)*100,"snapshot_sha256":format!("{:x}",Sha256::digest(snapshot))});
    if let Some((field, value)) = changes {
        payload[field] = value;
    }
    let raw = serde_json::to_string(&payload).unwrap();
    let mut message = b"codeguard.approval.v1\0key-one\0".to_vec();
    message.extend_from_slice(raw.as_bytes());
    let signature = pair
        .sign(&message)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    serde_json::to_vec(&json!({"schema_version":"1.0","key_id":"key-one","approval_json":raw,"signature_hex":signature})).unwrap()
}
impl Chain {
    fn new() -> Self {
        let pair = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
        let mut decisions = Vec::new();
        let mut snapshots = Vec::new();
        let mut envelopes = Vec::new();
        for index in 0..3 {
            let mut candidate = json!({"schema_version":if index==0 {"1.0"} else {"1.1"},"kind":"false_positive","id":format!("FP-{}",index+1),"identity":{"finding_id":"CG-1","checker_id":"ruff","native_rule_id":"F401","category":"lint","target":{"kind":"source","path":"app.py","file_sha256":"a".repeat(64)},"finding_fingerprint":"b".repeat(64),"tool_sha256":"c".repeat(64),"adapter_sha256":"d".repeat(64),"rulepack_sha256":"e".repeat(64)},"reason_code":"native_false_positive","rationale":"原生复核","reproducer_ref":"case-1","approved_policy_revision":(["r1","r2","r3"][index]),"approval_ref":"review-1","reviewer":"reviewer-1","created_at":(index+1)*100,"expires_at":(index+2)*100});
            if index > 0 {
                candidate["replaces_decision_id"] = json!(format!("FP-{index}"));
            }
            let bytes = serde_json::to_vec(&candidate).unwrap();
            let mut snapshot = json!({"schema_version":if index==0 {"1.0"} else {"1.1"},"policy_revision":(["r1","r2","r3"][index]),"max_lifetime_seconds":100,"decisions":[{"id":format!("FP-{}",index+1),"sha256":format!("{:x}",Sha256::digest(&bytes))}]});
            if index > 0 {
                snapshot["revoked_decision_ids"] = json!([format!("FP-{index}")]);
            }
            let raw = serde_json::to_vec(&snapshot).unwrap();
            envelopes.push(sign(&pair, &raw, index, None));
            snapshots.push(raw);
            decisions.push(bytes);
        }
        Self {
            decisions,
            snapshots,
            envelopes,
            trust: ApprovalTrustKey {
                key_id: "key-one".into(),
                public_key: pair.public_key().as_ref().try_into().unwrap(),
                valid_from: 1,
                valid_until: 1000,
                revoked: false,
            },
        }
    }
    fn prior(&self, index: usize) -> SignedPriorApprovalInput<'_> {
        SignedPriorApprovalInput {
            decision_bytes: &self.decisions[index],
            snapshot_bytes: &self.snapshots[index],
            envelope_bytes: &self.envelopes[index],
            trust: &self.trust,
            context: context(index),
        }
    }
    fn bind(
        &self,
        priors: &[SignedPriorApprovalInput<'_>],
    ) -> Result<SnapshotResolution, &'static str> {
        let observed = parse_false_positive_decision_candidate(&self.decisions[2])
            .unwrap()
            .identity;
        verify_and_bind_replacement_chain(
            &self.decisions[2],
            &observed,
            &self.snapshots[2],
            &self.envelopes[2],
            &self.trust,
            &context(2),
            priors,
        )
    }
}

#[test]
fn every_historical_signature_is_required_in_a_valid_replacement_chain() {
    let mut chain = Chain::new();
    assert_eq!(
        chain.bind(&[chain.prior(1), chain.prior(0)]).unwrap(),
        SnapshotResolution::BoundToPinnedSnapshot
    );
    assert_ne!(
        chain.bind(&[chain.prior(1)]).unwrap(),
        SnapshotResolution::BoundToPinnedSnapshot
    );
    assert_ne!(
        chain.bind(&[chain.prior(0), chain.prior(1)]),
        Ok(SnapshotResolution::BoundToPinnedSnapshot)
    );
    chain.envelopes[0] = sign(
        &Ed25519KeyPair::from_seed_unchecked(&[8; 32]).unwrap(),
        &chain.snapshots[0],
        0,
        None,
    );
    assert!(chain.bind(&[chain.prior(1), chain.prior(0)]).is_err());
}

#[test]
fn revoked_historical_key_and_oversized_chain_cannot_be_hidden_by_current_signature() {
    let chain = Chain::new();
    let mut revoked = chain.trust.clone();
    revoked.revoked = true;
    let mut root = chain.prior(0);
    root.trust = &revoked;
    assert_eq!(
        chain.bind(&[chain.prior(1), root]),
        Err("approval_key_revoked")
    );
    assert_eq!(
        chain.bind(&[]),
        Ok(SnapshotResolution::RevisionChainTruncated)
    );
    assert_eq!(
        chain.bind(&vec![chain.prior(0); 33]),
        Ok(SnapshotResolution::RevisionChainTooLong)
    );
}

#[test]
fn signed_rollback_future_order_scope_and_missing_host_context_are_rejected() {
    for change in [
        "sequence",
        "issued",
        "workspace",
        "clock",
        "candidate_time",
        "extra",
    ] {
        let mut chain = Chain::new();
        let pair = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
        match change {
            "sequence" => {
                chain.envelopes[1] = sign(
                    &pair,
                    &chain.snapshots[1],
                    1,
                    Some(("revision_sequence", json!(3))),
                )
            }
            "issued" => {
                chain.envelopes[1] = sign(
                    &pair,
                    &chain.snapshots[1],
                    1,
                    Some(("issued_at", json!(320))),
                )
            }
            "workspace" => {
                chain.envelopes[0] = sign(
                    &pair,
                    &chain.snapshots[0],
                    0,
                    Some(("workspace_id", json!("other"))),
                )
            }
            "candidate_time" => {
                let mut c: Value = serde_json::from_slice(&chain.decisions[0]).unwrap();
                c["created_at"] = json!(160);
                chain.decisions[0] = serde_json::to_vec(&c).unwrap();
                let mut s: Value = serde_json::from_slice(&chain.snapshots[0]).unwrap();
                s["decisions"][0]["sha256"] =
                    json!(format!("{:x}", Sha256::digest(&chain.decisions[0])));
                chain.snapshots[0] = serde_json::to_vec(&s).unwrap();
                chain.envelopes[0] = sign(&pair, &chain.snapshots[0], 0, None);
            }
            _ => {}
        }
        let mut priors = vec![chain.prior(1), chain.prior(0)];
        if change == "clock" {
            priors[0].context.now_unix = None;
        }
        if change == "extra" {
            priors.push(chain.prior(0));
        }
        assert_ne!(
            chain.bind(&priors),
            Ok(SnapshotResolution::BoundToPinnedSnapshot),
            "{change}"
        );
    }
}

#[test]
fn valid_prior_signature_cannot_claim_review_after_its_replacement_was_issued() {
    let mut chain = Chain::new();
    let pair = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    chain.envelopes[1] = sign(
        &pair,
        &chain.snapshots[1],
        1,
        Some(("expires_at", json!(400))),
    );
    let mut prior = chain.prior(1);
    prior.context.now_unix = Some(350);
    prior.context.max_lifetime_seconds = 300;
    // 前序签名在审核时刻仍有效，但审核晚于子批准的签发时刻。
    assert_eq!(
        chain.bind(&[prior, chain.prior(0)]),
        Err("approval_revision_order_invalid")
    );
}

#[cfg(unix)]
#[test]
fn native_git_binding_rejects_signed_but_unrelated_prior_baselines() {
    use codeguard_cli::{GitApprovalBaselineContext, verify_and_bind_replacement_chain_with_git};
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};

    let root = std::env::temp_dir().join(format!("cg-signed-git-chain-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let git = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|p| p.join("git"))
        .find(|p| p.is_file())
        .unwrap()
        .canonicalize()
        .unwrap();
    let run = |args: &[&str]| {
        let output = Command::new(&git)
            .current_dir(&root)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    run(&["init", "-q"]);
    let tree = run(&["mktree"]);
    let first = run(&[
        "-c",
        "user.name=fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit-tree",
        &tree,
        "-m",
        "first",
    ]);
    let second = run(&[
        "-c",
        "user.name=fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit-tree",
        &tree,
        "-p",
        &first,
        "-m",
        "second",
    ]);
    let third = run(&[
        "-c",
        "user.name=fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit-tree",
        &tree,
        "-p",
        &second,
        "-m",
        "third",
    ]);
    let unrelated = run(&[
        "-c",
        "user.name=fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit-tree",
        &tree,
        "-m",
        "unrelated",
    ]);
    let mut chain = Chain::new();
    let pair = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    for (index, baseline) in [&first, &second, &third].iter().enumerate() {
        chain.envelopes[index] = sign(
            &pair,
            &chain.snapshots[index],
            index,
            Some(("baseline_commit", json!(baseline))),
        );
    }
    let observed = parse_false_positive_decision_candidate(&chain.decisions[2])
        .unwrap()
        .identity;
    let mut current = context(2);
    current.baseline_commit = &third;
    let mut middle = chain.prior(1);
    middle.context.baseline_commit = &second;
    let mut earliest = chain.prior(0);
    earliest.context.baseline_commit = &first;
    let cancelled = AtomicBool::new(false);
    let host = GitApprovalBaselineContext {
        root: &root,
        git_tool: &git,
        deadline: Instant::now() + Duration::from_secs(15),
        cancelled: &cancelled,
    };
    let preview = codeguard_cli::bind_signed_false_positive_replacement_preview(
        &chain.decisions[2],
        &observed,
        &chain.snapshots[2],
        &chain.envelopes[2],
        &chain.trust,
        &current,
        &[middle, earliest],
        &host,
    )
    .unwrap();
    assert_eq!(preview.preview().decision_id, "FP-3");
    assert_eq!(preview.preview().expires_at, 400);
    assert!(!preview.preview().independent_approval_verified);
    assert_eq!(preview.baseline_commit(), third);
    assert!(
        codeguard_cli::bind_signed_false_positive_preview(
            &chain.decisions[2],
            &observed,
            &chain.snapshots[2],
            &chain.envelopes[2],
            &chain.trust,
            &current,
        )
        .is_err()
    );
    assert_eq!(
        verify_and_bind_replacement_chain_with_git(
            &chain.decisions[2],
            &observed,
            &chain.snapshots[2],
            &chain.envelopes[2],
            &chain.trust,
            &current,
            &[middle, earliest],
            &host
        ),
        Ok(SnapshotResolution::BoundToPinnedSnapshot)
    );
    cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        verify_and_bind_replacement_chain_with_git(
            &chain.decisions[2],
            &observed,
            &chain.snapshots[2],
            &chain.envelopes[2],
            &chain.trust,
            &current,
            &[middle, earliest],
            &host,
        ),
        Err("git_ancestry_execution_incomplete")
    );
    cancelled.store(false, std::sync::atomic::Ordering::Relaxed);
    chain.envelopes[0] = sign(
        &pair,
        &chain.snapshots[0],
        0,
        Some(("baseline_commit", json!(unrelated))),
    );
    let mut earliest = chain.prior(0);
    earliest.context.baseline_commit = &unrelated;
    let mut middle = chain.prior(1);
    middle.context.baseline_commit = &second;
    assert_eq!(
        verify_and_bind_replacement_chain_with_git(
            &chain.decisions[2],
            &observed,
            &chain.snapshots[2],
            &chain.envelopes[2],
            &chain.trust,
            &current,
            &[middle, earliest],
            &host
        ),
        Err("approval_baseline_not_ancestor")
    );
    assert!(matches!(
        codeguard_cli::bind_signed_false_positive_replacement_preview(
            &chain.decisions[2],
            &observed,
            &chain.snapshots[2],
            &chain.envelopes[2],
            &chain.trust,
            &current,
            &[middle, earliest],
            &host,
        ),
        Err("approval_baseline_not_ancestor")
    ));
    let mut earliest = chain.prior(0);
    earliest.context.baseline_commit = &unrelated;
    let mut middle = chain.prior(1);
    middle.context.baseline_commit = &second;
    fs::write(root.join(".git/shallow"), format!("{third}\n")).unwrap();
    assert_eq!(
        verify_and_bind_replacement_chain_with_git(
            &chain.decisions[2],
            &observed,
            &chain.snapshots[2],
            &chain.envelopes[2],
            &chain.trust,
            &current,
            &[middle, earliest],
            &host
        ),
        Err("git_ancestry_shallow_history")
    );
    fs::remove_dir_all(root).unwrap();
}
