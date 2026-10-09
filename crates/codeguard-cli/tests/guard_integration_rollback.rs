//! Explicit local controller composition. Fixture authority is never production identity.
#[path = "guard_integration_freshness.rs"]
mod fixture;
use codeguard_cli::guard_integration::{
    consumer::PreparedConsumption,
    projection::ProtectedMapping,
    ruff_profile::{RuffF401Policy, project_ruff, read_ruff_feedback},
    shadow::compare,
};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};
fn raw(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/guard-integration")
            .join(name),
    )
    .unwrap()
}

struct LocalAuthority {
    calls: std::cell::Cell<usize>,
    revoked: std::cell::Cell<bool>,
}
impl LocalAuthority {
    fn new() -> Self {
        Self {
            calls: 0.into(),
            revoked: false.into(),
        }
    }
}
impl guardengine::integration::eligibility::AuthorityProvider for LocalAuthority {
    fn verify_producer(
        &self,
        envelope: &guardengine::integration::GuardRunEnvelope,
        digest: &str,
    ) -> Result<
        guardengine::integration::eligibility::ProducerRecord,
        guardengine::integration::eligibility::AuthorityError,
    > {
        use guardengine::integration::eligibility::*;
        self.calls.set(self.calls.get() + 1);
        Ok(ProducerRecord {
            principal: "fixture-producer".into(),
            producer: envelope.producer.clone(),
            envelope_digest: digest.into(),
            validity: Validity {
                issued_at: 0,
                expires_at: 2_000_000_000,
                revoked: self.revoked.get(),
            },
        })
    }
    fn verify_approval(
        &self,
        _: &str,
    ) -> Result<
        guardengine::integration::eligibility::ApprovalRecord,
        guardengine::integration::eligibility::AuthorityError,
    > {
        Err(guardengine::integration::eligibility::AuthorityError::Untrusted)
    }
}

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cg-rollback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn put(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.0.join(name), bytes).unwrap();
    }
    fn snapshot(&self) -> std::collections::BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (
                    e.file_name().to_string_lossy().into(),
                    std::fs::read(e.path()).unwrap(),
                )
            })
            .collect()
    }
    fn native_version(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("--version")
            .current_dir(&self.0)
            .env_clear()
            .output()
            .unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn actual_default_shadow_optin_disable_preserves_native_and_required_policy() {
    let files = Files::new();
    let (output, expected) = fixture::sample(false);
    let prepared = PreparedConsumption::freeze(&expected, None).unwrap();
    let provider = LocalAuthority::new();
    let native = output.domain_bytes();
    files.put("native.json", native);
    let before = files.snapshot();
    let old_native = files.native_version();
    assert!(old_native.status.success());
    assert_eq!(before, files.snapshot());
    assert_eq!(provider.calls.get(), 0);
    assert!(compare(native, None).unwrap().engine_report().is_err());
    assert_eq!(
        prepared
            .consume_required(None, &provider, fixture::NOW, None)
            .err(),
        Some("required integration evidence missing")
    );

    // Real retained native bytes, independently fixed local policy; comparison is not consumption.
    let manifest: Value = serde_json::from_slice(&raw("ruff-f401/capture.json")).unwrap();
    let doc: Value = serde_json::from_slice(native).unwrap();
    let producer = manifest["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        manifest["cases"]["clean"]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let evidence = read_ruff_feedback(
        native,
        &policy,
        doc["run_id"].as_str().unwrap(),
        3,
        producer,
    )
    .unwrap();
    let vocabulary: Value = serde_json::from_slice(&raw("relations.json")).unwrap();
    let mapping =
        ProtectedMapping::parse(&serde_json::to_vec(&vocabulary["mapping"]).unwrap()).unwrap();
    let projection = project_ruff(
        &evidence,
        &mapping,
        &serde_json::from_slice(output.contract_bytes().unwrap()).unwrap(),
        guardengine::GuardSubject {
            id: "repo".into(),
            snapshot_digest: policy.source_digest(),
        },
    )
    .unwrap();
    assert_eq!(
        compare(native, Some(&projection))
            .unwrap()
            .engine_report()
            .unwrap()
            .decision,
        guardengine::Decision::Allow
    );
    assert_eq!(provider.calls.get(), 0);
    assert!(
        prepared
            .consume_required(None, &provider, fixture::NOW, None)
            .is_err()
    );

    // Actual explicit CLI returns a projection, still not authentication or a write grant.
    let argv = manifest["cases"]["clean"]["argv"].as_array().unwrap();
    let invocation = json!({"version":"codeguard.ruff-invocation/v1alpha1","packageVersion":"0.1.4","nativeSchema":"0.13.0","executable":argv[0],"argv":&argv[1..],"runId":doc["run_id"],"processExit":3,"producerSha256":producer});
    let context = json!({"version":"codeguard.ruff-context/v1alpha1","runId":doc["run_id"],"binding":expected.policy.binding,"target":"app.py","producerSha256":producer,"startedAt":"2026-10-09T00:00:00Z","finishedAt":"2026-10-09T00:01:00Z"});
    for (name, value) in [
        ("invocation.json", invocation),
        ("context.json", context),
        ("mapping.json", vocabulary["mapping"].clone()),
    ] {
        files.put(name, &serde_json::to_vec(&value).unwrap());
    }
    files.put("contract.json", output.contract_bytes().unwrap());
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command
        .arg("guard-project-ruff")
        .current_dir(&files.0)
        .env_clear();
    for (flag, name) in [
        ("--native-report", "native.json"),
        ("--invocation", "invocation.json"),
        ("--context", "context.json"),
        ("--mapping", "mapping.json"),
        ("--contract", "contract.json"),
    ] {
        command.arg(flag).arg(files.0.join(name));
    }
    let cli = command.output().unwrap();
    assert_eq!(cli.status.code(), Some(0));
    let bundle: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(bundle["nativeExit"], 3);
    assert_eq!(
        bundle["envelope"],
        serde_json::to_value(output.envelope()).unwrap()
    );
    for (name, bytes) in [
        ("contract", output.contract_bytes().unwrap()),
        ("facts", output.facts_bytes().unwrap()),
        ("report", output.report_bytes().unwrap()),
        ("domain", output.domain_bytes()),
    ] {
        assert_eq!(
            bundle["artifacts"][name].as_str().unwrap().as_bytes(),
            bytes
        );
    }
    assert_eq!(provider.calls.get(), 0);
    files.put("retained-projection.json", &cli.stdout);
    let enabled = prepared
        .consume_required(Some(&output), &provider, fixture::NOW, None)
        .unwrap();
    assert!(enabled.eligibility.eligible);
    assert_eq!(provider.calls.get(), 1);
    let retained = files.snapshot();

    // Disable only stops submitting integration evidence: never delete history or weaken policy.
    assert!(
        prepared
            .consume_required(None, &provider, fixture::NOW, None)
            .is_err()
    );
    assert_eq!(provider.calls.get(), 1);
    let after_native = files.native_version();
    assert_eq!(old_native.status.code(), after_native.status.code());
    assert_eq!(old_native.stdout, after_native.stdout);
    assert_eq!(old_native.stderr, after_native.stderr);
    assert_eq!(retained, files.snapshot());
    provider.revoked.set(true);
    assert!(
        !prepared
            .consume_required(Some(&output), &provider, fixture::NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    assert_eq!(provider.calls.get(), 2);
    assert_eq!(retained, files.snapshot());
    assert_eq!(output.domain_bytes(), native);
    if let Some(destination) = std::env::var_os("CODEGUARD_ROLLOUT_CAPTURE_DIR") {
        let destination = PathBuf::from(destination);
        std::fs::create_dir(&destination).unwrap();
        for (name, bytes) in &retained {
            std::fs::write(destination.join(name), bytes).unwrap();
        }
        std::fs::write(
            destination.join("native-version.stdout"),
            &old_native.stdout,
        )
        .unwrap();
        std::fs::write(
            destination.join("native-version.stderr"),
            &old_native.stderr,
        )
        .unwrap();
        let summary = json!({
            "profile": "explicit-local-controller-library",
            "authority": "fixture-provider-not-production",
            "nativeExit": 3, "projectionCliExit": cli.status.code(),
            "nativeEntryStatus": old_native.status.code(),
            "defaultRequiredMissingRejected": true, "shadowHasNoAuthority": true,
            "explicitCurrentConsumptionEligible": enabled.eligibility.eligible,
            "disabledRequiredMissingRejected": true, "reenableRevocationRejected": true,
            "authorityCalls": provider.calls.get(), "retainedFilesUnchanged": true,
            "cliAndSdkEnvelopeAndArtifactBytesEqual": true,
            "expectedConsumption": expected
        });
        std::fs::write(
            destination.join("summary.json"),
            serde_json::to_vec_pretty(&summary).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn required_some_delegates_without_repairing_block_partial_or_missing_approval() {
    for (name, review) in [("bad", false), ("missing-tool", false), ("bad", true)] {
        let (output, expected) = fixture::sample_native(name, review);
        let prepared = PreparedConsumption::freeze(&expected, None).unwrap();
        let provider = LocalAuthority::new();
        let direct = prepared
            .consume(&output, &provider, fixture::NOW, None)
            .unwrap();
        let required = prepared
            .consume_required(Some(&output), &provider, fixture::NOW, None)
            .unwrap();
        assert!(!required.eligibility.eligible);
        assert_eq!(required.freshness_key, direct.freshness_key);
        assert_eq!(
            required.eligibility.technical_decision,
            direct.eligibility.technical_decision
        );
        assert_eq!(required.eligibility.code, direct.eligibility.code);
    }
}
