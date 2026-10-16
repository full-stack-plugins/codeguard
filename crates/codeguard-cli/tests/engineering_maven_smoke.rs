#![cfg(unix)]
//! 同一次真实Maven执行的产品报告、私有双流与源码绑定；无SDK/CLI安装。
use codeguard_cli::{
    check_command::run_observed,
    engineering_projection::{
        ProjectionCheckerPin, ProjectionContext, ProjectionNativeRule, ProjectionRun,
        ProjectionSourceProof, project_raw,
    },
    maven_native_capture::MavenNativeCapture,
    tool_identity::hash_bundle_tree,
};
use codeguard_core::{GateImpact, NativeCheckerBinding};
use guard_protocol::{
    Classification, Digest, DomainOwner, EvidenceRef, Execution, ExecutionStatus, Identifier,
    ProtocolDocument, Retention, RunSequence, UtcTimestamp,
};
use sha2::{Digest as Hasher, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, atomic::AtomicBool},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}
fn digest(bytes: &[u8]) -> Digest {
    Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).unwrap()
}
fn private(root: &Path, name: &str, bytes: &[u8]) -> EvidenceRef {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(root.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
    EvidenceRef {
        digest: digest(bytes),
        media_type: id("application/octet-stream"),
        size: bytes.len() as u64,
        classification: Classification::Private,
        retention: Retention {
            policy_id: id("local-smoke-private"),
            retain_until: UtcTimestamp::new("2099-01-01T00:00:00Z").unwrap(),
        },
    }
}
use std::os::unix::fs::OpenOptionsExt;
fn timestamp(value: SystemTime) -> UtcTimestamp {
    let d = value.duration_since(UNIX_EPOCH).unwrap();
    let output = Command::new("date")
        .args([
            "-u",
            "-d",
            &format!("@{}.{:09}", d.as_secs(), d.subsec_nanos()),
            "+%Y-%m-%dT%H:%M:%S.%NZ",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    UtcTimestamp::new(String::from_utf8(output.stdout).unwrap().trim()).unwrap()
}
fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}
fn fixture_document(bytes: &[u8]) -> ProtocolDocument {
    guard_protocol::decode(bytes).unwrap().document().clone()
}

#[test]
#[ignore = "显式已有Maven/JDK21，隔离依赖缓存；CI独立调用，不静默跳过缺工具"]
fn real_maven_same_execution_projects_product_feedback_with_captured_sources_and_logs() {
    for reject_sink in [false, true] {
        let maven = PathBuf::from(
            std::env::var_os("CODEGUARD_MAVEN_BIN").expect("existing Maven required"),
        )
        .canonicalize()
        .unwrap();
        let java = PathBuf::from(
            std::env::var_os("CODEGUARD_TEST_JAVA_HOME").expect("existing JDK21 required"),
        )
        .canonicalize()
        .unwrap();
        let repo = PathBuf::from(
            std::env::var_os("CODEGUARD_JAVADOC_MAVEN_REPO")
                .expect("explicit isolated Maven artifact cache required"),
        );
        let test_root = std::env::temp_dir().join(format!(
            "cg-engineering-real-maven-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&test_root).unwrap();
        fs::set_permissions(&test_root, fs::Permissions::from_mode(0o700)).unwrap();
        let root = test_root.join("workspace");
        let evidence = test_root.join("evidence");
        fs::create_dir(&evidence).unwrap();
        fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        let source = b"/** */\npublic class Sample {}\n";
        let pom=b"<project><modelVersion>4.0.0</modelVersion><groupId>smoke</groupId><artifactId>demo</artifactId><version>1</version><properties><maven.compiler.source>17</maven.compiler.source><maven.compiler.target>17</maven.compiler.target></properties><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint><failOnWarnings>false</failOnWarnings></configuration></plugin></plugins></build></project>";
        fs::write(root.join("pom.xml"), pom).unwrap();
        fs::write(root.join("src/main/java/Sample.java"), source).unwrap();
        fs::write(root.join(".gitignore"), b"target/\n.codeguard/\n").unwrap();
        fs::create_dir_all(&repo).unwrap();
        if !reject_sink
            && std::env::var("CODEGUARD_ENGINEERING_MAVEN_SMOKE_WARMUP").as_deref() == Ok("1")
        {
            // 只准备隔离插件依赖缓存；本次准备输出从不作为后续CLI运行的证据。
            let settings = root.join("settings.xml");
            fs::write(
                &settings,
                include_bytes!("../resources/isolated_maven_settings.xml"),
            )
            .unwrap();
            let spec = codeguard_runtime::ProcessSpec {
                executable: maven.clone(),
                args: vec![
                    "-B".into(),
                    "-ntp".into(),
                    "-s".into(),
                    settings.clone().into_os_string(),
                    "-gs".into(),
                    settings.clone().into_os_string(),
                    format!("-Dmaven.repo.local={}", repo.display()).into(),
                    "org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc".into(),
                ],
                cwd: root.clone(),
                env: BTreeMap::from([
                    ("JAVA_HOME".into(), java.clone().into_os_string()),
                    ("MAVEN_SKIP_RC".into(), "1".into()),
                    (
                        "PATH".into(),
                        format!("{}:/usr/bin:/bin", java.join("bin").display()).into(),
                    ),
                ]),
                stdin: None,
                deadline: Instant::now() + Duration::from_secs(180),
                output_limit_bytes: 2 * 1024 * 1024,
            };
            let out = codeguard_runtime::run_process_recorded(
                &spec,
                &AtomicBool::new(false),
                &evidence,
                "dependency-cache-preparation.log",
            )
            .unwrap();
            assert_eq!(
                out.termination,
                codeguard_runtime::Termination::Exited(0),
                "cache preparation failed; private log retained"
            );
            fs::remove_file(settings).unwrap();
        }
        git(&root, &["init", "-q"]);
        git(
            &root,
            &[
                "-c",
                "user.name=smoke",
                "-c",
                "user.email=smoke@example.invalid",
                "add",
                ".",
            ],
        );
        git(
            &root,
            &[
                "-c",
                "user.name=smoke",
                "-c",
                "user.email=smoke@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        );
        let ProtocolDocument::VerificationPlan(mut plan) = fixture_document(include_bytes!(
            "../../../../guardengine/crates/guard-protocol/fixtures/valid/verification-plan.json"
        )) else {
            panic!()
        };
        let ProtocolDocument::TaskBinding(mut task) = fixture_document(include_bytes!(
            "../../../../guardengine/crates/guard-protocol/fixtures/valid/task-binding.json"
        )) else {
            panic!()
        };
        let ProtocolDocument::RuleDefinition(mut rule) = fixture_document(include_bytes!(
            "../../../../guardengine/crates/guard-protocol/fixtures/valid/rule-definition.json"
        )) else {
            panic!()
        };
        let ProtocolDocument::ProducerIdentity(producer) = fixture_document(include_bytes!(
            "../../../../guardengine/crates/guard-protocol/fixtures/valid/producer-identity.json"
        )) else {
            panic!()
        };
        rule.owner = DomainOwner::Codeguard;
        let path = guard_protocol::GitPath::from_bytes(b"src/main/java/Sample.java").unwrap();
        plan.obligations[0].targets[0].path = path.clone();
        plan.obligations[0].targets[0].input_digest = digest(source);
        plan.targets = plan.obligations[0].targets.clone();
        task.scope.paths = vec![path];
        let binding = guard_subject::CaptureBinding {
            repository_id: plan.targets[0].repository_id.clone(),
            task_id: task.task_id.clone(),
            worktree_id: task.worktree_id.clone(),
            contract_digest: plan.contract_digest.clone(),
            policy_digest: plan.policy_digest.clone(),
            verifier_digest: digest(b"local-smoke-javadoc-profile"),
            environment_digest: digest(&fs::read(java.join("release")).unwrap()),
        };
        let head = String::from_utf8(git(&root, &["rev-parse", "HEAD"])).unwrap();
        let selection = guard_subject::Selection::Worktree {
            base: guard_protocol::GitOid::new(head.trim()).unwrap(),
        };
        let subject = guard_subject::capture(&root, &binding, &selection).unwrap();
        plan.subject_digest = subject.identity_digest.clone();
        let captured = Mutex::new(Vec::<MavenNativeCapture>::new());
        let callback = |capture: &MavenNativeCapture| {
            captured
                .lock()
                .map_err(|_| "capture slot poisoned".to_string())?
                .push(capture.clone());
            if reject_sink {
                Err("intentional observer sink failure".into())
            } else {
                Ok(())
            }
        };
        let report_path = evidence.join("product-feedback.json");
        let args = vec![
            "java".into(),
            root.to_string_lossy().into_owned(),
            "--maven-tool".into(),
            maven.to_string_lossy().into_owned(),
            "--java-home".into(),
            java.to_string_lossy().into_owned(),
            "--maven-repo".into(),
            repo.to_string_lossy().into_owned(),
            "--repo-sha256".into(),
            hash_bundle_tree(&repo).unwrap(),
            "--format=json".into(),
            "--output".into(),
            report_path.to_string_lossy().into_owned(),
            "--timeout".into(),
            "180s".into(),
            "--jobs".into(),
            "1".into(),
        ];
        let outcome = run_observed(&args, &callback);
        assert_eq!(outcome.exit_code, std::process::ExitCode::from(3));
        if reject_sink {
            assert_eq!(
                outcome.capture_errors,
                vec!["intentional observer sink failure"]
            );
        } else {
            assert!(outcome.capture_errors.is_empty());
        }
        guard_subject::verify_current(&root, &binding, &selection, &subject).unwrap();
        let captured = captured.into_inner().unwrap();
        assert_eq!(
            captured.len(),
            1,
            "same actual Maven invocation must be captured exactly once"
        );
        let capture = &captured[0];
        assert_eq!(
            capture.outcome.termination,
            codeguard_runtime::Termination::Exited(0)
        );
        assert!(capture.outcome.stderr.is_empty());
        assert_eq!(capture.sources["src/main/java/Sample.java"].1, source);
        let raw = fs::read(report_path).unwrap();
        let document: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(
            document["native_results"]["java_javadoc"]["maven_multifile_probes"][0]["observation"],
            capture.native_report
        );
        assert!(
            !capture.native_report["findings"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let run = ProjectionRun {
            run_id: id("actual-smoke-run"),
            operation_id: id("actual-smoke-operation"),
            execution: Execution {
                status: ExecutionStatus::Completed,
                exit_code: Some(0),
                started_at: Some(timestamp(capture.dispatch_requested_at)),
                finished_at: Some(timestamp(capture.process_observed_finished_at)),
                sequence: RunSequence::new("1").unwrap(),
                lineage_id: id("actual-smoke-lineage"),
            },
        };
        let pin = ProjectionCheckerPin {
            verifier_id: plan.obligations[0].verifier_id.clone(),
            binding: NativeCheckerBinding {
                checker_id: "java.maven.javadoc".into(),
                tool_id: "maven-javadoc".into(),
                category: "comments".into(),
                obligation_id: plan.obligations[0].obligation_id.as_str().into(),
            },
            version: id("existing-distribution-artifact-pinned"),
            artifact_digest: digest(&fs::read(&maven).unwrap()),
            native_rules: ["JavadocEmptyComment", "JavadocMissingComment"]
                .iter()
                .map(|r| ProjectionNativeRule {
                    native_rule_id: id(r),
                    gate_impact: GateImpact::Blocking,
                })
                .collect(),
            feedback_target: None,
        };
        let raw_ref = private(&evidence, "original-report-copy.json", &raw);
        let mut ctx =
            ProjectionContext::new(&plan, &task, &[rule], &producer, &run, &raw_ref, &[pin])
                .unwrap();
        let proofs = capture
            .sources
            .iter()
            .map(|(path, (native, bytes))| {
                assert_eq!(path, "src/main/java/Sample.java");
                ProjectionSourceProof::new(
                    &plan,
                    &run,
                    &plan.targets[0],
                    native,
                    bytes,
                    &private(&evidence, "captured-source.java", bytes),
                )
                .unwrap()
            })
            .collect();
        let stdout_ref = private(&evidence, "native-stdout.log", &capture.outcome.stdout);
        private(&evidence, "native-stderr.log", &capture.outcome.stderr);
        ctx = ctx
            .with_java_execution(
                id("java.maven.javadoc"),
                &capture.outcome.stdout,
                &stdout_ref,
                0,
                id("3.12.0"),
                proofs,
            )
            .unwrap();
        let projected = project_raw(&raw, Some(&ctx));
        assert_eq!(projected.original_bytes(), raw);
        assert!(
            projected.unbound_diagnostics().is_empty(),
            "{:?}",
            projected.issues()
        );
        assert!(!projected.findings().is_empty(), "{:?}", projected.issues());
        assert_eq!(
            projected.results()[0].assessment.verdict,
            guard_protocol::Verdict::Violation
        );
        assert_eq!(
            projected.results()[0].assessment.coverage,
            guard_protocol::Coverage::Partial
        );
        assert_eq!(projected.trust_level(), "UNVERIFIED");
        assert!(!matches!(
            projected.gate().decision,
            codeguard_core::DeliveryDecision::Allow
                | codeguard_core::DeliveryDecision::AllowWithExceptions
        ));
        // 原件只存在0700私有父目录；保留供CI失败复核，不打印任何原生日志。
        eprintln!(
            "same-run Maven evidence: {} confirmed finding(s); UNVERIFIED/Partial",
            projected.findings().len()
        );
    }
}
