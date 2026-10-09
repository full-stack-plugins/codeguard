//! Explicit private Linux SDK artifact storage; consistency is not authority or eligibility.
pub mod access;
pub mod attempts;
use super::{
    consumer::{budget_output, serialized_size},
    envelope::{ApprovalAttachment, EnvelopeOutput},
};
use codeguard_runtime::private_artifact_store::{
    ArtifactInput, ArtifactLimit, MAX_BUNDLE_BYTES, PrivateArtifactStore, StagedArtifacts,
    StoreError, StoreLimits,
};
use guardengine::integration::{
    EvidenceProfile, GuardRunEnvelope, RunStatus, verify_engine_artifacts,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
const MANIFEST_LIMIT: usize = 16 * 1024;
#[derive(Debug)]
pub enum AuditError {
    InvalidEvidence,
    Budget,
    InvalidReceipt,
    Corrupt,
    Storage(StoreError),
}
impl From<StoreError> for AuditError {
    fn from(error: StoreError) -> Self {
        Self::Storage(error)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    version: String,
    key: String,
    manifest_digest: String,
}
impl Receipt {
    pub fn key(&self) -> &str {
        &self.key
    }
    /// A receipt is consistency metadata, never an access grant or producer authentication.
    pub fn from_json(bytes: &[u8]) -> Result<Self, AuditError> {
        if bytes.len() > 2048 {
            return Err(AuditError::Budget);
        }
        let value =
            codeguard_adapters::parse_unique_json(bytes).map_err(|_| AuditError::InvalidReceipt)?;
        let receipt: Self =
            serde_json::from_value(value).map_err(|_| AuditError::InvalidReceipt)?;
        if receipt.version != "codeguard.audit-receipt/v1alpha1"
            || !hex(&receipt.key)
            || !valid_digest(&receipt.manifest_digest)
        {
            return Err(AuditError::InvalidReceipt);
        }
        Ok(receipt)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    size: u64,
    digest: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: String,
    mapping_digest: Option<String>,
    files: BTreeMap<String, Blob>,
}
pub enum AuditEvidence<'a> {
    Output(&'a EnvelopeOutput),
    Attached(&'a ApprovalAttachment<'a>),
}
pub struct AuditStore {
    store: PrivateArtifactStore,
    limits: StoreLimits,
}
pub struct StagedAudit {
    stage: StagedArtifacts,
    receipt: Receipt,
}
impl StagedAudit {
    /// Retain before publication to reconcile a PublicationIndeterminate storage error.
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
    pub fn publish(self) -> Result<Receipt, AuditError> {
        self.stage.publish()?;
        Ok(self.receipt)
    }
}
fn hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_digest(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(hex)
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn identity(envelope_digest: &str, mapping: Option<&str>) -> String {
    let mut hash = Sha256::new();
    hash.update(b"codeguard.audit-key/v1alpha1\0");
    hash.update(envelope_digest.as_bytes());
    hash.update(b"\0");
    hash.update(mapping.unwrap_or("").as_bytes());
    format!("{:x}", hash.finalize())
}
fn validate(
    envelope: &GuardRunEnvelope,
    mapping: Option<&str>,
    native: &[u8],
    contract: Option<&[u8]>,
    facts: Option<&[u8]>,
    report: Option<&[u8]>,
) -> Result<(), AuditError> {
    if envelope.producer.guard != "codeguard"
        || envelope.producer.version != "0.1.4"
        || envelope.producer.analyzer_id != "codeguard.native-projection"
        || envelope.producer.analyzer_version != "0.1.0"
    {
        return Err(AuditError::InvalidEvidence);
    }
    match (contract, facts, report) {
        (Some(contract), Some(facts), Some(report)) => {
            if !mapping.is_some_and(valid_digest) {
                return Err(AuditError::InvalidEvidence);
            }
            verify_engine_artifacts(envelope, contract, facts, report)
                .map_err(|_| AuditError::InvalidEvidence)?;
        }
        (None, None, None) => {
            if mapping.is_some()
                || envelope.run_status == RunStatus::Completed
                || envelope.artifacts.contract.is_some()
                || envelope.artifacts.facts.is_some()
                || envelope.artifacts.report.is_some()
            {
                return Err(AuditError::InvalidEvidence);
            }
            envelope
                .validate(EvidenceProfile::NativeOnly)
                .map_err(|_| AuditError::InvalidEvidence)?;
        }
        _ => return Err(AuditError::InvalidEvidence),
    }
    if native.is_empty() {
        if !envelope.artifacts.domain.is_empty() {
            return Err(AuditError::InvalidEvidence);
        }
    } else if envelope.artifacts.domain.len() != 1
        || envelope.artifacts.domain[0].digest != digest(native)
        || envelope.artifacts.domain[0].uri != format!("artifact://{}/native.json", envelope.run_id)
    {
        return Err(AuditError::InvalidEvidence);
    }
    Ok(())
}
impl AuditStore {
    /// Explicit already-existing current-owner 0700 directory. No default path or native call.
    pub fn open(root: &Path, limits: StoreLimits) -> Result<Self, AuditError> {
        Ok(Self {
            store: PrivateArtifactStore::open(root, limits)?,
            limits,
        })
    }
    pub fn stage(&self, evidence: AuditEvidence<'_>) -> Result<StagedAudit, AuditError> {
        let (output, envelope) = match evidence {
            AuditEvidence::Output(output) => (output, output.envelope()),
            AuditEvidence::Attached(attached) => (attached.original, attached.envelope()),
        };
        budget_output(output, envelope).map_err(|_| AuditError::Budget)?;
        let envelope_size = serialized_size(envelope).map_err(|_| AuditError::Budget)?;
        let mut total = envelope_size + output.domain_bytes().len() + MANIFEST_LIMIT;
        for bytes in [
            output.contract_bytes(),
            output.facts_bytes(),
            output.report_bytes(),
        ]
        .into_iter()
        .flatten()
        {
            total = total.checked_add(bytes.len()).ok_or(AuditError::Budget)?;
        }
        if total as u64 > MAX_BUNDLE_BYTES || total as u64 > self.limits.max_total_bytes {
            return Err(AuditError::Budget);
        }
        validate(
            envelope,
            output.mapping_digest(),
            output.domain_bytes(),
            output.contract_bytes(),
            output.facts_bytes(),
            output.report_bytes(),
        )?;
        let encoded = serde_json::to_vec(envelope).map_err(|_| AuditError::InvalidEvidence)?;
        let mut inputs = vec![
            ArtifactInput {
                name: "envelope.json",
                bytes: &encoded,
            },
            ArtifactInput {
                name: "native.json",
                bytes: output.domain_bytes(),
            },
        ];
        for (name, bytes) in [
            ("contract.json", output.contract_bytes()),
            ("facts.json", output.facts_bytes()),
            ("report.json", output.report_bytes()),
        ] {
            if let Some(bytes) = bytes {
                inputs.push(ArtifactInput { name, bytes });
            }
        }
        let files = inputs
            .iter()
            .map(|input| {
                (
                    input.name.to_owned(),
                    Blob {
                        size: input.bytes.len() as u64,
                        digest: digest(input.bytes),
                    },
                )
            })
            .collect();
        let manifest = Manifest {
            version: "codeguard.audit-manifest/v1alpha1".into(),
            mapping_digest: output.mapping_digest().map(str::to_owned),
            files,
        };
        let manifest_bytes =
            serde_json::to_vec(&manifest).map_err(|_| AuditError::InvalidEvidence)?;
        if manifest_bytes.len() > MANIFEST_LIMIT {
            return Err(AuditError::Budget);
        }
        let key = identity(&digest(&encoded), output.mapping_digest());
        let receipt = Receipt {
            version: "codeguard.audit-receipt/v1alpha1".into(),
            key: key.clone(),
            manifest_digest: digest(&manifest_bytes),
        };
        inputs.push(ArtifactInput {
            name: "manifest.json",
            bytes: &manifest_bytes,
        });
        Ok(StagedAudit {
            stage: self.store.stage(&key, &inputs)?,
            receipt,
        })
    }
    /// Recheck every required original blob. Returned evidence still needs independent expectations
    /// and fresh authority through consumer::consume; loading never grants eligibility.
    pub fn read(&self, receipt: &Receipt) -> Result<EnvelopeOutput, AuditError> {
        if receipt.version != "codeguard.audit-receipt/v1alpha1"
            || !hex(&receipt.key)
            || !valid_digest(&receipt.manifest_digest)
        {
            return Err(AuditError::InvalidReceipt);
        }
        let limits = [
            ArtifactLimit {
                name: "manifest.json",
                max_bytes: MANIFEST_LIMIT as u64,
            },
            ArtifactLimit {
                name: "envelope.json",
                max_bytes: 1_048_576,
            },
            ArtifactLimit {
                name: "native.json",
                max_bytes: 1_048_576,
            },
            ArtifactLimit {
                name: "contract.json",
                max_bytes: 16_777_216,
            },
            ArtifactLimit {
                name: "facts.json",
                max_bytes: 16_777_216,
            },
            ArtifactLimit {
                name: "report.json",
                max_bytes: 16_777_216,
            },
        ];
        let mut files = self.store.read_with_limits(&receipt.key, &limits)?;
        let bytes = files.remove("manifest.json").ok_or(AuditError::Corrupt)?;
        if digest(&bytes) != receipt.manifest_digest {
            return Err(AuditError::Corrupt);
        }
        let value =
            codeguard_adapters::parse_unique_json(&bytes).map_err(|_| AuditError::Corrupt)?;
        let manifest: Manifest = serde_json::from_value(value).map_err(|_| AuditError::Corrupt)?;
        if manifest.version != "codeguard.audit-manifest/v1alpha1"
            || manifest.files.len() != files.len()
        {
            return Err(AuditError::Corrupt);
        }
        for (name, blob) in &manifest.files {
            let bytes = files.get(name).ok_or(AuditError::Corrupt)?;
            if blob.size != bytes.len() as u64 || blob.digest != digest(bytes) {
                return Err(AuditError::Corrupt);
            }
        }
        let encoded = files.remove("envelope.json").ok_or(AuditError::Corrupt)?;
        if identity(&digest(&encoded), manifest.mapping_digest.as_deref()) != receipt.key {
            return Err(AuditError::Corrupt);
        }
        let value =
            codeguard_adapters::parse_unique_json(&encoded).map_err(|_| AuditError::Corrupt)?;
        let envelope: GuardRunEnvelope =
            serde_json::from_value(value).map_err(|_| AuditError::Corrupt)?;
        let native = files.remove("native.json").ok_or(AuditError::Corrupt)?;
        let contract = files.remove("contract.json");
        let facts = files.remove("facts.json");
        let report = files.remove("report.json");
        if !files.is_empty() {
            return Err(AuditError::Corrupt);
        }
        validate(
            &envelope,
            manifest.mapping_digest.as_deref(),
            &native,
            contract.as_deref(),
            facts.as_deref(),
            report.as_deref(),
        )?;
        Ok(EnvelopeOutput::restore_verified(
            envelope,
            manifest.mapping_digest,
            contract,
            facts,
            report,
            native,
        ))
    }
}
