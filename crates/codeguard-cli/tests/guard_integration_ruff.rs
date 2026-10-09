use codeguard_cli::guard_integration::ruff_profile::{RuffF401Policy, read_ruff_feedback};
use guardengine::Completeness;
use serde_json::Value;
use std::fs;
fn fixture(name: &str) -> Vec<u8> {
    fs::read(format!(
        "{}/../../tests/fixtures/guard-integration/ruff-f401/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
fn read(name: &str) -> codeguard_cli::guard_integration::ruff_profile::RuffEvidence {
    let manifest: Value = serde_json::from_slice(&fixture("capture")).unwrap();
    let bytes = fixture(name);
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let producer = manifest["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        manifest["cases"][name]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    read_ruff_feedback(
        &bytes,
        &policy,
        document["run_id"].as_str().unwrap(),
        3,
        producer,
    )
    .unwrap()
}
#[test]
fn genuine_bad_fixed_and_clean_have_only_narrow_f401_coverage() {
    for (name, count) in [("bad", 1), ("fixed", 0), ("clean", 0)] {
        let evidence = read(name);
        assert_eq!(
            evidence.completeness(),
            Completeness::Complete,
            "{name}: {:?}",
            evidence.gaps()
        );
        assert_eq!(evidence.finding_count(), count);
        assert_eq!(evidence.raw_bytes(), fixture(name));
        assert_eq!(evidence.native_exit(), 3);
    }
}
#[test]
fn genuine_faults_and_coverage_evasion_remain_partial() {
    for name in [
        "missing-tool",
        "disabled-rule",
        "noqa",
        "per-file-ignore",
        "source-changed",
        "tool-error",
    ] {
        let evidence = read(name);
        assert_eq!(evidence.completeness(), Completeness::Partial, "{name}");
        assert!(!evidence.gaps().is_empty());
        assert_eq!(evidence.raw_bytes(), fixture(name));
        assert_eq!(evidence.native_exit(), 3);
    }
}
#[test]
fn feedback_reader_rejects_unknown_duplicate_truncated_or_foreign_invocation() {
    let raw = fixture("clean");
    let mut doc: Value = serde_json::from_slice(&raw).unwrap();
    let manifest: Value = serde_json::from_slice(&fixture("capture")).unwrap();
    let producer = manifest["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        manifest["cases"]["clean"]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let run = doc["run_id"].as_str().unwrap().to_owned();
    for path in [vec!["unknown"], vec!["files", "unknown"]] {
        let mut changed = doc.clone();
        if path.len() == 1 {
            changed["unknown"] = true.into();
        } else {
            changed["files"][0]["unknown"] = true.into();
        }
        assert!(
            read_ruff_feedback(
                &serde_json::to_vec(&changed).unwrap(),
                &policy,
                &run,
                3,
                producer
            )
            .is_err()
        );
    }
    doc["schema_version"] = "0.14.0".into();
    assert!(
        read_ruff_feedback(
            &serde_json::to_vec(&doc).unwrap(),
            &policy,
            &run,
            3,
            producer
        )
        .is_err()
    );
    assert!(read_ruff_feedback(&raw[..raw.len() / 2], &policy, &run, 3, producer).is_err());
    assert!(read_ruff_feedback(&raw, &policy, "other-run", 3, producer).is_err());
    assert!(read_ruff_feedback(&raw, &policy, &run, 0, producer).is_err());
    let duplicate = String::from_utf8(raw.clone())
        .unwrap()
        .replacen('{', "{\"exit_code\":3,", 1);
    assert!(read_ruff_feedback(duplicate.as_bytes(), &policy, &run, 3, producer).is_err());
    assert!(read_ruff_feedback(&vec![b' '; 1024 * 1024 + 1], &policy, &run, 3, producer).is_err());
}

#[test]
fn real_narrow_observations_project_without_rewriting_native_three() {
    use codeguard_cli::guard_integration::{
        projection::ProtectedMapping, ruff_profile::project_ruff,
    };
    use guardengine::{GuardContract, GuardSubject};
    for (name, mode, decision) in [
        ("clean", "enforce", "ALLOW"),
        ("bad", "enforce", "BLOCK"),
        ("bad", "review", "REQUIRE_APPROVAL"),
        ("missing-tool", "review", "BLOCK"),
    ] {
        let evidence = read(name);
        let manifest: Value = serde_json::from_slice(&fixture("capture")).unwrap();
        let mapping=ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"f401"},{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}"#).unwrap();
        let contract:GuardContract=serde_json::from_value(serde_json::json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"ruff-f401","revision":"1"},"spec":{"rules":[{"id":"f401","enforcement":mode,"assertion":{"type":"forbid_relation","subject":"python","predicate":"has","object":"unused-import"}}]}})).unwrap();
        let projected = project_ruff(
            &evidence,
            &mapping,
            &contract,
            GuardSubject {
                id: "repo".into(),
                snapshot_digest: format!(
                    "sha256:{}",
                    manifest["cases"][name]["sourceSha256"].as_str().unwrap()
                ),
            },
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&projected.report().decision).unwrap(),
            decision
        );
        assert_eq!(projected.domain_bytes(), fixture(name));
        assert_eq!(
            serde_json::from_slice::<Value>(projected.domain_bytes()).unwrap()["exit_code"],
            3
        );
    }
}

#[test]
fn complete_narrow_scope_is_bound_and_independently_recomputed() {
    use codeguard_cli::guard_integration::{
        envelope::FrozenRun, projection::ProtectedMapping, ruff_profile::project_ruff,
    };
    use guardengine::{
        GuardContract, GuardSubject,
        integration::{CoverageStatus, RunBinding, verify_engine_artifacts},
    };
    let name = "clean";
    let evidence = read(name);
    let manifest: Value = serde_json::from_slice(&fixture("capture")).unwrap();
    let producer = manifest["codeguardSha256"].as_str().unwrap();
    let source = manifest["cases"][name]["sourceSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new("app.py", source, producer).unwrap();
    let document: Value = serde_json::from_slice(evidence.raw_bytes()).unwrap();
    let binding = RunBinding {
        repo_id: "repo".into(),
        task_id: "task".into(),
        worktree_id: "worktree".into(),
        requirement_ids: vec!["ruff-f401".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: policy.source_digest(),
        baseline_digest: None,
    };
    let frozen = FrozenRun::new(
        document["run_id"].as_str().unwrap(),
        binding,
        &policy.obligations(),
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap();
    let mapping=ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"f401"},{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}"#).unwrap();
    let contract:GuardContract=serde_json::from_value(serde_json::json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"ruff-f401","revision":"1"},"spec":{"rules":[{"id":"f401","enforcement":"enforce","assertion":{"type":"forbid_relation","subject":"python","predicate":"has","object":"unused-import"}}]}})).unwrap();
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
    let output = frozen.complete(projection).unwrap();
    assert_eq!(output.envelope().coverage.status, CoverageStatus::Complete);
    assert_eq!(output.domain_bytes(), evidence.raw_bytes());
    verify_engine_artifacts(
        output.envelope(),
        output.contract_bytes().unwrap(),
        output.facts_bytes().unwrap(),
        output.report_bytes().unwrap(),
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
#[ignore = "requires official Ruff0.16.8 executable via CODEGUARD_RUFF_F401_TOOL"]
fn real_native_ruff_profile_controls() {
    use codeguard_cli::guard_integration::ruff_profile::{CONFIG, TOOL_SHA256};
    use sha2::{Digest, Sha256};
    use std::process::Command;
    let tool = std::env::var("CODEGUARD_RUFF_F401_TOOL").expect("provide existing pinned Ruff");
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&tool).unwrap())),
        TOOL_SHA256
    );
    let binary = env!("CARGO_BIN_EXE_codeguard");
    let producer = format!("{:x}", Sha256::digest(fs::read(binary).unwrap()));
    let root = std::env::temp_dir().join(format!("cg-ruff-profile-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let capture = std::env::var_os("CODEGUARD_RUFF_CAPTURE_DIR").map(std::path::PathBuf::from);
    if let Some(path) = &capture {
        fs::create_dir(path).expect("explicit capture directory must be new");
    }
    let cases = [
        ("bad", "import os\n", CONFIG.to_owned(), true, 1),
        ("fixed", "print('fixed')\n", CONFIG.to_owned(), true, 0),
        (
            "clean",
            "import os\nprint(os.name)\n",
            CONFIG.to_owned(),
            true,
            0,
        ),
        ("missing-tool", "import os\n", CONFIG.to_owned(), false, 0),
        (
            "disabled-rule",
            "import os\n",
            CONFIG.replace("F401", "E501"),
            false,
            0,
        ),
        (
            "noqa",
            "import os # noqa: F401\n",
            CONFIG.to_owned(),
            false,
            0,
        ),
        (
            "per-file-ignore",
            "import os\n",
            format!("{CONFIG}[tool.ruff.lint.per-file-ignores]\n\"app.py\" = [\"F401\"]\n"),
            false,
            0,
        ),
        (
            "source-changed",
            "print('changed')\n",
            CONFIG.to_owned(),
            false,
            0,
        ),
        (
            "tool-error",
            "import os\n",
            "[tool.ruff]\ninvalid_key = true\n".into(),
            false,
            0,
        ),
    ];
    let mut manifest = serde_json::Map::new();
    for (name, source, config, complete, findings) in cases {
        let dir = root.join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("app.py"), source).unwrap();
        fs::write(dir.join("pyproject.toml"), &config).unwrap();
        let frozen_source = if name == "source-changed" {
            "import os\n"
        } else {
            source
        };
        let source_hash = format!("{:x}", Sha256::digest(frozen_source.as_bytes()));
        let policy = RuffF401Policy::new("app.py", &source_hash, &producer).unwrap();
        let selected_tool = if name == "missing-tool" {
            dir.join("missing-ruff").to_string_lossy().into_owned()
        } else {
            tool.clone()
        };
        let args = vec![
            "lint".to_owned(),
            "python".into(),
            dir.to_string_lossy().into_owned(),
            "--format=json".into(),
            "--file".into(),
            "app.py".into(),
            "--ruff-tool".into(),
            selected_tool.clone(),
        ];
        let native = Command::new(binary).args(&args).output().unwrap();
        assert_eq!(native.status.code(), Some(3), "{name}");
        let document: Value = serde_json::from_slice(&native.stdout).unwrap();
        let run = document["run_id"].as_str().unwrap();
        let evidence = read_ruff_feedback(&native.stdout, &policy, run, 3, &producer).unwrap();
        assert_eq!(
            evidence.completeness() == Completeness::Complete,
            complete,
            "{name}: {:?}",
            evidence.gaps()
        );
        assert_eq!(evidence.finding_count(), findings, "{name}");
        let raw = Command::new(&selected_tool)
            .args(["check", "--no-cache", "--output-format", "json", "app.py"])
            .current_dir(&dir)
            .output();
        let raw_exit = raw.as_ref().ok().and_then(|out| out.status.code());
        if let Some(path) = &capture {
            let outdir = path.join(name);
            fs::create_dir(&outdir).unwrap();
            fs::write(outdir.join("feedback.json"), &native.stdout).unwrap();
            fs::write(outdir.join("stderr.txt"), &native.stderr).unwrap();
            for mode in ["enforce", "review"] {
                export_projection(&outdir.join(mode), &evidence, &policy, run, mode);
            }
            fs::write(outdir.join("app.py"), source).unwrap();
            fs::write(outdir.join("pyproject.toml"), &config).unwrap();
            if let Ok(raw) = raw {
                fs::write(outdir.join("ruff-stdout.json"), raw.stdout).unwrap();
                fs::write(outdir.join("ruff-stderr.txt"), raw.stderr).unwrap();
            }
        }
        manifest.insert(name.into(),serde_json::json!({"executable":binary,"argv":args,"ruffArgv":[selected_tool,"check","--no-cache","--output-format","json","app.py"],"codeguardExit":3,"ruffExit":raw_exit,"codeguardSha256":producer,"toolSha256":TOOL_SHA256,"frozenSourceSha256":source_hash,"feedbackSha256":format!("{:x}",Sha256::digest(&native.stdout)),"narrowComplete":complete,"findings":findings}));
    }
    if let Some(path) = capture {
        fs::write(
            path.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
fn export_projection(
    dir: &std::path::Path,
    evidence: &codeguard_cli::guard_integration::ruff_profile::RuffEvidence,
    policy: &RuffF401Policy,
    run: &str,
    mode: &str,
) {
    use codeguard_cli::guard_integration::{
        envelope::FrozenRun, projection::ProtectedMapping, ruff_profile::project_ruff,
    };
    use guardengine::{GuardContract, GuardSubject, integration::RunBinding};
    let mapping=ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"f401"},{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}"#).unwrap();
    let contract:GuardContract=serde_json::from_value(serde_json::json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"ruff-f401","revision":"1"},"spec":{"rules":[{"id":"f401","enforcement":mode,"assertion":{"type":"forbid_relation","subject":"python","predicate":"has","object":"unused-import"}}]}})).unwrap();
    let binding = RunBinding {
        repo_id: "repo".into(),
        task_id: "task".into(),
        worktree_id: "worktree".into(),
        requirement_ids: vec!["ruff-f401".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: policy.source_digest(),
        baseline_digest: None,
    };
    let frozen = FrozenRun::new(
        run,
        binding,
        &policy.obligations(),
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap();
    let projection = project_ruff(
        evidence,
        &mapping,
        &contract,
        GuardSubject {
            id: "repo".into(),
            snapshot_digest: policy.source_digest(),
        },
    )
    .unwrap();
    let output = frozen.complete(projection).unwrap();
    fs::create_dir(dir).unwrap();
    fs::write(
        dir.join("envelope.json"),
        serde_json::to_vec_pretty(output.envelope()).unwrap(),
    )
    .unwrap();
    fs::write(dir.join("contract.json"), output.contract_bytes().unwrap()).unwrap();
    fs::write(dir.join("facts.json"), output.facts_bytes().unwrap()).unwrap();
    fs::write(dir.join("report.json"), output.report_bytes().unwrap()).unwrap();
    fs::write(dir.join("domain.json"), output.domain_bytes()).unwrap();
}

#[test]
fn narrow_profile_rejects_broader_contracts_and_unmapped_clean_coverage() {
    use codeguard_cli::guard_integration::{
        projection::ProtectedMapping, ruff_profile::project_ruff,
    };
    use guardengine::{GuardContract, GuardSubject};
    let evidence = read("clean");
    let manifest: Value = serde_json::from_slice(&fixture("capture")).unwrap();
    let subject = || GuardSubject {
        id: "repo".into(),
        snapshot_digest: format!(
            "sha256:{}",
            manifest["cases"]["clean"]["sourceSha256"].as_str().unwrap()
        ),
    };
    let mapping=ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"f401"},{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}"#).unwrap();
    let mut contract_json = serde_json::json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"ruff-f401","revision":"1"},"spec":{"rules":[{"id":"f401","enforcement":"enforce","assertion":{"type":"forbid_relation","subject":"python","predicate":"has","object":"unused-import"}}]}});
    let mut other = contract_json["spec"]["rules"][0].clone();
    other["id"] = "security".into();
    contract_json["spec"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(other);
    let broader: GuardContract = serde_json::from_value(contract_json.clone()).unwrap();
    assert!(project_ruff(&evidence, &mapping, &broader, subject()).is_err());
    contract_json["spec"]["rules"].as_array_mut().unwrap().pop();
    let contract: GuardContract = serde_json::from_value(contract_json).unwrap();
    let missing=ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"gap","detail":"ruff_f401_scope_incomplete"},"rule_id":"f401"}]}"#).unwrap();
    assert!(project_ruff(&evidence, &missing, &contract, subject()).is_err());
    let mut wrong = subject();
    wrong.snapshot_digest = format!("sha256:{}", "e".repeat(64));
    assert!(project_ruff(&evidence, &mapping, &contract, wrong).is_err());
}

#[test]
fn observed_identity_changes_and_missing_settings_cannot_complete() {
    let raw = fixture("clean");
    let doc: Value = serde_json::from_slice(&raw).unwrap();
    let manifest: Value = serde_json::from_slice(&fixture("capture")).unwrap();
    let producer = manifest["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        manifest["cases"]["clean"]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let run = doc["run_id"].as_str().unwrap();
    assert!(read_ruff_feedback(&raw, &policy, run, 3, &"e".repeat(64)).is_err());
    for field in ["tool_sha256", "config_sha256", "source_sha256"] {
        let mut changed = doc.clone();
        changed["files"][0][field] = "d".repeat(64).into();
        let evidence = read_ruff_feedback(
            &serde_json::to_vec(&changed).unwrap(),
            &policy,
            run,
            3,
            producer,
        )
        .unwrap();
        assert_eq!(evidence.completeness(), Completeness::Partial, "{field}");
    }
    for field in ["rule_settings", "suppression_audit"] {
        let mut changed = doc.clone();
        changed["files"][0][field] = Value::Null;
        let evidence = read_ruff_feedback(
            &serde_json::to_vec(&changed).unwrap(),
            &policy,
            run,
            3,
            producer,
        )
        .unwrap();
        assert_eq!(evidence.completeness(), Completeness::Partial, "{field}");
    }
}
