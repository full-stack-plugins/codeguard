//! Explicit fixture export for cross-repository SDK checks; not production publication.
use codeguard_cli::guard_integration::{
    envelope::FrozenRun,
    projection::{ProtectedMapping, project},
    reader::read_native,
    scope::FrozenObligations,
};
use guardengine::{GuardContract, GuardSubject, integration::RunBinding};
use serde_json::json;
use std::{collections::BTreeMap, fs, io::Write};
#[path = "../tests/support/guard_integration.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let destination = args
        .next()
        .ok_or("usage: guard_integration_fixture NEW_DIRECTORY")?;
    if args.next().is_some() {
        return Err("exactly one new output directory is required".into());
    }
    let native = serde_json::to_vec(&fixture::valid())?;
    let invocation = fixture::invocation();
    let evidence = read_native(&native, &invocation)?;
    let obligations = FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))?;
    let contract: GuardContract = serde_json::from_value(
        json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"fixture-policy","revision":"1"},"spec":{"rules":[{"id":"r","enforcement":"advise","assertion":{"type":"forbid_relation","subject":"code","predicate":"has","object":"gap"}}]}}),
    )?;
    let mapping_bytes = br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"r"},{"source":{"kind":"gap","detail":"native profile unqualified"},"rule_id":"r"}]}"#;
    let mapping = ProtectedMapping::parse(mapping_bytes)?;
    let binding = RunBinding {
        repo_id: "repo".into(),
        task_id: "task".into(),
        worktree_id: "worktree".into(),
        requirement_ids: vec!["req".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: format!("sha256:{}", "a".repeat(64)),
        baseline_digest: None,
    };
    let frozen = FrozenRun::new(
        "native-001",
        binding.clone(),
        &obligations,
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )?;
    let projected = project(
        &evidence,
        &obligations,
        &mapping,
        &contract,
        GuardSubject {
            id: binding.repo_id.clone(),
            snapshot_digest: binding.source_snapshot_digest.clone(),
        },
    )?;
    let output = frozen.complete(projected)?;
    let manifest = serde_json::to_vec_pretty(&json!({
        "version":"codeguard.fixture-export/v1alpha1", "fixture_only":true,
        "native_qualified":false,"evidence_profile":"engine-backed",
        "coverage":"partial","decision":"BLOCK",
        "provenance":"Synthetic adapter fixture; no real Git or native execution authority",
        "binding":binding,
        "scope":{"python/app/lint/ruff":["src/main.py"]}
    }))?;
    let context = serde_json::to_vec_pretty(&json!({
        "version":"codeguard.context/v1alpha1", "runId":"native-001", "binding":binding,
        "requiredTargets":{"python/app/lint/ruff":["src/main.py"]},
        "startedAt":"2026-10-09T00:00:00Z", "finishedAt":"2026-10-09T00:01:00Z"
    }))?;
    let files = [
        ("envelope.json", serde_json::to_vec(output.envelope())?),
        ("contract.json", output.contract_bytes().unwrap().to_vec()),
        ("facts.json", output.facts_bytes().unwrap().to_vec()),
        ("report.json", output.report_bytes().unwrap().to_vec()),
        ("native.json", output.domain_bytes().to_vec()),
        ("invocation.json", serde_json::to_vec(&invocation)?),
        ("mapping.json", mapping_bytes.to_vec()),
        ("fixture.json", manifest),
        ("context.json", context),
    ];
    let mut directory = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory.create(&destination)?;
    for (name, bytes) in files {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(std::path::Path::new(&destination).join(name))?
            .write_all(&bytes)?;
    }
    println!("wrote engine-backed partial/BLOCK fixture to {destination}");
    Ok(())
}
