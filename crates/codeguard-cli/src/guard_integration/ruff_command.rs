//! Separate opt-in transport for the reviewed Ruff profile; never executes native tools.
use super::{
    command::{diagnostic, encode, exit_code, json, read},
    envelope::FrozenRun,
    projection::{ProtectedMapping, exact},
    ruff_profile::{RuffF401Policy, project_ruff, read_ruff_feedback},
};
use guardengine::{
    GuardContract, GuardSubject,
    integration::{RunBinding, RunStatus},
};
use serde::Deserialize;
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
    target: String,
    producer_sha256: String,
    started_at: String,
    finished_at: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Invocation {
    version: String,
    package_version: String,
    native_schema: String,
    executable: String,
    argv: Vec<String>,
    run_id: String,
    process_exit: u64,
    producer_sha256: String,
}
struct Prepared {
    frozen: FrozenRun,
    policy: RuffF401Policy,
    invocation: Invocation,
    mapping: ProtectedMapping,
    contract: GuardContract,
    subject: GuardSubject,
    native_path: PathBuf,
}
fn prepare(args: &[String]) -> Result<Prepared, &'static str> {
    if args.len() != 10 {
        return Err("invalid_arguments");
    }
    let mut paths = BTreeMap::new();
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
    if context.version != "codeguard.ruff-context/v1alpha1" {
        return Err("unsupported_context_version");
    }
    let source = context
        .binding
        .source_snapshot_digest
        .strip_prefix("sha256:")
        .ok_or("invalid_source_digest")?;
    let policy = RuffF401Policy::new(&context.target, source, &context.producer_sha256)?;
    let invocation: Invocation = json(&read(&paths["--invocation"], 16_384)?)?;
    if invocation.version != "codeguard.ruff-invocation/v1alpha1"
        || invocation.package_version != "0.1.4"
        || invocation.native_schema != "0.13.0"
        || invocation.process_exit != 3
        || invocation.run_id != context.run_id
        || invocation.producer_sha256 != context.producer_sha256
        || invocation
            .argv
            .iter()
            .chain(std::iter::once(&invocation.executable))
            .any(|value| value.len() > 4096 || value.chars().any(char::is_control))
        || !Path::new(&invocation.executable).is_absolute()
        || invocation.argv.len() != 8
        || invocation.argv[0] != "lint"
        || invocation.argv[1] != "python"
        || !Path::new(&invocation.argv[2]).is_absolute()
        || invocation.argv[3] != "--format=json"
        || invocation.argv[4] != "--file"
        || invocation.argv[5] != context.target
        || invocation.argv[6] != "--ruff-tool"
        || !Path::new(&invocation.argv[7]).is_absolute()
    {
        return Err("unsupported_or_conflicting_ruff_invocation");
    }
    let mapping = ProtectedMapping::parse(&read(&paths["--mapping"], INPUT_BYTES)?)?;
    let contract: GuardContract = json(&read(&paths["--contract"], INPUT_BYTES)?)?;
    contract
        .validate()
        .map_err(|_| "invalid_protected_contract")?;
    for rule in &contract.spec.rules {
        let guardengine::GuardAssertion::ForbidRelation {
            subject,
            predicate,
            object,
        } = &rule.assertion;
        if ![subject, predicate, object].iter().all(|s| exact(s)) {
            return Err("unsupported_ruff_contract");
        }
    }
    mapping.validate_references(&contract)?;
    mapping.validate_ruff_scope(&contract)?;
    let subject = GuardSubject {
        id: context.binding.repo_id.clone(),
        snapshot_digest: context.binding.source_snapshot_digest.clone(),
    };
    let frozen = FrozenRun::new(
        &context.run_id,
        context.binding,
        &policy.obligations(),
        &context.started_at,
        &context.finished_at,
    )?;
    Ok(Prepared {
        frozen,
        policy,
        invocation,
        mapping,
        contract,
        subject,
        native_path: paths["--native-report"].clone(),
    })
}
/// Reads only explicitly supplied artifacts. Existing native/guard-project commands never enter here.
pub fn run(args: &[String]) -> ExitCode {
    let mut stderr = io::stderr().lock();
    let prepared = match prepare(args) {
        Ok(value) => value,
        Err(code) => {
            diagnostic(&mut stderr, "unbound", code);
            return ExitCode::from(4);
        }
    };
    let parsed = read(&prepared.native_path, INPUT_BYTES).and_then(|bytes| {
        let document: serde_json::Value = json(&bytes)?;
        if document["files"][0]["recheck_cwd"] != prepared.invocation.argv[2] {
            return Err("captured_root_conflict");
        }
        read_ruff_feedback(
            &bytes,
            &prepared.policy,
            &prepared.invocation.run_id,
            3,
            &prepared.invocation.producer_sha256,
        )
    });
    let mut output = match &parsed {
        Err(code) => prepared.frozen.ruff_error(code, None),
        Ok(evidence) => project_ruff(
            evidence,
            &prepared.mapping,
            &prepared.contract,
            prepared.subject,
        )
        .and_then(|projection| prepared.frozen.complete(projection))
        .unwrap_or_else(|code| prepared.frozen.ruff_error(code, Some(evidence))),
    };
    if codeguard_runtime::sigint_cancellation_requested() {
        output = prepared.frozen.ruff_cancelled(parsed.as_ref().ok());
    }
    let encoded = match encode(&output, 3) {
        Ok(bytes) => bytes,
        Err(code) => {
            output = prepared.frozen.ruff_error(code, parsed.as_ref().ok());
            match encode(&output, 3) {
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
        output = prepared.frozen.ruff_cancelled(parsed.as_ref().ok());
        match encode(&output, 3) {
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
                .map_or("projection_failed", |d| d.code.as_str()),
        );
    }
    ExitCode::from(exit)
}
