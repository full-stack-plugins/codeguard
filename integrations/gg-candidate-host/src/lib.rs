//! Opt-in trusted local executor. No native workspace dependency direction changes.
use codeguard_cli::guard_integration::ruff_profile::CONFIG;
use gitguard::{Repository, candidate::CandidateSnapshot};
use sha2::{Digest, Sha256};
use std::path::Path;
pub struct PreparedCandidate {
    repo: Repository,
    candidate: CandidateSnapshot,
    target: String,
    source: Vec<u8>,
    source_digest: String,
}
impl PreparedCandidate {
    /// The snapshot and explicit root come from the independent controller, never candidate files.
    pub fn from_bytes(bytes: &[u8], root: &Path, target: &str) -> Result<Self, &'static str> {
        if bytes.len() > 1_048_576
            || target.len() > 256
            || !target.ends_with(".py")
            || target.starts_with('/')
            || target.chars().any(|c| c.is_control() || c == '\\')
            || target
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == ".." || p == ".git")
        {
            return Err("invalid candidate input");
        }
        let candidate: CandidateSnapshot =
            serde_json::from_slice(bytes).map_err(|_| "invalid candidate snapshot")?;
        let repo = Repository::discover(root, candidate.repo_id())
            .map_err(|_| "repository unavailable")?;
        candidate
            .validate(&repo)
            .map_err(|_| "candidate binding mismatch")?;
        if !candidate.clean() {
            return Err("candidate is not clean");
        }
        if !candidate.allowed_paths().iter().any(|p| {
            target.as_bytes() == p
                || target
                    .as_bytes()
                    .strip_prefix(p.as_slice())
                    .is_some_and(|suffix| suffix.starts_with(b"/"))
        }) {
            return Err("target outside frozen scope");
        }
        let files = repo
            .read_commit_files(candidate.candidate_oid())
            .map_err(|_| "unsupported candidate tree")?;
        let config = files
            .iter()
            .find(|f| f.path() == b"pyproject.toml")
            .ok_or("missing fixed configuration")?;
        if config.contents() != CONFIG.as_bytes() {
            return Err("candidate configuration differs");
        }
        let file = files
            .iter()
            .find(|f| f.path() == target.as_bytes())
            .ok_or("candidate target missing")?;
        if file.contents().len() > 1_048_576 {
            return Err("source budget exceeded");
        }
        let source = file.contents().to_vec();
        let source_digest = format!("{:x}", Sha256::digest(&source));
        Ok(Self {
            repo,
            candidate,
            target: target.into(),
            source,
            source_digest,
        })
    }
    pub fn candidate_snapshot_digest(&self) -> &str {
        self.candidate.source_snapshot_digest()
    }
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
}
/// Protected local executor configuration. A supplied digest is not external authentication.
pub struct ProducerPin {
    pub executable: std::path::PathBuf,
    pub sha256: String,
    pub ruff_executable: std::path::PathBuf,
}
pub struct CandidateCapture {
    prepared: PreparedCandidate,
    policy: codeguard_cli::guard_integration::ruff_profile::RuffF401Policy,
    evidence: codeguard_cli::guard_integration::ruff_profile::RuffEvidence,
    run_id: String,
    started: String,
    finished: String,
    provenance: serde_json::Value,
}
pub struct CandidateProjection {
    envelope: guardengine::integration::GuardRunEnvelope,
    provenance_bytes: Vec<u8>,
    output: codeguard_cli::guard_integration::envelope::EnvelopeOutput,
    provenance: serde_json::Value,
}
fn now() -> Result<String, &'static str> {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| "clock unavailable")
}
fn verify_binary(
    path: &Path,
    expected: &str,
    deadline: std::time::Instant,
) -> Result<(), &'static str> {
    use std::io::Read;
    if !path.is_absolute()
        || expected.len() != 64
        || !expected
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid controller binary pin");
    }
    let meta = std::fs::symlink_metadata(path).map_err(|_| "binary unavailable")?;
    if !meta.is_file() || meta.len() > 256 * 1024 * 1024 {
        return Err("unsupported binary file");
    }
    let file = std::fs::File::open(path).map_err(|_| "binary unavailable")?;
    let mut reader = file.take(256 * 1024 * 1024 + 1);
    let mut hash = Sha256::new();
    let mut bytes = [0u8; 65536];
    let mut total = 0;
    loop {
        if std::time::Instant::now() >= deadline {
            return Err("binary verification deadline");
        }
        let n = reader.read(&mut bytes).map_err(|_| "binary read failed")?;
        if n == 0 {
            break;
        }
        total += n;
        if total > 256 * 1024 * 1024 {
            return Err("binary budget exceeded");
        }
        hash.update(&bytes[..n]);
    }
    if format!("{:x}", hash.finalize()) != expected {
        return Err("controller binary pin mismatch");
    }
    Ok(())
}
impl PreparedCandidate {
    /// Actual invocation is required: raw reports/digests cannot construct this capture type.
    pub fn capture(
        self,
        pin: &ProducerPin,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<CandidateCapture, &'static str> {
        use codeguard_cli::guard_integration::ruff_profile::{
            RuffF401Policy, TOOL_SHA256, read_ruff_feedback,
        };
        use codeguard_runtime::{ProcessSpec, Termination, run_process};
        let started = now()?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        verify_binary(&pin.executable, &pin.sha256, deadline)?;
        verify_binary(&pin.ruff_executable, TOOL_SHA256, deadline)?;
        self.candidate
            .validate(&self.repo)
            .map_err(|_| "candidate changed before native capture")?;
        let project = tempfile::tempdir().map_err(|_| "private project unavailable")?;
        let target = project.path().join(&self.target);
        std::fs::create_dir_all(target.parent().ok_or("invalid target")?)
            .map_err(|_| "private target unavailable")?;
        std::fs::write(&target, &self.source).map_err(|_| "private source write failed")?;
        std::fs::write(project.path().join("pyproject.toml"), CONFIG)
            .map_err(|_| "private configuration write failed")?;
        let args: Vec<std::ffi::OsString> = vec![
            "lint".into(),
            "python".into(),
            project.path().as_os_str().into(),
            "--format=json".into(),
            "--file".into(),
            self.target.clone().into(),
            "--ruff-tool".into(),
            pin.ruff_executable.as_os_str().into(),
        ];
        let spec = ProcessSpec {
            executable: pin.executable.clone(),
            args,
            cwd: project.path().into(),
            env: std::collections::BTreeMap::new(),
            stdin: None,
            deadline,
            output_limit_bytes: 1_048_576,
        };
        let outcome = run_process(&spec, cancel);
        if outcome.termination != Termination::Exited(3) {
            return Err("native capture did not complete in supported profile");
        }
        self.candidate
            .validate(&self.repo)
            .map_err(|_| "candidate changed during native capture")?;
        let policy = RuffF401Policy::new(&self.target, &self.source_digest, &pin.sha256)?;
        let document: serde_json::Value =
            serde_json::from_slice(&outcome.stdout).map_err(|_| "invalid native feedback")?;
        let run_id = document["run_id"]
            .as_str()
            .ok_or("missing native run identity")?
            .to_owned();
        if document["files"][0]["recheck_cwd"] != project.path().to_string_lossy().as_ref() {
            return Err("native captured root mismatch");
        }
        let evidence = read_ruff_feedback(&outcome.stdout, &policy, &run_id, 3, &pin.sha256)?;
        let finished = now()?;
        let provenance = serde_json::json!({"version":"codeguard.gg-candidate-capture/v1alpha1","profile":"local-trusted-executor","authorization":"not-evaluated","candidate":self.candidate,"candidateBindingDigest":self.candidate.binding_digest(),"sourceScope":{"target":self.target,"sourceSha256":self.source_digest,"configSha256":format!("{:x}",Sha256::digest(CONFIG.as_bytes()))},"invocation":{"executable":pin.executable,"producerSha256":pin.sha256,"toolSha256":TOOL_SHA256,"argv":spec.args.iter().map(|s|s.to_string_lossy()).collect::<Vec<_>>(),"nativeExit":3,"stdoutSha256":format!("{:x}",Sha256::digest(&outcome.stdout)),"stderrSha256":format!("{:x}",Sha256::digest(&outcome.stderr))},"runId":run_id,"startedAt":started,"finishedAt":finished});
        Ok(CandidateCapture {
            prepared: self,
            policy,
            evidence,
            run_id,
            started,
            finished,
            provenance,
        })
    }
}
impl CandidateCapture {
    pub fn native_exit(&self) -> u8 {
        self.evidence.native_exit()
    }
    pub fn raw_bytes(&self) -> &[u8] {
        self.evidence.raw_bytes()
    }
    pub fn provenance(&self) -> &serde_json::Value {
        &self.provenance
    }
    pub fn source_bytes(&self) -> &[u8] {
        &self.prepared.source
    }
    /// Expected current queue identity is supplied separately from the captured observation.
    pub fn project(
        &self,
        expected: &[u8],
        mapping: &codeguard_cli::guard_integration::projection::ProtectedMapping,
        contract: &guardengine::GuardContract,
    ) -> Result<CandidateProjection, &'static str> {
        use codeguard_cli::guard_integration::{envelope::FrozenRun, ruff_profile::project_ruff};
        if expected.len() > 1_048_576 {
            return Err("expected candidate budget exceeded");
        }
        let expected: CandidateSnapshot =
            serde_json::from_slice(expected).map_err(|_| "invalid expected candidate")?;
        if expected.binding_digest() != self.prepared.candidate.binding_digest() {
            return Err("captured candidate differs from current controller binding");
        }
        expected
            .validate(&self.prepared.repo)
            .map_err(|_| "candidate no longer matches clean capture")?;
        if !expected.clean() {
            return Err("current candidate is not clean");
        }
        let binding = guardengine::integration::RunBinding {
            repo_id: expected.repo_id().into(),
            task_id: expected.task_id().into(),
            worktree_id: expected.worktree_id().into(),
            requirement_ids: expected.requirement_ids().to_vec(),
            candidate_oid: expected.candidate_oid().into(),
            base_oid: expected.base_oid().into(),
            merge_group_id: expected.merge_group_id().map(str::to_owned),
            source_snapshot_digest: self.policy.source_digest(),
            baseline_digest: None,
        };
        let subject = guardengine::GuardSubject {
            id: binding.repo_id.clone(),
            snapshot_digest: binding.source_snapshot_digest.clone(),
        };
        let frozen = FrozenRun::new(
            &self.run_id,
            binding,
            &self.policy.obligations(),
            &self.started,
            &self.finished,
        )?;
        let projection = project_ruff(&self.evidence, mapping, contract, subject)?;
        let output = frozen.complete(projection)?;
        let provenance_bytes = serde_json::to_vec(&self.provenance)
            .map_err(|_| "candidate artifact serialization failed")?;
        let mut envelope = output.envelope().clone();
        envelope
            .artifacts
            .domain
            .push(guardengine::integration::ArtifactRef {
                uri: format!("artifact://{}/candidate.json", self.run_id),
                digest: format!("sha256:{:x}", Sha256::digest(&provenance_bytes)),
                media_type: "application/json".into(),
            });
        envelope
            .validate(guardengine::integration::EvidenceProfile::EngineBacked)
            .map_err(|_| "candidate envelope invalid")?;
        Ok(CandidateProjection {
            envelope,
            provenance_bytes,
            output,
            provenance: self.provenance.clone(),
        })
    }
}
impl CandidateProjection {
    pub fn envelope(&self) -> &guardengine::integration::GuardRunEnvelope {
        &self.envelope
    }
    pub fn contract_bytes(&self) -> Option<&[u8]> {
        self.output.contract_bytes()
    }
    pub fn facts_bytes(&self) -> Option<&[u8]> {
        self.output.facts_bytes()
    }
    pub fn report_bytes(&self) -> Option<&[u8]> {
        self.output.report_bytes()
    }
    pub fn domain_bytes(&self) -> &[u8] {
        self.output.domain_bytes()
    }
    pub fn provenance_bytes(&self) -> &[u8] {
        &self.provenance_bytes
    }
    pub fn provenance(&self) -> &serde_json::Value {
        &self.provenance
    }
}

pub mod transport;
