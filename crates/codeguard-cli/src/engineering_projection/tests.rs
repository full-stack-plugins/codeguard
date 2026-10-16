use super::project_raw;
use sha2::Digest as Hasher;
#[test]
fn unknown_version_and_duplicate_keys_are_incomplete_and_original_bytes_survive() {
    for raw in [
        br#"{ "report_type":"run_report", "schema_version":"9.0" }"#.as_slice(),
        br#"{"report_type":"run_report","schema_version":"1.4","schema_version":"1.0"}"#.as_slice(),
    ] {
        let p = project_raw(raw, None);
        assert_eq!(p.original_bytes(), raw);
        assert_eq!(p.trust_level(), "UNVERIFIED");
        assert_eq!(p.status(), "incomplete");
        assert_eq!(
            p.gate().decision,
            codeguard_core::DeliveryDecision::Incomplete
        );
        assert!(p.results().is_empty());
    }
}

#[test]
fn real_historical_feedback_has_a_valid_closed_profile_but_never_becomes_allow() {
    let raw = include_bytes!(
        "../../../../tests/acceptance/evidence/cfquery-comment-boundary-2026-10-05.json"
    );
    let value: serde_json::Value = serde_json::from_slice(raw).unwrap();
    super::feedback_schema::validate(&value, "0.38.0").unwrap();
    let p = project_raw(raw, None);
    assert_eq!(p.original_bytes(), raw);
    assert_eq!(p.trust_level(), "UNVERIFIED");
    assert!(
        p.issues()
            .iter()
            .any(|s| s.contains("lacks_frozen_obligation_evidence")),
        "{:?}",
        p.issues()
    );
    assert!(!matches!(
        p.gate().decision,
        codeguard_core::DeliveryDecision::Allow
            | codeguard_core::DeliveryDecision::AllowWithExceptions
    ));
    let mut bad = value;
    bad["authority"] = serde_json::json!("trusted");
    let encoded = serde_json::to_vec(&bad).unwrap();
    let p = project_raw(&encoded, None);
    assert_eq!(p.status(), "incomplete");
    assert!(p.results().is_empty());
}
#[test]
fn latest_historical_feedback_profiles_are_validated_without_file_or_network_resolution() {
    let archive: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../tests/acceptance/evidence/check-c-family-documentation-default.json"
    ))
    .unwrap();
    for name in ["first", "missing_tool", "deadline"] {
        let report = &archive[name];
        assert_eq!(report["schema_version"], "0.72.0");
        super::feedback_schema::validate(report, "0.72.0").unwrap();
        let encoded = serde_json::to_vec(report).unwrap();
        let p = project_raw(&encoded, None);
        assert!(
            p.issues()
                .iter()
                .any(|s| s.contains("lacks_frozen_obligation_evidence")),
            "{:?}",
            p.issues()
        );
    }
}
fn legacy_report() -> serde_json::Value {
    serde_json::json!({"schema_version":"1.0","report_type":"run_report","operation":"lint","request_id":"req-1","run_id":"native-1","command_status":"incomplete","exit_code":3,
 "request":{"schema_version":"1.0","report_type":"check_request","request_id":"req-1","command":"lint","selection":{"language":"python"},"root":"/workspace/example","content_source":{"kind":"working_tree"},"options":{"format":"json","offline":false,"timeout_ms":30000,"jobs":2}},
 "plan_id":"plan-1","identities":{"content":{"kind":"working_tree","digest":"a".repeat(64)},"policy":{"source":"trusted-ci","revision":"r1","digest":"b".repeat(64)},"tools":[{"id":"ruff","version":"0.16.8","binary_sha256":"c".repeat(64)}]},
 "results":[{"obligation_id":"OB-1","completion":"incomplete","reason":"native_timeout","findings":[{"id":"f-1","native_rule_id":"F401","tool_id":"ruff","severity":"UNKNOWN","gate_impact":"blocking","message":"retained native diagnostic","obligation_id":"OB-1","evidence_refs":["run:native-1:ruff"]}],"coverage":{"expected":["src/Example.java"],"observed":["src/Example.java"],"unresolved":[],"proof_kind":"native_report"},"execution_refs":["run:native-1:ruff"]}],
 "delivery_gate":{"decision":"not_evaluated","eligible":false,"blocking_finding_ids":["f-1"],"incomplete_obligation_ids":["OB-1"],"coverage_mismatch_ids":[],"invalid_ledger_ids":[]},"warnings":[],"next_actions":["fix and rerun"]})
}
fn context(raw: &[u8]) -> super::ProjectionContext {
    use guard_protocol::{ProtocolDocument, decode};
    fn read(name: &str) -> ProtocolDocument {
        let raw=match name {
    "verification-plan.json"=>include_bytes!("../../../../../guardengine/crates/guard-protocol/fixtures/valid/verification-plan.json").as_slice(),
    "task-binding.json"=>include_bytes!("../../../../../guardengine/crates/guard-protocol/fixtures/valid/task-binding.json").as_slice(),
    "rule-definition.json"=>include_bytes!("../../../../../guardengine/crates/guard-protocol/fixtures/valid/rule-definition.json").as_slice(),
    "producer-identity.json"=>include_bytes!("../../../../../guardengine/crates/guard-protocol/fixtures/valid/producer-identity.json").as_slice(),
    _=>panic!("unknown fixture")
  };
        decode(raw).unwrap().document().clone()
    }
    let ProtocolDocument::VerificationPlan(plan) = read("verification-plan.json") else {
        panic!()
    };
    let ProtocolDocument::TaskBinding(task) = read("task-binding.json") else {
        panic!()
    };
    let ProtocolDocument::RuleDefinition(mut rule) = read("rule-definition.json") else {
        panic!()
    };
    rule.owner = guard_protocol::DomainOwner::Codeguard;
    let ProtocolDocument::ProducerIdentity(producer) = read("producer-identity.json") else {
        panic!()
    };
    let execution = guard_protocol::Execution {
        status: guard_protocol::ExecutionStatus::TimedOut,
        exit_code: None,
        started_at: Some(guard_protocol::UtcTimestamp::new("2026-10-17T00:00:00Z").unwrap()),
        finished_at: Some(guard_protocol::UtcTimestamp::new("2026-10-17T00:00:01Z").unwrap()),
        sequence: guard_protocol::RunSequence::new("1").unwrap(),
        lineage_id: guard_protocol::Identifier::new("lineage-1").unwrap(),
    };
    use sha2::{Digest, Sha256};
    let evidence = guard_protocol::EvidenceRef {
        digest: guard_protocol::Digest::new(format!("sha256:{:x}", Sha256::digest(raw))).unwrap(),
        media_type: guard_protocol::Identifier::new("application/json").unwrap(),
        size: raw.len() as u64,
        classification: guard_protocol::Classification::Private,
        retention: guard_protocol::Retention {
            policy_id: guard_protocol::Identifier::new("private-raw").unwrap(),
            retain_until: guard_protocol::UtcTimestamp::new("2027-01-01T00:00:00Z").unwrap(),
        },
    };
    super::ProjectionContext::new(
        &plan,
        &task,
        &[rule],
        &producer,
        &super::ProjectionRun {
            run_id: guard_protocol::Identifier::new("run-1").unwrap(),
            operation_id: guard_protocol::Identifier::new("op-1").unwrap(),
            execution,
        },
        &evidence,
        &[super::ProjectionCheckerPin {
            verifier_id: plan.obligations[0].verifier_id.clone(),
            binding: codeguard_core::NativeCheckerBinding {
                checker_id: "python.ruff".into(),
                tool_id: "ruff".into(),
                category: "lint".into(),
                obligation_id: "OB-1".into(),
            },
            version: guard_protocol::Identifier::new("0.16.8").unwrap(),
            artifact_digest: guard_protocol::Digest::new(format!("sha256:{}", "c".repeat(64)))
                .unwrap(),
            native_rules: vec![super::ProjectionNativeRule {
                native_rule_id: guard_protocol::Identifier::new("F401").unwrap(),
                gate_impact: codeguard_core::GateImpact::Blocking,
            }],
            feedback_target: None,
        }],
    )
    .unwrap()
}
#[test]
fn partial_known_violation_is_retained_and_native_unknown_severity_never_guessed() {
    let raw = serde_json::to_vec(&legacy_report()).unwrap();
    let ctx = context(&raw);
    let p = project_raw(&raw, Some(&ctx));
    assert_eq!(p.findings().len(), 1);
    assert_eq!(p.findings()[0].severity, "UNKNOWN");
    assert_eq!(p.results().len(), 1, "{:?}", p.issues());
    let r = &p.results()[0];
    assert_eq!(r.assessment.verdict, guard_protocol::Verdict::Violation);
    assert_eq!(r.assessment.coverage, guard_protocol::Coverage::Partial);
    assert_eq!(
        r.execution.status,
        guard_protocol::ExecutionStatus::TimedOut
    );
    assert_eq!(r.severity, guard_protocol::Severity::High);
    assert_eq!(
        p.gate().decision,
        codeguard_core::DeliveryDecision::NotEvaluated
    );
    assert_eq!(p.trust_level(), "UNVERIFIED");
    assert_eq!(p.original_bytes(), raw);
}
#[test]
fn noncanonical_minor_missing_context_and_source_mutation_cannot_project_pass() {
    let mut value = legacy_report();
    value["schema_version"] = serde_json::json!("1.04");
    let encoded = serde_json::to_vec(&value).unwrap();
    let p = project_raw(&encoded, None);
    assert!(p.issues().iter().any(|s| s.contains("unknown exact")));
    let raw = serde_json::to_vec(&legacy_report()).unwrap();
    let p = project_raw(&raw, None);
    assert!(p.results().is_empty());
    assert_eq!(p.status(), "incomplete");
    let ctx = context(&raw);
    let mut changed = legacy_report();
    changed["warnings"] = serde_json::json!(["changed"]);
    let encoded = serde_json::to_vec(&changed).unwrap();
    let p = project_raw(&encoded, Some(&ctx));
    assert!(p.results().is_empty());
    assert!(
        p.issues()
            .iter()
            .any(|s| s.contains("digest/size mismatch"))
    );
}

#[test]
fn missing_module_is_kept_and_actual_execution_metadata_cannot_be_invented() {
    let raw = serde_json::to_vec(&legacy_report()).unwrap();
    let original = context(&raw);
    let mut plan = original.plan.clone();
    let mut missing = plan.obligations[0].clone();
    missing.obligation_id = guard_protocol::Identifier::new("OB-2").unwrap();
    plan.obligations.push(missing);
    let mut pins = original.checker_pins.clone();
    let mut second = pins[0].clone();
    second.binding.obligation_id = "OB-2".into();
    pins.push(second);
    let ctx = super::ProjectionContext::new(
        &plan,
        &original.task,
        &original.rules,
        &original.producer,
        &original.run,
        &original.raw_evidence,
        &pins,
    )
    .unwrap();
    let projected = project_raw(&raw, Some(&ctx));
    assert_eq!(projected.results().len(), 2, "{:?}", projected.issues());
    assert_eq!(
        projected.results()[1].assessment.verdict,
        guard_protocol::Verdict::Unknown
    );
    assert_eq!(
        projected.results()[1].execution.status,
        guard_protocol::ExecutionStatus::NotRun
    );
    assert!(
        projected
            .gate()
            .incomplete_obligation_ids
            .iter()
            .any(|id| id == "OB-2")
    );
    let mut run = original.run.clone();
    run.execution.status = guard_protocol::ExecutionStatus::Completed;
    run.execution.started_at = None;
    run.execution.finished_at = None;
    run.execution.exit_code = Some(0);
    assert!(
        super::ProjectionContext::new(
            &original.plan,
            &original.task,
            &original.rules,
            &original.producer,
            &run,
            &original.raw_evidence,
            &original.checker_pins
        )
        .is_err()
    );
}

#[test]
fn historical_provider_with_native_findings_survives_dual_read_without_zero_findings_inference() {
    let archive: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../tests/acceptance/evidence/cargo-audit-path-discovery-2026-10-06.json"
    ))
    .unwrap();
    let source = &archive["reports"][0];
    assert_eq!(source["schema_version"], "0.56.0");
    super::feedback_schema::validate(source, "0.56.0").unwrap();
    let raw = serde_json::to_vec(source).unwrap();
    let projected = project_raw(&raw, None);
    assert_eq!(projected.original_bytes(), raw);
    let retained: serde_json::Value = serde_json::from_slice(projected.original_bytes()).unwrap();
    assert_eq!(retained["native_results"], source["native_results"]);
    assert_eq!(projected.status(), "incomplete");
    assert_eq!(projected.trust_level(), "UNVERIFIED");
    assert!(projected.results().is_empty());
    // 没有规范义务映射不能把空public结果当成零违规；原生诊断留在私有原件。
    assert!(
        projected
            .issues()
            .iter()
            .any(|s| s.contains("lacks_frozen_obligation_evidence"))
    );
}

#[test]
fn even_a_structurally_valid_old_full_allow_cannot_enable_trusted_binding() {
    let mut value = legacy_report();
    value["operation"] = serde_json::json!("check");
    value["request"]["command"] = serde_json::json!("check");
    value["request"]["selection"]["language"] = serde_json::json!("all");
    value["command_status"] = serde_json::json!("complete");
    value["exit_code"] = serde_json::json!(0);
    value["results"][0]["completion"] = serde_json::json!("complete");
    value["results"][0]
        .as_object_mut()
        .unwrap()
        .remove("reason");
    value["results"][0]["findings"] = serde_json::json!([]);
    value["delivery_gate"]["decision"] = serde_json::json!("allow");
    value["delivery_gate"]["eligible"] = serde_json::json!(true);
    value["delivery_gate"]["blocking_finding_ids"] = serde_json::json!([]);
    value["delivery_gate"]["incomplete_obligation_ids"] = serde_json::json!([]);
    let raw = serde_json::to_vec(&value).unwrap();
    crate::run_report::parse_run_report(&raw).expect("valid legacy full allow declaration");
    let projected = project_raw(&raw, None);
    assert_eq!(
        projected.gate().decision,
        codeguard_core::DeliveryDecision::Incomplete
    );
    assert_eq!(projected.trust_level(), "UNVERIFIED");
    assert!(projected.results().is_empty());
    assert_eq!(projected.original_bytes(), raw);
}

#[test]
fn wrong_native_tool_pin_is_not_relabelled_as_an_approved_verifier() {
    let mut report = legacy_report();
    report["identities"]["tools"][0]["binary_sha256"] = serde_json::json!("d".repeat(64));
    let raw = serde_json::to_vec(&report).unwrap();
    let ctx = context(&raw);
    let p = project_raw(&raw, Some(&ctx));
    assert!(p.results().is_empty());
    assert!(
        p.issues()
            .iter()
            .any(|s| s.contains("differs from independent pin"))
    );
    assert_eq!(p.trust_level(), "UNVERIFIED");
    assert_eq!(p.findings().len(), 1);
}

fn actual_clippy_feedback() -> serde_json::Value {
    let archive: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../tests/acceptance/evidence/clippy-documentation-aggregate-native-wasm.json"
    ))
    .unwrap();
    archive["cases"][0]["aggregate"].clone()
}
fn clippy_context(raw: &[u8]) -> super::ProjectionContext {
    let base = context(raw);
    let mut plan = base.plan.clone();
    let path = guard_protocol::GitPath::from_bytes(b"src/lib.rs").unwrap();
    let source = guard_protocol::Digest::new(
        "sha256:c553c13ed9a28a0112c4ea2d6df17670d2afe53256592b237ca0fdc2fa1e0d3d",
    )
    .unwrap();
    plan.obligations[0].targets[0].path = path.clone();
    plan.obligations[0].targets[0].input_digest = source;
    plan.targets = plan.obligations[0].targets.clone();
    let mut task = base.task.clone();
    task.scope.paths = vec![path];
    let mut pin = base.checker_pins[0].clone();
    pin.binding.checker_id = "rust.cargo_clippy".into();
    pin.binding.tool_id = "cargo-clippy".into();
    pin.version = guard_protocol::Identifier::new("1.90.0").unwrap();
    pin.native_rules = vec![super::ProjectionNativeRule {
        native_rule_id: guard_protocol::Identifier::new("clippy::missing_errors_doc").unwrap(),
        gate_impact: codeguard_core::GateImpact::Blocking,
    }];
    pin.artifact_digest = guard_protocol::Digest::new(
        "sha256:9dada6167fb0bc26e44e23f09179ea15b4d9a8ec491fef1ac5a16484699540ac",
    )
    .unwrap();
    super::ProjectionContext::new(
        &plan,
        &task,
        &base.rules,
        &base.producer,
        &base.run,
        &base.raw_evidence,
        &[pin],
    )
    .unwrap()
}
#[test]
fn actual_feedback_confirmed_clippy_violation_is_typed_without_inventing_complete_coverage() {
    let source = actual_clippy_feedback();
    let raw = serde_json::to_vec(&source).unwrap();
    let ctx = clippy_context(&raw);
    let projected = project_raw(&raw, Some(&ctx));
    assert_eq!(
        projected.findings().len(),
        1,
        "actual native_results.rust_lint finding lost: {:?}",
        projected.issues()
    );
    assert_eq!(
        projected.findings()[0].native_rule_id,
        "clippy::missing_errors_doc"
    );
    assert_eq!(projected.findings()[0].severity, "warning");
    assert_eq!(
        projected.results()[0].assessment.verdict,
        guard_protocol::Verdict::Violation
    );
    assert_eq!(
        projected.results()[0].assessment.coverage,
        guard_protocol::Coverage::Partial
    );
    assert_eq!(projected.trust_level(), "UNVERIFIED");
    assert_eq!(projected.status(), "incomplete");
}

fn with_feedback_pin(
    ctx: &super::ProjectionContext,
    pin: super::ProjectionCheckerPin,
) -> super::ProjectionContext {
    super::ProjectionContext::new(
        &ctx.plan,
        &ctx.task,
        &ctx.rules,
        &ctx.producer,
        &ctx.run,
        &ctx.raw_evidence,
        &[pin],
    )
    .unwrap()
}

#[test]
fn actual_feedback_missing_rule_mapping_wrong_checker_or_target_never_implies_clean() {
    let raw = serde_json::to_vec(&actual_clippy_feedback()).unwrap();
    let ctx = clippy_context(&raw);
    for scenario in 0..4 {
        let mut pin = ctx.checker_pins[0].clone();
        let altered = match scenario {
            0 => {
                pin.native_rules.clear();
                with_feedback_pin(&ctx, pin)
            }
            1 => {
                pin.artifact_digest =
                    guard_protocol::Digest::new(format!("sha256:{}", "f".repeat(64))).unwrap();
                with_feedback_pin(&ctx, pin)
            }
            2 => {
                let mut plan = ctx.plan.clone();
                plan.obligations[0].targets[0].input_digest =
                    guard_protocol::Digest::new(format!("sha256:{}", "f".repeat(64))).unwrap();
                plan.targets = plan.obligations[0].targets.clone();
                super::ProjectionContext::new(
                    &plan,
                    &ctx.task,
                    &ctx.rules,
                    &ctx.producer,
                    &ctx.run,
                    &ctx.raw_evidence,
                    &[pin],
                )
                .unwrap()
            }
            _ => {
                pin.binding.checker_id = "rust.cargo_audit".into();
                with_feedback_pin(&ctx, pin)
            }
        };
        let p = project_raw(&raw, Some(&altered));
        assert!(p.findings().is_empty(), "scenario {scenario}");
        assert_eq!(
            p.results()[0].assessment.verdict,
            guard_protocol::Verdict::Unknown
        );
        assert_eq!(
            p.results()[0].assessment.coverage,
            guard_protocol::Coverage::Unknown
        );
        assert_eq!(p.status(), "incomplete");
        assert_eq!(p.original_bytes(), raw);
        let expected = [
            "approved rule mapping missing",
            "artifact/authority mismatch",
            "source digest differs",
            "artifact/authority mismatch",
        ][scenario];
        assert!(
            p.issues().iter().any(|s| s.contains(expected)),
            "{:?}",
            p.issues()
        );
    }
}

#[test]
fn actual_advisory_unknown_severity_requires_explicit_lock_target_and_retains_native_value() {
    let archive: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../tests/acceptance/evidence/cargo-audit-path-discovery-2026-10-06.json"
    ))
    .unwrap();
    let raw = serde_json::to_vec(&archive["reports"][0]).unwrap();
    let base = context(&raw);
    let mut plan = base.plan.clone();
    let path = guard_protocol::GitPath::from_bytes(b"Cargo.lock").unwrap();
    plan.obligations[0].targets[0].path = path.clone();
    plan.obligations[0].targets[0].input_digest = guard_protocol::Digest::new(
        "sha256:d6535c7338c1e7344b96a040b6e7f8d580dda98a06d03fe4f359877a7e2c6bf7",
    )
    .unwrap();
    plan.targets = plan.obligations[0].targets.clone();
    let mut task = base.task.clone();
    task.scope.paths = vec![path.clone()];
    let mut pin = base.checker_pins[0].clone();
    pin.binding.checker_id = "rust.cargo_audit".into();
    pin.binding.tool_id = "cargo-audit".into();
    pin.artifact_digest = guard_protocol::Digest::new(
        "sha256:9ae8935b86669f69d13525d16371d3f706772cc3e4d254d47ac32ddefc8ee53e",
    )
    .unwrap();
    pin.native_rules = vec![super::ProjectionNativeRule {
        native_rule_id: guard_protocol::Identifier::new("RUSTSEC-2020-0071").unwrap(),
        gate_impact: codeguard_core::GateImpact::Blocking,
    }];
    for explicit in [false, true] {
        pin.feedback_target = explicit.then(|| path.clone());
        let ctx = super::ProjectionContext::new(
            &plan,
            &task,
            &base.rules,
            &base.producer,
            &base.run,
            &base.raw_evidence,
            &[pin.clone()],
        )
        .unwrap();
        let p = project_raw(&raw, Some(&ctx));
        if explicit {
            assert_eq!(p.findings().len(), 1, "{:?}", p.issues());
            assert_eq!(p.findings()[0].severity, "unverified");
            assert_eq!(
                p.results()[0].assessment.verdict,
                guard_protocol::Verdict::Violation
            );
            assert_eq!(
                p.results()[0].assessment.coverage,
                guard_protocol::Coverage::Partial
            );
        } else {
            assert!(p.findings().is_empty());
            assert_eq!(
                p.results()[0].assessment.verdict,
                guard_protocol::Verdict::Unknown
            );
            assert!(
                p.issues()
                    .iter()
                    .any(|s| s.contains("explicit frozen feedback_target required"))
            );
        }
        assert_eq!(p.trust_level(), "UNVERIFIED");
        assert_eq!(p.status(), "incomplete");
        assert_eq!(p.original_bytes(), raw);
    }
}

#[test]
fn actual_feedback_rustdoc_confirmed_diagnostic_uses_only_approved_rule_mapping() {
    let raw = serde_json::to_vec(&actual_clippy_feedback()).unwrap();
    let base = clippy_context(&raw);
    let mut pin = base.checker_pins[0].clone();
    pin.binding.checker_id = "rust.cargo_rustdoc".into();
    pin.binding.tool_id = "rustdoc".into();
    pin.native_rules[0].native_rule_id = guard_protocol::Identifier::new("missing_docs").unwrap();
    let ctx = with_feedback_pin(&base, pin);
    let p = project_raw(&raw, Some(&ctx));
    assert_eq!(p.findings().len(), 1, "{:?}", p.issues());
    assert_eq!(p.findings()[0].native_rule_id, "missing_docs");
    assert_eq!(
        p.results()[0].assessment.verdict,
        guard_protocol::Verdict::Violation
    );
    assert_eq!(
        p.results()[0].assessment.coverage,
        guard_protocol::Coverage::Partial
    );
    assert_eq!(p.status(), "incomplete");
    assert_eq!(p.trust_level(), "UNVERIFIED");
}

#[test]
fn actual_feedback_partial_scan_retains_already_bound_positive_diagnostic() {
    let mut report = actual_clippy_feedback();
    report["native_results"]["rust_lint"]["local_scan_complete"] = serde_json::json!(false);
    super::feedback_schema::validate(&report, "0.56.0").unwrap();
    let raw = serde_json::to_vec(&report).unwrap();
    let ctx = clippy_context(&raw);
    let p = project_raw(&raw, Some(&ctx));
    assert_eq!(
        p.findings().len(),
        1,
        "partial execution erased positive diagnostic: {:?}",
        p.issues()
    );
    assert_eq!(
        p.results()[0].assessment.verdict,
        guard_protocol::Verdict::Violation
    );
    assert_eq!(
        p.results()[0].assessment.coverage,
        guard_protocol::Coverage::Partial
    );
    assert_eq!(p.results()[0].execution.status, ctx.run.execution.status);
    assert_eq!(p.trust_level(), "UNVERIFIED");
    assert_eq!(p.status(), "incomplete");
}

fn actual_maven_javadoc_feedback() -> serde_json::Value {
    let archive:serde_json::Value=serde_json::from_slice(include_bytes!("../../../../tests/acceptance/evidence/maven-javadoc-detailed-descriptions-fixtures-2026-10-06.json")).unwrap();
    archive[0]["aggregate"].clone()
}
#[test]
fn actual_maven_diagnostic_without_independent_source_proof_is_typed_unbound() {
    let raw = serde_json::to_vec(&actual_maven_javadoc_feedback()).unwrap();
    let base = clippy_context(&raw);
    let mut pin = base.checker_pins[0].clone();
    pin.binding.checker_id = "java.maven.javadoc".into();
    pin.binding.tool_id = "maven-javadoc".into();
    pin.native_rules[0].native_rule_id =
        guard_protocol::Identifier::new("JavadocEmptyComment").unwrap();
    let ctx = with_feedback_pin(&base, pin);
    let p = project_raw(&raw, Some(&ctx));
    assert_eq!(p.unbound_diagnostics().len(), 1, "{:?}", p.issues());
    assert_eq!(
        p.unbound_diagnostics()[0].native_rule_id,
        "JavadocEmptyComment"
    );
    assert!(p.findings().is_empty());
    assert_eq!(
        p.results()[0].assessment.verdict,
        guard_protocol::Verdict::Unknown
    );
}

fn private_ref(base: &super::ProjectionContext, bytes: &[u8]) -> guard_protocol::EvidenceRef {
    let mut evidence = base.raw_evidence.clone();
    evidence.digest =
        guard_protocol::Digest::new(format!("sha256:{:x}", sha2::Sha256::digest(bytes))).unwrap();
    evidence.size = bytes.len() as u64;
    evidence
}
#[test]
fn controlled_maven_parser_source_binding_keeps_partial_violation_and_rejects_other_run() {
    // 受控新输入测试：仅复用实际报告格式，不冒称历史报告的源码或日志已被捕获。
    let raw = serde_json::to_vec(&actual_maven_javadoc_feedback()).unwrap();
    let base = context(&raw);
    let source = b"public class Sample {}\n";
    let path = guard_protocol::GitPath::from_bytes(b"src/main/java/Sample.java").unwrap();
    let mut plan = base.plan.clone();
    plan.obligations[0].targets[0].path = path.clone();
    plan.obligations[0].targets[0].input_digest = private_ref(&base, source).digest;
    plan.targets = plan.obligations[0].targets.clone();
    let mut task = base.task.clone();
    task.scope.paths = vec![path];
    let mut pin = base.checker_pins[0].clone();
    pin.binding.checker_id = "java.maven.javadoc".into();
    pin.binding.tool_id = "maven-javadoc".into();
    pin.native_rules[0].native_rule_id =
        guard_protocol::Identifier::new("JavadocEmptyComment").unwrap();
    pin.artifact_digest = guard_protocol::Digest::new(
        "sha256:5de154f4a84239e4021e4ed6937ad4f41d9db01350572beb58bcbf99abd422b7",
    )
    .unwrap();
    let ctx = super::ProjectionContext::new(
        &plan,
        &task,
        &base.rules,
        &base.producer,
        &base.run,
        &base.raw_evidence,
        &[pin],
    )
    .unwrap();
    let log=b"[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\n[WARNING] Javadoc Warnings\n[WARNING] /workspace/src/main/java/Sample.java:1: warning: empty comment\n[WARNING] public class Sample {}\n[WARNING] ^\n[WARNING] 1 warning\n[INFO] BUILD SUCCESS\n";
    let evidence = private_ref(&ctx, source);
    let proof = super::ProjectionSourceProof::new(
        &plan,
        &ctx.run,
        &plan.targets[0],
        "/workspace/src/main/java/Sample.java",
        source,
        &evidence,
    )
    .unwrap();
    let bound = ctx
        .clone()
        .with_java_execution(
            guard_protocol::Identifier::new("java.maven.javadoc").unwrap(),
            log,
            &private_ref(&ctx, log),
            0,
            guard_protocol::Identifier::new("3.12.0").unwrap(),
            vec![proof.clone()],
        )
        .unwrap();
    let p = project_raw(&raw, Some(&bound));
    assert_eq!(p.findings().len(), 1, "{:?}", p.issues());
    assert!(p.unbound_diagnostics().is_empty());
    assert_eq!(p.findings()[0].severity, "UNKNOWN");
    assert_eq!(
        p.results()[0].assessment.verdict,
        guard_protocol::Verdict::Violation
    );
    assert_eq!(
        p.results()[0].assessment.coverage,
        guard_protocol::Coverage::Partial
    );
    assert_eq!(p.results()[0].evidence_refs.len(), 3);
    let mut other_run = ctx.run.clone();
    other_run.run_id = guard_protocol::Identifier::new("other-run").unwrap();
    let wrong = super::ProjectionSourceProof::new(
        &plan,
        &other_run,
        &plan.targets[0],
        "/workspace/src/main/java/Sample.java",
        source,
        &evidence,
    )
    .unwrap();
    assert!(
        ctx.clone()
            .with_java_execution(
                guard_protocol::Identifier::new("java.maven.javadoc").unwrap(),
                log,
                &private_ref(&ctx, log),
                0,
                guard_protocol::Identifier::new("3.12.0").unwrap(),
                vec![wrong]
            )
            .is_err()
    );
    assert!(
        super::ProjectionSourceProof::new(
            &plan,
            &ctx.run,
            &plan.targets[0],
            "/workspace/src/main/java/Sample.java",
            b"changed",
            &evidence
        )
        .is_err()
    );
    let bad_log = log
        .iter()
        .copied()
        .map(|b| if b == b'S' { b'X' } else { b })
        .collect::<Vec<_>>();
    let mismatch = ctx
        .clone()
        .with_java_execution(
            guard_protocol::Identifier::new("java.maven.javadoc").unwrap(),
            &bad_log,
            &private_ref(&ctx, &bad_log),
            0,
            guard_protocol::Identifier::new("3.12.0").unwrap(),
            vec![proof],
        )
        .unwrap();
    let p = project_raw(&raw, Some(&mismatch));
    assert!(p.findings().is_empty());
    assert_eq!(p.unbound_diagnostics().len(), 1);
    assert_eq!(
        p.results()[0].assessment.verdict,
        guard_protocol::Verdict::Unknown
    );
}

#[test]
fn controlled_p3c_reuses_native_xml_catalog_and_source_proof_with_failed_execution() {
    // 专业映射单测的受控PMD输入，不宣称当前Maven/P3C实际执行或真实历史阳性。
    let mut report = actual_maven_javadoc_feedback();
    let bytes = serde_json::to_vec(&report).unwrap();
    let base = context(&bytes);
    let source = b"public class Sample {}\n";
    let path = guard_protocol::GitPath::from_bytes(b"src/main/java/Sample.java").unwrap();
    let source_ref = private_ref(&base, source);
    let mut plan = base.plan.clone();
    plan.obligations[0].targets[0].path = path.clone();
    plan.obligations[0].targets[0].input_digest = source_ref.digest.clone();
    plan.targets = plan.obligations[0].targets.clone();
    let mut task = base.task.clone();
    task.scope.paths = vec![path];
    let mut pin = base.checker_pins[0].clone();
    pin.binding.checker_id = "java.maven.p3c".into();
    pin.binding.tool_id = "maven-p3c".into();
    pin.native_rules[0].native_rule_id =
        guard_protocol::Identifier::new("ClassMustHaveAuthorRule").unwrap();
    let artifact = pin
        .artifact_digest
        .as_str()
        .strip_prefix("sha256:")
        .unwrap()
        .to_string();
    let ctx = super::ProjectionContext::new(
        &plan,
        &task,
        &base.rules,
        &base.producer,
        &base.run,
        &base.raw_evidence,
        &[pin.clone()],
    )
    .unwrap();
    let proof = super::ProjectionSourceProof::new(
        &plan,
        &ctx.run,
        &plan.targets[0],
        "/scratch/Sample.java",
        source,
        &source_ref,
    )
    .unwrap();
    let xml=br#"<?xml version="1.0"?><pmd xmlns="http://pmd.sourceforge.net/report/2.0.0" version="6.15.0" timestamp="2026-10-17T00:00:00Z"><file name="/scratch/Sample.java"><violation beginline="1" endline="1" begincolumn="1" endcolumn="1" rule="ClassMustHaveAuthorRule" ruleset="AlibabaJavaComments" priority="3">Missing author</violation></file></pmd>"#;
    let bound = ctx
        .clone()
        .with_java_execution(
            guard_protocol::Identifier::new("java.maven.p3c").unwrap(),
            xml,
            &private_ref(&ctx, xml),
            1,
            guard_protocol::Identifier::new("6.15.0").unwrap(),
            vec![proof],
        )
        .unwrap();
    report["native_results"]["java_p3c"] = serde_json::json!({"schema_version":"0.2.0","report_type":"java_p3c_project_observation","checker_id":"java.maven.p3c","authority":"local_unverified","delivery_decision":"not_evaluated","files":[{"observation":{"schema_version":"0.2.0","report_type":"java_p3c_local_feedback","maven_tool_sha256":artifact,"local_status":"incomplete","reason":"native_execution_failed","source_sha256":source_ref.digest.as_str().strip_prefix("sha256:").unwrap(),"path":"src/main/java/Sample.java","declared_rulesets":["rulesets/java/ali-comment.xml"],"findings":[{"path":"src/main/java/Sample.java","rule_id":"ClassMustHaveAuthorRule","ruleset":"AlibabaJavaComments","native_priority":3,"line":1,"column":1}]}}]});
    let (evidence, _, unbound) = super::feedback_java::extract(&report, &bound, &pin);
    assert!(unbound.is_empty());
    assert_eq!(evidence.result.findings.len(), 1);
    assert_eq!(evidence.result.findings[0].severity, "native_priority:3");
    assert_eq!(
        evidence.result.completion,
        codeguard_core::Completion::Incomplete
    );
    for (replacement, confirmed) in [
        (
            String::from_utf8(xml.to_vec())
                .unwrap()
                .replace("6.15.0", "6.14.0"),
            false,
        ),
        (
            String::from_utf8(xml.to_vec()).unwrap().replace(
                "</pmd>",
                "<error filename=\"/scratch/Other.java\" msg=\"native processing failed\"/></pmd>",
            ),
            true,
        ),
    ] {
        let variant = ctx
            .clone()
            .with_java_execution(
                guard_protocol::Identifier::new("java.maven.p3c").unwrap(),
                replacement.as_bytes(),
                &private_ref(&ctx, replacement.as_bytes()),
                1,
                guard_protocol::Identifier::new("6.15.0").unwrap(),
                bound.java_executions[0].sources.clone(),
            )
            .unwrap();
        let (evidence, _, unbound) = super::feedback_java::extract(&report, &variant, &pin);
        if confirmed {
            assert_eq!(
                evidence.result.findings.len(),
                1,
                "native processing failure erased confirmed positive"
            );
            assert!(unbound.is_empty());
        } else {
            assert!(
                evidence.result.findings.is_empty(),
                "wrong PMD version was accepted as confirmed finding"
            );
            assert_eq!(unbound.len(), 1);
        }
    }
    report["native_results"]["java_p3c"]["files"][0]["observation"]["source_sha256"] =
        serde_json::json!("f".repeat(64));
    let (evidence, _, unbound) = super::feedback_java::extract(&report, &bound, &pin);
    assert!(evidence.result.findings.is_empty());
    assert_eq!(unbound.len(), 1);
}

#[test]
fn current_maven_producer_slot_assembly_preserves_exact_closed_feedback_contract() {
    let mut report = actual_maven_javadoc_feedback();
    report["schema_version"] = serde_json::json!("0.73.0");
    let old = report["native_results"].clone();
    report["native_results"] = crate::check_native_results::assemble(
        old.as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect(),
    );
    crate::check_report_assembly::finalize(
        &mut report,
        crate::check_report_assembly::ReportShapeInput {
            c_paths_nonempty: false,
            c_family_comments: serde_json::Value::Null,
            gradle_cve_requested: false,
            gradle_requested: false,
            gradle_result_key: "java_gradle_model",
            java_gradle_cve: serde_json::Value::Null,
            gradle_cve_tasks: serde_json::Value::Null,
            java_gradle_model: serde_json::Value::Null,
            gradle_javadoc_tasks: serde_json::Value::Null,
            zig_lint: old["zig_lint"].clone(),
            ruby_lint: old["ruby_lint"].clone(),
            shell_lint: old["shell_lint"].clone(),
            lint_only: false,
        },
    );
    let version = report["schema_version"].as_str().unwrap();
    assert!(
        super::feedback_schema::validate(&report, version).is_ok(),
        "producer assembly omitted required closed-schema native slots: {}",
        report["native_results"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    for absent in ["rust_lint", "kotlin_lint", "swift_lint"] {
        assert!(
            report["native_results"].get(absent).is_none(),
            "unused native slot must remain absent: {absent}"
        );
    }
    let mut rust = actual_clippy_feedback();
    rust["schema_version"] = serde_json::json!("0.73.0");
    rust["gradle_javadoc_tasks"] = serde_json::Value::Null;
    assert!(
        super::feedback_schema::validate(&rust, "0.73.0").is_ok(),
        "known real Rust native payload must remain supported"
    );
    let mut old_claim = report.clone();
    old_claim["schema_version"] = serde_json::json!("0.69.0");
    assert!(super::feedback_schema::validate(&old_claim, "0.69.0").is_err());
    for (name, value) in [
        ("unknown_slot", serde_json::json!({})),
        ("rust_lint", serde_json::json!({"schema_version":"9.0.0"})),
    ] {
        let mut invalid = report.clone();
        invalid["native_results"][name] = value;
        assert!(
            super::feedback_schema::validate(&invalid, "0.73.0").is_err(),
            "new wire accepted invalid native slot {name}"
        );
    }
    let mut missing = report.clone();
    missing.as_object_mut().unwrap().remove("command_status");
    assert!(super::feedback_schema::validate(&missing, "0.73.0").is_err());
    let mut unexpected = report.clone();
    unexpected["unknown_root"] = serde_json::json!(true);
    assert!(super::feedback_schema::validate(&unexpected, "0.73.0").is_err());
    assert!(super::feedback_schema::validate(&report, "0.74.0").is_err());
    assert!(super::feedback_schema::validate(&report, "0.073.0").is_err());
}

#[test]
fn current_feedback_selection_matches_the_actual_product_registry() {
    use std::collections::BTreeSet;

    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../schemas/check-feedback-v0.73.schema.json"
    ))
    .unwrap();
    let actual: BTreeSet<String> = schema["oneOf"][0]["properties"]["selection"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect();
    let mut expected: BTreeSet<String> = codeguard_adapters::legacy_registry()
        .unwrap()
        .languages
        .into_iter()
        .map(|language| language.id)
        .collect();
    expected.insert("all".to_owned());
    assert_eq!(
        actual, expected,
        "wire choices must match the product registry"
    );
}

#[test]
fn closed_schema_diagnostics_locate_static_fields_without_echoing_raw_values() {
    let mut report = actual_maven_javadoc_feedback();
    report["schema_version"] = serde_json::json!("0.73.0");
    report["command_status"] = serde_json::json!("PRIVATE_UNTRUSTED_VALUE_DO_NOT_LOG");
    let error = super::feedback_schema::validate(&report, "0.73.0").unwrap_err();
    assert!(error.contains("properties/command_status"), "{error}");
    assert!(!error.contains("PRIVATE_UNTRUSTED_VALUE_DO_NOT_LOG"));
    assert!(!error.contains("credentials"));
}
