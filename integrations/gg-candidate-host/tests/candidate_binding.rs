use codeguard_gg_candidate_host::PreparedCandidate;
use gitguard::{
    Repository, candidate::CandidateRequest, scope::TaskScope, subject::SubjectRequest,
};
use std::{path::Path, process::Command};
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn fixture(format: &str) -> (tempfile::TempDir, Vec<u8>) {
    let root = tempfile::tempdir().unwrap();
    git(
        root.path(),
        &["init", "-q", &format!("--object-format={format}")],
    );
    std::fs::write(root.path().join("app.py"), "print('clean')\n").unwrap();
    std::fs::write(
        root.path().join("pyproject.toml"),
        codeguard_cli::guard_integration::ruff_profile::CONFIG,
    )
    .unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "candidate"]);
    let oid = git(root.path(), &["rev-parse", "HEAD"]);
    let repo = Repository::discover(root.path(), "repo").unwrap();
    let subject = repo
        .resolve_subject(SubjectRequest::Commit(oid.clone()))
        .unwrap();
    let scope = TaskScope::advisory(
        "task",
        vec!["R".into()],
        vec![b"app.py".to_vec(), b"pyproject.toml".to_vec()],
        &"a".repeat(64),
        None,
    )
    .unwrap();
    let snapshot = repo
        .prepare_candidate(
            &subject,
            &scope,
            &CandidateRequest {
                worktree_id: "worktree".into(),
                base_oid: oid,
                merge_group_id: None,
                members: vec![],
            },
        )
        .unwrap();
    (root, serde_json::to_vec(&snapshot).unwrap())
}
#[test]
fn malformed_protected_candidate_cannot_start_native_capture() {
    assert!(
        PreparedCandidate::from_bytes(b"{}", Path::new("/no-such-repository"), "app.py").is_err()
    );
}
#[test]
fn actual_sha1_and_sha256_trees_prepare_without_relabeling_file_digest() {
    for format in ["sha1", "sha256"] {
        let (root, expected) = fixture(format);
        let prepared = PreparedCandidate::from_bytes(&expected, root.path(), "app.py").unwrap();
        assert_ne!(
            prepared.candidate_snapshot_digest(),
            prepared.source_digest()
        );
    }
}
#[test]
fn dirty_source_or_changed_configuration_cannot_prepare() {
    for target in ["app.py", "pyproject.toml"] {
        let (root, expected) = fixture("sha1");
        std::fs::write(root.path().join(target), "changed\n").unwrap();
        assert!(PreparedCandidate::from_bytes(&expected, root.path(), "app.py").is_err());
    }
}
#[test]
#[ignore = "requires explicit controller-pinned native CodeGuard and official Ruff executables"]
fn actual_native_capture_binds_both_oid_formats_and_rejects_queue_reuse() {
    use codeguard_cli::guard_integration::projection::ProtectedMapping;
    use codeguard_gg_candidate_host::ProducerPin;
    use sha2::{Digest, Sha256};
    let executable = std::path::PathBuf::from(
        std::env::var_os("CODEGUARD_CANDIDATE_PRODUCER").expect("explicit producer"),
    );
    let ruff_executable = std::path::PathBuf::from(
        std::env::var_os("CODEGUARD_CANDIDATE_RUFF").expect("explicit Ruff"),
    );
    let sha256 =
        std::env::var("CODEGUARD_CANDIDATE_PRODUCER_SHA256").expect("controller producer pin");
    let pin = ProducerPin {
        executable,
        sha256,
        ruff_executable,
    };
    let mapping=ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"f401"},{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}"#).unwrap();
    for format in ["sha1", "sha256"] {
        let (root, expected) = fixture(format);
        let captured = PreparedCandidate::from_bytes(&expected, root.path(), "app.py")
            .unwrap()
            .capture(&pin, &std::sync::atomic::AtomicBool::new(false))
            .unwrap();
        assert_eq!(captured.native_exit(), 3);
        for mode in ["enforce", "review"] {
            let contract:guardengine::GuardContract=serde_json::from_value(serde_json::json!({"apiVersion":"guard.partme.ai/v1alpha1","kind":"GuardContract","metadata":{"id":"ruff","revision":"1"},"spec":{"rules":[{"id":"f401","enforcement":mode,"assertion":{"type":"forbid_relation","subject":"python","predicate":"has","object":"unused-import"}}]}})).unwrap();
            let projected = captured.project(&expected, &mapping, &contract).unwrap();
            assert_eq!(projected.domain_bytes(), captured.raw_bytes());
            assert_eq!(projected.envelope().artifacts.domain.len(), 2);
            assert_eq!(
                projected.envelope().decision,
                Some(guardengine::Decision::Allow)
            );
            let source_hash = format!("sha256:{:x}", Sha256::digest(b"print('clean')\n"));
            assert_eq!(
                projected.envelope().binding.source_snapshot_digest,
                source_hash
            );
            assert_ne!(
                projected.provenance()["candidate"]["source_snapshot_digest"]
                    .as_str()
                    .unwrap(),
                source_hash.trim_start_matches("sha256:")
            );
            guardengine::integration::verify_engine_artifacts(
                projected.envelope(),
                projected.contract_bytes().unwrap(),
                projected.facts_bytes().unwrap(),
                projected.report_bytes().unwrap(),
            )
            .unwrap();
            for key in [
                "repo_id",
                "task_id",
                "worktree_id",
                "base_oid",
                "candidate_oid",
                "merge_group_id",
                "requirement_ids",
            ] {
                let mut changed: serde_json::Value = serde_json::from_slice(&expected).unwrap();
                changed[key] = if key == "requirement_ids" {
                    serde_json::json!(["another"])
                } else {
                    serde_json::json!("another")
                };
                assert!(
                    captured
                        .project(&serde_json::to_vec(&changed).unwrap(), &mapping, &contract)
                        .is_err(),
                    "{key}"
                );
            }
        }
    }
}
fn pin() -> codeguard_gg_candidate_host::ProducerPin {
    codeguard_gg_candidate_host::ProducerPin {
        executable: std::env::var_os("CODEGUARD_CANDIDATE_PRODUCER")
            .expect("producer")
            .into(),
        sha256: std::env::var("CODEGUARD_CANDIDATE_PRODUCER_SHA256").expect("pin"),
        ruff_executable: std::env::var_os("CODEGUARD_CANDIDATE_RUFF")
            .expect("Ruff")
            .into(),
    }
}
fn mapping() -> codeguard_cli::guard_integration::projection::ProtectedMapping {
    codeguard_cli::guard_integration::projection::ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"f401"},{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}"#).unwrap()
}
fn contract(mode: &str) -> guardengine::GuardContract {
    serde_json::from_value(serde_json::json!({"apiVersion":"guard.partme.ai/v1alpha1","kind":"GuardContract","metadata":{"id":"ruff","revision":"1"},"spec":{"rules":[{"id":"f401","enforcement":mode,"assertion":{"type":"forbid_relation","subject":"python","predicate":"has","object":"unused-import"}}]}})).unwrap()
}
fn snapshot(root: &Path, oid: &str, base: &str, members: Vec<String>) -> Vec<u8> {
    let repo = Repository::discover(root, "repo").unwrap();
    let subject = repo
        .resolve_subject(SubjectRequest::Commit(oid.into()))
        .unwrap();
    let scope = TaskScope::advisory(
        "task",
        vec!["R".into()],
        vec![b"app.py".to_vec(), b"pyproject.toml".to_vec()],
        &"a".repeat(64),
        None,
    )
    .unwrap();
    serde_json::to_vec(
        &repo
            .prepare_candidate(
                &subject,
                &scope,
                &CandidateRequest {
                    worktree_id: "worktree".into(),
                    base_oid: base.into(),
                    merge_group_id: if members.is_empty() {
                        None
                    } else {
                        Some("queue-A".into())
                    },
                    members,
                },
            )
            .unwrap(),
    )
    .unwrap()
}
fn state(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn walk(
        root: &Path,
        base: &Path,
        out: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, base, out)
            } else {
                out.insert(
                    path.strip_prefix(base).unwrap().into(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn export(
    name: &str,
    capture: &codeguard_gg_candidate_host::CandidateCapture,
    expected: &[u8],
    projection: &codeguard_gg_candidate_host::CandidateProjection,
    root: &Path,
) {
    if let Some(dir) = std::env::var_os("CODEGUARD_CANDIDATE_CAPTURE_DIR") {
        let dir = std::path::PathBuf::from(dir).join(name);
        std::fs::create_dir(&dir).unwrap();
        for (name, bytes) in [
            ("expected-candidate.json", expected),
            ("native.json", capture.raw_bytes()),
            ("candidate.json", projection.provenance_bytes()),
            ("app.py", capture.source_bytes()),
            (
                "pyproject.toml",
                codeguard_cli::guard_integration::ruff_profile::CONFIG.as_bytes(),
            ),
            ("contract.json", projection.contract_bytes().unwrap()),
            ("facts.json", projection.facts_bytes().unwrap()),
            ("report.json", projection.report_bytes().unwrap()),
        ] {
            std::fs::write(dir.join(name), bytes).unwrap();
        }
        std::fs::write(
            dir.join("envelope.json"),
            serde_json::to_vec(projection.envelope()).unwrap(),
        )
        .unwrap();
        git(
            root,
            &[
                "bundle",
                "create",
                dir.join("source.bundle").to_str().unwrap(),
                "--all",
            ],
        );
    }
}
#[test]
#[ignore = "requires explicit trusted local native tool pins"]
fn actual_queue_m_not_pr_head_and_changed_full_identity_rejects_old_evidence() {
    use guardengine::Decision;
    let pin = pin();
    let map = mapping();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for format in ["sha1", "sha256"] {
        let (root, _) = fixture(format);
        let base = git(root.path(), &["rev-parse", "HEAD"]);
        std::fs::write(root.path().join("app.py"), "import os\nprint('bad')\n").unwrap();
        git(root.path(), &["add", "app.py"]);
        git(root.path(), &["commit", "-qm", "PR head"]);
        let pr = git(root.path(), &["rev-parse", "HEAD"]);
        let old = snapshot(root.path(), &pr, &base, vec![]);
        let before = state(root.path());
        let captured = PreparedCandidate::from_bytes(&old, root.path(), "app.py")
            .unwrap()
            .capture(&pin, &cancel)
            .unwrap();
        assert_eq!(before, state(root.path()));
        for (mode, expected) in [
            ("enforce", Decision::Block),
            ("review", Decision::RequireApproval),
        ] {
            let output = captured.project(&old, &map, &contract(mode)).unwrap();
            assert_eq!(output.envelope().decision, Some(expected));
            export(
                &format!("{format}-bad-{mode}"),
                &captured,
                &old,
                &output,
                root.path(),
            );
        }
        std::fs::write(root.path().join("app.py"), "print('clean')\n").unwrap();
        git(root.path(), &["add", "app.py"]);
        git(root.path(), &["commit", "-qm", "synthetic M"]);
        let m = git(root.path(), &["rev-parse", "HEAD"]);
        git(root.path(), &["branch", "queue-M", &m]);
        git(root.path(), &["update-ref", "HEAD", &pr]);
        assert_ne!(git(root.path(), &["rev-parse", "HEAD"]), m);
        let queue = snapshot(root.path(), &m, &base, vec![base.clone(), pr.clone()]);
        assert!(
            captured
                .project(&queue, &map, &contract("enforce"))
                .is_err()
        );
        let before = state(root.path());
        let queue_capture = PreparedCandidate::from_bytes(&queue, root.path(), "app.py")
            .unwrap()
            .capture(&pin, &cancel)
            .unwrap();
        let output = queue_capture
            .project(&queue, &map, &contract("enforce"))
            .unwrap();
        assert_eq!(output.envelope().decision, Some(Decision::Allow));
        assert_eq!(output.envelope().binding.candidate_oid, m);
        assert_eq!(before, state(root.path()));
        export(
            &format!("{format}-queue-clean"),
            &queue_capture,
            &queue,
            &output,
            root.path(),
        );
        for key in [
            "members",
            "base_oid",
            "merge_group_id",
            "source_snapshot_digest",
            "allowed_paths",
            "policy_digest",
        ] {
            let mut changed: serde_json::Value = serde_json::from_slice(&queue).unwrap();
            match key {
                "members" => changed[key].as_array_mut().unwrap().reverse(),
                "base_oid" => changed[key] = serde_json::json!(pr),
                "merge_group_id" => changed[key] = serde_json::json!("queue-B"),
                "allowed_paths" => changed[key] = serde_json::json!([[97]]),
                _ => changed[key] = serde_json::json!("b".repeat(64)),
            }
            assert!(
                queue_capture
                    .project(
                        &serde_json::to_vec(&changed).unwrap(),
                        &map,
                        &contract("enforce")
                    )
                    .is_err(),
                "{key}"
            );
        }
        std::fs::write(root.path().join("app.py"), "dirty after capture\n").unwrap();
        assert!(
            queue_capture
                .project(&queue, &map, &contract("enforce"))
                .is_err()
        );
    }
}
#[test]
fn committed_wrong_config_staged_dirty_and_missing_object_are_rejected() {
    let (root, expected) = fixture("sha1");
    let original = std::fs::read(root.path().join("app.py")).unwrap();
    std::fs::write(root.path().join("app.py"), "staged\n").unwrap();
    git(root.path(), &["add", "app.py"]);
    std::fs::write(root.path().join("app.py"), &original).unwrap();
    assert!(PreparedCandidate::from_bytes(&expected, root.path(), "app.py").is_err());
    git(root.path(), &["add", "app.py"]);
    let mut missing: serde_json::Value = serde_json::from_slice(&expected).unwrap();
    missing["candidate_oid"] = serde_json::json!("f".repeat(40));
    assert!(
        PreparedCandidate::from_bytes(
            &serde_json::to_vec(&missing).unwrap(),
            root.path(),
            "app.py"
        )
        .is_err()
    );
    std::fs::write(
        root.path().join("pyproject.toml"),
        "[tool.ruff.lint]\nignore=['F401']\n",
    )
    .unwrap();
    git(root.path(), &["add", "pyproject.toml"]);
    git(root.path(), &["commit", "-qm", "disabled"]);
    let oid = git(root.path(), &["rev-parse", "HEAD"]);
    let changed = snapshot(root.path(), &oid, &oid, vec![]);
    assert!(PreparedCandidate::from_bytes(&changed, root.path(), "app.py").is_err());
}
#[test]
fn wrong_repository_and_source_budget_fail_without_native_execution() {
    let (root, expected) = fixture("sha256");
    let (other, _) = fixture("sha1");
    assert!(PreparedCandidate::from_bytes(&expected, other.path(), "app.py").is_err());
    std::fs::write(root.path().join("app.py"), vec![b' '; 1_048_577]).unwrap();
    git(root.path(), &["add", "app.py"]);
    git(root.path(), &["commit", "-qm", "oversized"]);
    let oid = git(root.path(), &["rev-parse", "HEAD"]);
    let changed = snapshot(root.path(), &oid, &oid, vec![]);
    assert!(PreparedCandidate::from_bytes(&changed, root.path(), "app.py").is_err());
    assert!(PreparedCandidate::from_bytes(&vec![b' '; 1_048_577], root.path(), "app.py").is_err());
}
#[test]
fn controller_binary_pin_mismatch_refuses_capture() {
    let (root, expected) = fixture("sha1");
    let candidate = PreparedCandidate::from_bytes(&expected, root.path(), "app.py").unwrap();
    let pin = codeguard_gg_candidate_host::ProducerPin {
        executable: "/usr/bin/true".into(),
        sha256: "0".repeat(64),
        ruff_executable: "/usr/bin/true".into(),
    };
    assert!(
        candidate
            .capture(&pin, &std::sync::atomic::AtomicBool::new(false))
            .is_err()
    );
}

#[test]
fn typed_prebinding_transport_rejects_all_four_failure_classes_without_envelopes() {
    use codeguard_gg_candidate_host::transport::{PrebindingCode, prepare};
    fn rejected(
        result: Result<
            PreparedCandidate,
            codeguard_gg_candidate_host::transport::PrebindingDiagnostic,
        >,
        code: PrebindingCode,
    ) {
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("unexpected prepared candidate"),
        };
        assert_eq!(error.code(), code);
        let wire: serde_json::Value = serde_json::from_slice(&error.to_json()).unwrap();
        assert_eq!(
            wire,
            serde_json::json!({"apiVersion":"codeguard.transport/v1alpha1","kind":"GuardTransportDiagnostic","phase":"unbound","code":code.as_str(),"exitCode":4})
        );
        assert!(wire.get("envelope").is_none());
        assert!(wire.get("decision").is_none());
        if let Some(dir) = std::env::var_os("CODEGUARD_TRANSPORT_CAPTURE_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{}.json", code.as_str())),
                error.to_json(),
            )
            .unwrap();
        }
    }
    for format in ["sha1", "sha256"] {
        let (root, snapshot) = fixture(format);
        assert!(prepare(&snapshot, root.path(), "app.py").is_ok());
        rejected(
            prepare(&snapshot, root.path(), "../app.py"),
            PrebindingCode::BadParameters,
        );
        rejected(
            prepare(b"{", root.path(), "app.py"),
            PrebindingCode::BadParameters,
        );
        rejected(
            prepare(
                &snapshot,
                &root.path().join("missing-nonempty-repository"),
                "app.py",
            ),
            PrebindingCode::UnknownRepository,
        );
        let nonrepo = tempfile::tempdir().unwrap();
        rejected(
            prepare(&snapshot, nonrepo.path(), "app.py"),
            PrebindingCode::UnknownRepository,
        );
        for field in ["candidate_oid", "base_oid"] {
            for value in [None, Some(serde_json::json!(""))] {
                let mut doc: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
                if let Some(value) = value {
                    doc[field] = value;
                } else {
                    doc.as_object_mut().unwrap().remove(field);
                }
                rejected(
                    prepare(&serde_json::to_vec(&doc).unwrap(), root.path(), "app.py"),
                    PrebindingCode::MissingCandidateOrBase,
                );
            }
        }
        for field in ["requirement_ids", "allowed_paths", "policy_digest"] {
            let mut doc: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
            doc[field] = if field == "policy_digest" {
                serde_json::json!("")
            } else {
                serde_json::json!([])
            };
            rejected(
                prepare(&serde_json::to_vec(&doc).unwrap(), root.path(), "app.py"),
                PrebindingCode::UnfrozenScope,
            );
        }
        std::fs::write(root.path().join("app.py"), "dirty\n").unwrap();
        rejected(
            prepare(&snapshot, root.path(), "app.py"),
            PrebindingCode::CandidateBindingInvalid,
        );
    }
}

#[test]
fn typed_transport_rejects_raw_budget_depth_and_duplicate_ambiguity() {
    use codeguard_gg_candidate_host::transport::{PrebindingCode, prepare};
    let (root, snapshot) = fixture("sha1");
    let text = String::from_utf8(snapshot).unwrap();
    let duplicate = text
        .replacen('{', "{\"candidate_oid\":\"\",", 1)
        .into_bytes();
    let deeply_nested = format!("{}0{}", "[".repeat(200), "]".repeat(200)).into_bytes();
    for bytes in [vec![b' '; 1_048_577], duplicate, deeply_nested] {
        let Err(error) = prepare(&bytes, root.path(), "app.py") else {
            panic!("invalid input accepted")
        };
        assert_eq!(error.code(), PrebindingCode::BadParameters);
        assert!(
            !String::from_utf8(error.to_json())
                .unwrap()
                .contains(&root.path().to_string_lossy().to_string())
        );
    }
}
