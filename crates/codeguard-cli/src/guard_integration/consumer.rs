//! Read-only SDK consumption. Actual Git candidate/provenance validation is a host responsibility.
use super::envelope::{ApprovalAttachment, EnvelopeOutput};
use guardengine::integration::GuardRunEnvelope;
use guardengine::integration::eligibility::{
    ArtifactBytes, AuthorityError, AuthorityProvider, EligibilityCode, EligibilityPolicy,
    EligibilityResult, evaluate_eligibility, validate_approval_record,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Write;

/// Protected controller expectations, independently retained from projection creation.
#[derive(Clone, Serialize)]
pub struct ExpectedConsumption {
    pub policy: EligibilityPolicy,
    pub run_id: String,
    pub mapping_digest: String,
    pub envelope_digest: String,
    pub raw_domain_digest: String,
}
pub struct Consumption {
    /// Observation identity, never a cache authorization token.
    pub freshness_key: String,
    pub eligibility: EligibilityResult,
}
/// Independently selected baseline authentication, separate from technical review.
/// The reference is opaque: this module never fetches its URI or issues approval.
#[derive(Clone, Serialize)]
pub struct BaselineRequirement {
    pub reference: String,
    pub purpose: String,
    pub policy: EligibilityPolicy,
}
/// Owned controller expectations; submitted evidence cannot replace this context.
pub struct PreparedConsumption {
    expected: ExpectedConsumption,
    baseline: Option<BaselineRequirement>,
}
impl PreparedConsumption {
    pub(super) fn expected(&self) -> &ExpectedConsumption {
        &self.expected
    }
    pub fn freeze(
        expected: &ExpectedConsumption,
        baseline: Option<&BaselineRequirement>,
    ) -> Result<Self, &'static str> {
        serialized_size(&(expected, baseline))?;
        match (&expected.policy.binding.baseline_digest, baseline) {
            (None, None) => {}
            (Some(digest), Some(b)) => {
                let valid_text = |s: &str| {
                    !s.trim().is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
                };
                if !digest.strip_prefix("sha256:").is_some_and(|s| {
                    s.len() == 64
                        && s.bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                }) || !valid_text(&b.reference)
                    || !valid_text(&b.purpose)
                    || b.policy.binding != expected.policy.binding
                    || b.policy.contract_digest != expected.policy.contract_digest
                    || b.policy.producer != expected.policy.producer
                    || b.policy.required_scopes != expected.policy.required_scopes
                    || b.policy.action != expected.policy.action
                    || !b
                        .policy
                        .approval_principals
                        .get(&b.purpose)
                        .is_some_and(|p| !p.is_empty())
                {
                    return Err("invalid frozen baseline expectation");
                }
            }
            _ => return Err("baseline requirement missing or extraneous"),
        }
        Ok(Self {
            expected: expected.clone(),
            baseline: baseline.cloned(),
        })
    }
    pub fn consume(
        &self,
        output: &EnvelopeOutput,
        provider: &dyn AuthorityProvider,
        now: i64,
        cause: Option<&str>,
    ) -> Result<Consumption, &'static str> {
        self.consume_view(output, output.envelope(), provider, now, cause)
    }
    pub fn consume_attached(
        &self,
        attachment: &ApprovalAttachment<'_>,
        provider: &dyn AuthorityProvider,
        now: i64,
        cause: Option<&str>,
    ) -> Result<Consumption, &'static str> {
        self.consume_view(
            attachment.original,
            attachment.envelope(),
            provider,
            now,
            cause,
        )
    }
    fn consume_view(
        &self,
        output: &EnvelopeOutput,
        envelope: &GuardRunEnvelope,
        provider: &dyn AuthorityProvider,
        now: i64,
        cause: Option<&str>,
    ) -> Result<Consumption, &'static str> {
        let mut consumed = consume_view(
            output,
            envelope,
            &self.expected,
            provider,
            now,
            cause,
            self.baseline.is_some(),
        )?;
        if let Some(b) = &self.baseline {
            let validation = provider
                .verify_approval(&b.reference)
                .map_err(|e| match e {
                    AuthorityError::Unavailable => EligibilityCode::ProviderUnavailable,
                    AuthorityError::Untrusted => EligibilityCode::InvalidApproval,
                })
                .and_then(|record| validate_approval_record(&record, &b.policy, &b.purpose, now));
            // Baseline approval can only withdraw eligibility, never repair technical failure.
            if consumed.eligibility.eligible {
                if let Err(code) = validation {
                    consumed.eligibility.eligible = false;
                    consumed.eligibility.code = code;
                    consumed.eligibility.audit.code = code;
                }
            }
        }
        consumed.freshness_key = digest(
            &serde_json::to_vec(&(
                "codeguard.baseline-consumption/v1",
                &self.expected,
                &self.baseline,
                &consumed.eligibility.audit,
            ))
            .map_err(|_| "invalid baseline freshness key")?,
        );
        Ok(consumed)
    }
}
struct Budget(usize);
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("metadata budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(super) fn serialized_size(value: &impl Serialize) -> Result<usize, &'static str> {
    let mut budget = Budget(1_048_576);
    serde_json::to_writer(&mut budget, value).map_err(|_| "consumer metadata budget exceeded")?;
    Ok(1_048_576 - budget.0)
}
pub(super) fn budget_output(
    output: &EnvelopeOutput,
    envelope: &GuardRunEnvelope,
) -> Result<(), &'static str> {
    serialized_size(envelope)?;
    if output.domain_bytes().len() > 1_048_576 {
        return Err("consumer domain budget exceeded");
    }
    for bytes in [
        output.contract_bytes(),
        output.facts_bytes(),
        output.report_bytes(),
    ]
    .into_iter()
    .flatten()
    {
        if bytes.len() > guardengine::integration::MAX_ARTIFACT_BYTES {
            return Err("consumer artifact budget exceeded");
        }
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// No cache, native process or I/O. The authority provider is an explicit caller port;
/// its freshness and side effects remain the caller/provider contract.
pub fn consume(
    output: &EnvelopeOutput,
    expected: &ExpectedConsumption,
    provider: &dyn AuthorityProvider,
    now: i64,
    cause: Option<&str>,
) -> Result<Consumption, &'static str> {
    consume_view(
        output,
        output.envelope(),
        expected,
        provider,
        now,
        cause,
        false,
    )
}

/// Consume a bounded controller attachment; new envelope authentication is required.
pub fn consume_attached(
    attachment: &ApprovalAttachment<'_>,
    expected: &ExpectedConsumption,
    provider: &dyn AuthorityProvider,
    now: i64,
    cause: Option<&str>,
) -> Result<Consumption, &'static str> {
    consume_view(
        attachment.original,
        attachment.envelope(),
        expected,
        provider,
        now,
        cause,
        false,
    )
}
#[allow(clippy::too_many_arguments)]
fn consume_view(
    output: &EnvelopeOutput,
    envelope: &GuardRunEnvelope,
    expected: &ExpectedConsumption,
    provider: &dyn AuthorityProvider,
    now: i64,
    cause: Option<&str>,
    baseline_checked: bool,
) -> Result<Consumption, &'static str> {
    budget_output(output, envelope)?;
    serialized_size(expected)?;
    if now < 0 {
        return Err("invalid controller time");
    }
    if envelope.binding.baseline_digest.is_some() && !baseline_checked {
        return Err("non-null baseline requires frozen authentication context");
    }
    if cause.is_some_and(|s| s.len() > 16_384) {
        return Err("consumer input budget exceeded");
    }
    let (contract, facts, report) = (
        output.contract_bytes(),
        output.facts_bytes(),
        output.report_bytes(),
    );
    let envelope_digest = digest(&serde_json::to_vec(envelope).map_err(|_| "invalid envelope")?);
    let raw_digest = digest(output.domain_bytes());
    if envelope.run_id != expected.run_id
        || envelope_digest != expected.envelope_digest
        || raw_digest != expected.raw_domain_digest
        || output.mapping_digest() != Some(expected.mapping_digest.as_str())
    {
        return Err("consumer expected identity changed");
    }
    if envelope.artifacts.domain.len() != 1 || envelope.artifacts.domain[0].digest != raw_digest {
        return Err("consumer domain artifact mismatch");
    }
    let artifacts = ArtifactBytes {
        contract: contract.ok_or("engine-backed contract required")?,
        facts: facts.ok_or("engine-backed facts required")?,
        report: report.ok_or("engine-backed report required")?,
    };
    let eligibility =
        evaluate_eligibility(envelope, artifacts, &expected.policy, provider, now, cause);
    let freshness_key = digest(
        &serde_json::to_vec(&(
            "codeguard.sdk-consumption/v1alpha1",
            expected,
            &eligibility.audit,
        ))
        .map_err(|_| "invalid freshness key")?,
    );
    Ok(Consumption {
        freshness_key,
        eligibility,
    })
}
