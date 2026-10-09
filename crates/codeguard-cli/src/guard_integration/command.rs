//! Explicit Unix file-to-SDK transport. No native execution or implicit publication.
use super::{
    envelope::{EnvelopeOutput, FrozenRun},
    profile::InvocationDescriptor,
    projection::{ProtectedMapping, project},
    reader::read_native,
    scope::FrozenObligations,
};
use guardengine::{
    Decision, GuardContract, GuardSubject,
    integration::{GuardRunEnvelope, MAX_ARTIFACT_BYTES, RunBinding, RunStatus},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const INPUT_BYTES: u64 = 1_048_576;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    version: String,
    run_id: String,
    binding: RunBinding,
    required_targets: BTreeMap<String, Vec<String>>,
    started_at: String,
    finished_at: String,
}
struct Prepared {
    frozen: FrozenRun,
    invocation: InvocationDescriptor,
    obligations: FrozenObligations,
    mapping: ProtectedMapping,
    contract: GuardContract,
    subject: GuardSubject,
    native_path: PathBuf,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TransportDiagnostic<'a> {
    api_version: &'static str,
    kind: &'static str,
    phase: &'static str,
    code: &'a str,
    exit_code: u8,
}
pub(super) fn diagnostic(stderr: &mut impl Write, phase: &'static str, code: &str) {
    let _ = serde_json::to_writer(
        &mut *stderr,
        &TransportDiagnostic {
            api_version: "codeguard.transport/v1alpha1",
            kind: "GuardTransportDiagnostic",
            phase,
            code,
            exit_code: 4,
        },
    );
    let _ = stderr.write_all(b"\n");
}
pub(super) fn read(path: &Path, cap: u64) -> Result<Vec<u8>, &'static str> {
    codeguard_runtime::read_bounded_regular_file(path, cap).map_err(|_| "artifact_read_failed")
}
pub(super) fn json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, &'static str> {
    let value = codeguard_adapters::parse_unique_json(bytes).map_err(|_| "invalid_input_json")?;
    serde_json::from_value(value).map_err(|_| "invalid_input_fields")
}
fn prepare(args: &[String]) -> Result<Prepared, &'static str> {
    let mut paths = BTreeMap::new();
    if args.len() != 10 {
        return Err("invalid_arguments");
    }
    for pair in args.chunks_exact(2) {
        if ![
            "--invocation",
            "--native-report",
            "--context",
            "--mapping",
            "--contract",
        ]
        .contains(&pair[0].as_str())
            || pair[1].is_empty()
            || pair[1].starts_with("--")
            || paths
                .insert(pair[0].as_str(), PathBuf::from(&pair[1]))
                .is_some()
        {
            return Err("invalid_arguments");
        }
    }
    let context: Context = json(&read(&paths["--context"], INPUT_BYTES)?)?;
    if context.version != "codeguard.context/v1alpha1" {
        return Err("unsupported_context_version");
    }
    let obligations = FrozenObligations::new(context.required_targets)?;
    let invocation = InvocationDescriptor::parse(&read(&paths["--invocation"], 16_384)?)?;
    if invocation.run_id != context.run_id {
        return Err("context_invocation_conflict");
    }
    let mapping = ProtectedMapping::parse(&read(&paths["--mapping"], INPUT_BYTES)?)?;
    // Only JSON contracts on this new CLI: no aliases or unrestricted YAML loader.
    let contract: GuardContract = json(&read(&paths["--contract"], INPUT_BYTES)?)?;
    contract
        .validate()
        .map_err(|_| "invalid_protected_contract")?;
    let subject = GuardSubject {
        id: context.binding.repo_id.clone(),
        snapshot_digest: context.binding.source_snapshot_digest.clone(),
    };
    let frozen = FrozenRun::new(
        &context.run_id,
        context.binding,
        &obligations,
        &context.started_at,
        &context.finished_at,
    )?;
    Ok(Prepared {
        frozen,
        invocation,
        obligations,
        mapping,
        contract,
        subject,
        native_path: paths["--native-report"].clone(),
    })
}
#[derive(Serialize)]
struct InlineArtifacts<'a> {
    contract: Option<&'a str>,
    facts: Option<&'a str>,
    report: Option<&'a str>,
    domain: Option<&'a str>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bundle<'a> {
    api_version: &'static str,
    kind: &'static str,
    native_exit: u64,
    envelope: &'a GuardRunEnvelope,
    artifacts: InlineArtifacts<'a>,
}
pub(super) fn encode(output: &EnvelopeOutput, native_exit: u64) -> Result<Vec<u8>, &'static str> {
    fn text(bytes: Option<&[u8]>) -> Result<Option<&str>, &'static str> {
        bytes
            .map(std::str::from_utf8)
            .transpose()
            .map_err(|_| "artifact_encoding_failed")
    }
    let bundle = Bundle {
        api_version: "codeguard.projection/v1alpha1",
        kind: "GuardProjectionBundle",
        native_exit,
        envelope: output.envelope(),
        artifacts: InlineArtifacts {
            contract: text(output.contract_bytes())?,
            facts: text(output.facts_bytes())?,
            report: text(output.report_bytes())?,
            domain: text((!output.domain_bytes().is_empty()).then(|| output.domain_bytes()))?,
        },
    };
    struct Bounded(Vec<u8>);
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > MAX_ARTIFACT_BYTES.saturating_sub(self.0.len()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "transport size exceeded",
                ));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut encoded = Bounded(Vec::new());
    serde_json::to_writer(&mut encoded, &bundle).map_err(|_| "transport_serialization_failed")?;
    encoded
        .write_all(b"\n")
        .map_err(|_| "transport_serialization_failed")?;
    Ok(encoded.0)
}
pub(super) fn exit_code(envelope: &GuardRunEnvelope) -> u8 {
    if envelope.run_status != RunStatus::Completed {
        return 4;
    }
    match envelope.decision {
        Some(Decision::Allow) => 0,
        Some(Decision::Block) => 2,
        Some(Decision::RequireApproval) => 3,
        None => 4,
    }
}
/// Invoke only for the explicit guard-project command. Native commands never enter here.
pub fn run(args: &[String]) -> ExitCode {
    let mut stderr = io::stderr().lock();
    let prepared = match prepare(args) {
        Ok(value) => value,
        Err(code) => {
            diagnostic(&mut stderr, "unbound", code);
            return ExitCode::from(4);
        }
    };
    let parsed = read(&prepared.native_path, INPUT_BYTES)
        .and_then(|bytes| read_native(&bytes, &prepared.invocation));
    let mut output = match &parsed {
        Err(code) => prepared.frozen.error(code, None),
        Ok(evidence) => {
            let result = if matches!(evidence.report().exit_code, 4 | 130) {
                prepared.frozen.failure(evidence)
            } else {
                project(
                    evidence,
                    &prepared.obligations,
                    &prepared.mapping,
                    &prepared.contract,
                    prepared.subject,
                )
                .and_then(|projection| prepared.frozen.complete(projection))
            };
            result.unwrap_or_else(|code| prepared.frozen.error(code, Some(evidence)))
        }
    };
    if codeguard_runtime::sigint_cancellation_requested() {
        output = prepared.frozen.cancelled(parsed.as_ref().ok());
    }
    let encoded = match encode(&output, prepared.invocation.process_exit) {
        Ok(bytes) => bytes,
        Err(code) => {
            output = prepared.frozen.error(code, parsed.as_ref().ok());
            match encode(&output, prepared.invocation.process_exit) {
                Ok(bytes) => bytes,
                Err(code) => {
                    diagnostic(&mut stderr, "bound", code);
                    return ExitCode::from(4);
                }
            }
        }
    };
    let encoded = if codeguard_runtime::sigint_cancellation_requested()
        && output.envelope().run_status != RunStatus::Cancelled
    {
        output = prepared.frozen.cancelled(parsed.as_ref().ok());
        match encode(&output, prepared.invocation.process_exit) {
            Ok(bytes) => bytes,
            Err(code) => {
                diagnostic(&mut stderr, "bound", code);
                return ExitCode::from(4);
            }
        }
    } else {
        encoded
    };
    let exit = exit_code(output.envelope());
    if io::stdout().lock().write_all(&encoded).is_err() {
        diagnostic(&mut stderr, "bound", "stdout_write_failed");
        return ExitCode::from(4);
    }
    if exit == 4 {
        diagnostic(
            &mut stderr,
            "bound",
            output
                .envelope()
                .diagnostics
                .first()
                .map_or("projection_failed", |diagnostic| diagnostic.code.as_str()),
        );
    }
    ExitCode::from(exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frozen() -> FrozenRun {
        let obligations =
            FrozenObligations::new(BTreeMap::from([("scope".into(), vec!["target".into()])]))
                .unwrap();
        FrozenRun::new(
            "run",
            RunBinding {
                repo_id: "repo".into(),
                task_id: "task".into(),
                worktree_id: "tree".into(),
                requirement_ids: vec!["R1".into()],
                candidate_oid: "a".repeat(40),
                base_oid: "b".repeat(40),
                merge_group_id: None,
                source_snapshot_digest: format!("sha256:{}", "c".repeat(64)),
                baseline_digest: None,
            },
            &obligations,
            "2026-10-09T00:00:00Z",
            "2026-10-09T00:01:00Z",
        )
        .unwrap()
    }
    #[test]
    fn generic_exit_mapping_does_not_qualify_current_native_profile() {
        let mut envelope = frozen().error("test", None).envelope().clone();
        assert_eq!(exit_code(&envelope), 4);
        envelope.run_status = RunStatus::Completed;
        for (decision, expected) in [
            (Decision::Allow, 0),
            (Decision::Block, 2),
            (Decision::RequireApproval, 3),
        ] {
            envelope.decision = Some(decision);
            assert_eq!(exit_code(&envelope), expected);
        }
        envelope.decision = None;
        assert_eq!(exit_code(&envelope), 4);
    }
    #[test]
    fn bound_adapter_cancellation_is_null_and_has_no_success_report() {
        let output = frozen().cancelled(None);
        assert_eq!(output.envelope().run_status, RunStatus::Cancelled);
        assert!(output.envelope().decision.is_none());
        assert!(output.report_bytes().is_none());
        assert_eq!(exit_code(output.envelope()), 4);
        output
            .envelope()
            .validate(guardengine::integration::EvidenceProfile::NativeOnly)
            .unwrap();
    }
}
