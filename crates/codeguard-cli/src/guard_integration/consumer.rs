//! Read-only SDK consumption. Actual Git candidate/provenance validation is a host responsibility.
use super::envelope::EnvelopeOutput;
use guardengine::integration::eligibility::{
    ArtifactBytes, AuthorityProvider, EligibilityPolicy, EligibilityResult, evaluate_eligibility,
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
fn bounded(value: &impl Serialize) -> Result<(), &'static str> {
    serde_json::to_writer(Budget(1_048_576), value).map_err(|_| "consumer metadata budget exceeded")
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
    let envelope = output.envelope();
    // All borrowed metadata and bytes are bounded before GE hashes/serializes anything.
    bounded(envelope)?;
    bounded(expected)?;
    if cause.is_some_and(|s| s.len() > 16_384) || output.domain_bytes().len() > 1_048_576 {
        return Err("consumer input budget exceeded");
    }
    let (contract, facts, report) = (
        output.contract_bytes(),
        output.facts_bytes(),
        output.report_bytes(),
    );
    for bytes in [contract, facts, report].into_iter().flatten() {
        if bytes.len() > guardengine::integration::MAX_ARTIFACT_BYTES {
            return Err("consumer artifact budget exceeded");
        }
    }
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
