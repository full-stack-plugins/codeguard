//! Memory-only publication after structural binding; controller authentication is separate.
use super::{projection::Projection, reader::NativeEvidence, scope::FrozenObligations};
use guardengine::integration::{
    ArtifactRef, Artifacts, Coverage, CoverageStatus, Diagnostic, EvidenceProfile,
    GuardRunEnvelope, INTEGRATION_VERSION, Producer, RunBinding, RunStatus,
    verify_engine_artifacts,
};
use sha2::{Digest, Sha256};

pub struct FrozenRun {
    required_targets: std::collections::BTreeMap<String, Vec<String>>,
    template: GuardRunEnvelope,
}
pub struct EnvelopeOutput {
    envelope: GuardRunEnvelope,
    contract: Option<Vec<u8>>,
    facts: Option<Vec<u8>>,
    report: Option<Vec<u8>>,
    domain: Vec<u8>,
}
impl EnvelopeOutput {
    pub fn envelope(&self) -> &GuardRunEnvelope {
        &self.envelope
    }
    pub fn contract_bytes(&self) -> Option<&[u8]> {
        self.contract.as_deref()
    }
    pub fn facts_bytes(&self) -> Option<&[u8]> {
        self.facts.as_deref()
    }
    pub fn report_bytes(&self) -> Option<&[u8]> {
        self.report.as_deref()
    }
    pub fn domain_bytes(&self) -> &[u8] {
        &self.domain
    }
}

fn reference(run: &str, name: &str, bytes: &[u8]) -> ArtifactRef {
    ArtifactRef {
        uri: format!("artifact://{run}/{name}"),
        digest: format!("sha256:{:x}", Sha256::digest(bytes)),
        media_type: "application/json".into(),
    }
}
fn diagnostic(code: &str) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        message: code.into(),
        retryable: false,
        source: None,
    }
}
impl FrozenRun {
    /// Validate structural caller binding; this does not resolve Git objects or authenticate policy.
    pub fn new(
        run_id: &str,
        binding: RunBinding,
        obligations: &FrozenObligations,
        started: &str,
        finished: &str,
    ) -> Result<Self, &'static str> {
        if run_id.is_empty()
            || run_id.len() > 128
            || !run_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err("invalid run ID");
        }
        let required = obligations.scope_ids();
        let template = GuardRunEnvelope {
            api_version: INTEGRATION_VERSION.into(),
            kind: "GuardRunEnvelope".into(),
            run_id: run_id.into(),
            producer: Producer {
                guard: "codeguard".into(),
                version: "0.1.4".into(),
                analyzer_id: "codeguard.native-projection".into(),
                analyzer_version: "0.1.0".into(),
            },
            binding,
            run_status: RunStatus::Error,
            decision: None,
            coverage: Coverage {
                status: CoverageStatus::Partial,
                required_scopes: required.clone(),
                observed_scopes: vec![],
                missing_scopes: required,
            },
            artifacts: Artifacts {
                contract: None,
                facts: None,
                report: None,
                domain: vec![],
            },
            approval_refs: vec![],
            diagnostics: vec![diagnostic("unpublished")],
            started_at: started.into(),
            finished_at: finished.into(),
            expires_at: None,
        };
        template
            .validate(EvidenceProfile::NativeOnly)
            .map_err(|_| "binding preconditions failed")?;
        Ok(Self {
            template,
            required_targets: obligations.frozen_targets(),
        })
    }
    pub fn complete(&self, projection: Projection) -> Result<EnvelopeOutput, &'static str> {
        if projection.required_targets != self.required_targets
            || projection.native_run_id != self.template.run_id
            || projection.required_scopes != self.template.coverage.required_scopes
            || projection.facts.subject.id != self.template.binding.repo_id
            || projection.facts.subject.snapshot_digest
                != self.template.binding.source_snapshot_digest
        {
            return Err("projection differs from frozen binding");
        }
        let facts = serde_json::to_vec(&projection.facts).map_err(|_| "cannot serialize facts")?;
        let report =
            serde_json::to_vec(&projection.report).map_err(|_| "cannot serialize report")?;
        let mut envelope = self.template.clone();
        envelope.run_status = RunStatus::Completed;
        envelope.decision = Some(projection.report.decision.clone());
        if projection.facts.completeness == guardengine::Completeness::Complete {
            // Only a sealed, independently qualified profile projection can
            // construct Complete facts; caller declarations cannot select this.
            envelope.coverage.status = CoverageStatus::Complete;
            envelope.coverage.observed_scopes = envelope.coverage.required_scopes.clone();
            envelope.coverage.missing_scopes.clear();
            envelope.diagnostics = vec![diagnostic("qualified_scope_complete")];
        } else {
            envelope.diagnostics = vec![diagnostic("native_profile_unqualified")];
        }
        envelope.artifacts = Artifacts {
            contract: Some(reference(
                &envelope.run_id,
                "contract.json",
                &projection.contract_bytes,
            )),
            facts: Some(reference(&envelope.run_id, "facts.json", &facts)),
            report: Some(reference(&envelope.run_id, "report.json", &report)),
            domain: vec![reference(
                &envelope.run_id,
                "native.json",
                &projection.domain,
            )],
        };
        verify_engine_artifacts(&envelope, &projection.contract_bytes, &facts, &report)
            .map_err(|_| "engine artifact verification failed")?;
        Ok(EnvelopeOutput {
            envelope,
            contract: Some(projection.contract_bytes),
            facts: Some(facts),
            report: Some(report),
            domain: projection.domain,
        })
    }
    /// Ruff transport failure retains only the sealed evidence for this frozen scope.
    #[cfg(unix)]
    pub(super) fn ruff_error(
        &self,
        code: &'static str,
        evidence: Option<&super::ruff_profile::RuffEvidence>,
    ) -> EnvelopeOutput {
        let mut output = self.error(code, None);
        if let Some(evidence) = evidence.filter(|e| {
            e.matches_binding(
                &self.template.run_id,
                &self.template.binding.source_snapshot_digest,
                &self.required_targets,
            )
        }) {
            output.domain = evidence.raw_bytes().to_vec();
            output.envelope.artifacts.domain = vec![reference(
                &self.template.run_id,
                "native.json",
                &output.domain,
            )];
        }
        output
    }
    #[cfg(unix)]
    pub(super) fn ruff_cancelled(
        &self,
        evidence: Option<&super::ruff_profile::RuffEvidence>,
    ) -> EnvelopeOutput {
        let mut output = self.ruff_error("adapter_cancelled", evidence);
        output.envelope.run_status = RunStatus::Cancelled;
        output
    }
    /// Bound adapter failure. Attach only evidence matching this run and snapshot.
    #[cfg(unix)]
    pub(crate) fn error(
        &self,
        code: &'static str,
        evidence: Option<&NativeEvidence>,
    ) -> EnvelopeOutput {
        let mut envelope = self.template.clone();
        envelope.diagnostics = vec![diagnostic(code)];
        let domain = evidence
            .filter(|e| {
                e.report().run_id == envelope.run_id
                    && e.report().document()["identities"]["content"]["digest"]
                        .as_str()
                        .is_some_and(|digest| {
                            envelope.binding.source_snapshot_digest == format!("sha256:{digest}")
                        })
            })
            .map_or_else(Vec::new, |e| e.raw_bytes().to_vec());
        if !domain.is_empty() {
            envelope.artifacts.domain = vec![reference(&envelope.run_id, "native.json", &domain)];
        }
        EnvelopeOutput {
            envelope,
            contract: None,
            facts: None,
            report: None,
            domain,
        }
    }
    #[cfg(unix)]
    pub(crate) fn cancelled(&self, evidence: Option<&NativeEvidence>) -> EnvelopeOutput {
        let mut output = self.error("adapter_cancelled", evidence);
        output.envelope.run_status = RunStatus::Cancelled;
        output
    }
    pub fn failure(&self, evidence: &NativeEvidence) -> Result<EnvelopeOutput, &'static str> {
        if evidence.report().run_id != self.template.run_id
            || format!(
                "sha256:{}",
                evidence.report().document()["identities"]["content"]["digest"]
                    .as_str()
                    .ok_or("missing snapshot")?
            ) != self.template.binding.source_snapshot_digest
        {
            return Err("native failure differs from frozen binding");
        }
        let (status, code) = match evidence.report().exit_code {
            4 => (RunStatus::Error, "native_execution_error"),
            130 => (RunStatus::Cancelled, "native_cancelled"),
            _ => return Err("native result is not execution failure"),
        };
        let mut envelope = self.template.clone();
        envelope.run_status = status;
        envelope.diagnostics = vec![diagnostic(code)];
        envelope.artifacts.domain = vec![reference(
            &envelope.run_id,
            "native.json",
            evidence.raw_bytes(),
        )];
        envelope
            .validate(EvidenceProfile::NativeOnly)
            .map_err(|_| "failed envelope validation")?;
        Ok(EnvelopeOutput {
            envelope,
            contract: None,
            facts: None,
            report: None,
            domain: evidence.raw_bytes().to_vec(),
        })
    }
}
