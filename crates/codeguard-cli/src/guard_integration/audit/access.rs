//! Explicit retained-evidence access for a trusted local controller.
//! Capabilities isolate delegated API sessions, not hostile processes sharing the owner's UID.
use super::{AuditError, AuditEvidence, AuditStore, Receipt, digest, hex};
use crate::guard_integration::{
    consumer::{self, Consumption, ExpectedConsumption},
    envelope::EnvelopeOutput,
};
use codeguard_runtime::private_artifact_store::{PrivateArtifactStore, RootLease, StoreLimits};
use guardengine::{
    Decision,
    integration::{RunStatus, eligibility::AuthorityProvider},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, sync::Arc};
const CONTROL: &str = ".audit-retention-v1.json";
const MARKER: &str = ".audit-initialized-v1";
const MARKER_BYTES: &[u8] = b"codeguard.audit-initialized/v1alpha1";
const CONTROL_LIMIT: u64 = 128 * 1024;
const VERSION: &str = "codeguard.audit-retention/v1alpha1";
#[derive(Debug)]
pub enum AccessError {
    Permission,
    InvalidInput,
    ClockRollback,
    Expired,
    Purged,
    Missing,
    Corrupt,
    Budget,
    Evidence(AuditError),
    Consumption,
}
impl From<AuditError> for AccessError {
    fn from(value: AuditError) -> Self {
        Self::Evidence(value)
    }
}
impl From<codeguard_runtime::private_artifact_store::StoreError> for AccessError {
    fn from(value: codeguard_runtime::private_artifact_store::StoreError) -> Self {
        Self::Evidence(AuditError::Storage(value))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionPolicy {
    pub max_age_seconds: u64,
}
// No serialization, constructors, mutable fields or receipt-to-capability conversion.
pub struct Administration(Arc<()>);
pub struct ViewAccess(Arc<()>);
/// Raw access is separate from a public-view capability.
/// ```compile_fail
/// use codeguard_cli::guard_integration::audit::{Receipt, access::{AuditAccess, ViewAccess}};
/// fn forbidden(store: &AuditAccess, view: &ViewAccess, receipt: &Receipt) {
///     store.read_raw(view, receipt, 100);
/// }
/// ```
/// Serialized input cannot create a raw grant.
/// ```compile_fail
/// use codeguard_cli::guard_integration::audit::access::RawAccess;
/// let forged: RawAccess = serde_json::from_str("{}").unwrap();
/// ```
pub struct RawAccess(Arc<()>);
pub struct AuditAccess {
    audit: AuditStore,
    controls: PrivateArtifactStore,
    session: Arc<()>,
    namespace: String,
    policy: RetentionPolicy,
    limits: StoreLimits,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    receipt_digest: String,
    created_at: i64,
    expires_at: i64,
    purged: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    version: String,
    namespace_digest: String,
    policy: RetentionPolicy,
    max_entries: usize,
    max_total_bytes: u64,
    watermark: i64,
    records: BTreeMap<String, Record>,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Unavailable,
}
/// Only fixed enums and a constant version are exposed. No arbitrary producer text is public.
#[derive(Serialize)]
pub struct PublicView {
    version: &'static str,
    availability: Availability,
    run_status: Option<RunStatus>,
    decision: Option<Decision>,
}
fn receipt_digest(receipt: &Receipt) -> Result<String, AccessError> {
    // Receipt also implements Deserialize: reject oversized forged fields while borrowed,
    // before serialization, hashing or allocating a copy.
    if receipt.version != "codeguard.audit-receipt/v1alpha1"
        || !hex(&receipt.key)
        || !super::valid_digest(&receipt.manifest_digest)
    {
        return Err(AccessError::Corrupt);
    }
    let encoded = serde_json::to_vec(receipt).map_err(|_| AccessError::Corrupt)?;
    Receipt::from_json(&encoded)?;
    Ok(digest(&encoded))
}
impl AuditAccess {
    /// Explicit private namespace. `now` in subsequent methods must come from the protected
    /// controller clock, never an untrusted request parameter. Opening does not execute tools.
    pub fn open(
        root: &Path,
        limits: StoreLimits,
        namespace: &str,
        policy: RetentionPolicy,
    ) -> Result<(Self, Administration), AccessError> {
        if namespace.is_empty()
            || namespace.len() > 128
            || namespace.chars().any(char::is_control)
            || policy.max_age_seconds == 0
            || policy.max_age_seconds > 366 * 86400
            || limits.max_entries < 4
            || limits.max_total_bytes <= CONTROL_LIMIT * 2
        {
            return Err(AccessError::InvalidInput);
        }
        let controls = PrivateArtifactStore::open(root, limits)?;
        // Reserve a staging slot and a full control replacement's peak bytes. All physical
        // control/orphan files still count in the runtime's actual root usage calculation.
        let audit = AuditStore::open(
            root,
            StoreLimits {
                max_entries: limits.max_entries - 1,
                max_total_bytes: limits.max_total_bytes - CONTROL_LIMIT,
            },
        )?;
        let session = Arc::new(());
        let result = Self {
            audit,
            controls,
            session: session.clone(),
            namespace: digest(namespace.as_bytes()),
            policy,
            limits,
        };
        {
            let lease = result.controls.lease()?;
            let marker = lease.read_control(MARKER, 64)?;
            let control = lease.read_control(CONTROL, CONTROL_LIMIT)?;
            if marker.is_none() && control.is_none() {
                if !lease.is_empty()? {
                    return Err(AccessError::Corrupt);
                }
                // Marker precedes checkpoint. Interrupted initialization fails closed, and loss
                // of a checkpoint after all bundles were purged cannot silently reset the clock.
                lease.replace_control(MARKER, MARKER_BYTES, 64)?;
                let initial = State {
                    version: VERSION.into(),
                    namespace_digest: result.namespace.clone(),
                    policy,
                    max_entries: limits.max_entries,
                    max_total_bytes: limits.max_total_bytes,
                    watermark: 0,
                    records: BTreeMap::new(),
                };
                result.save(&lease, &initial)?;
            }
            if lease.read_control(MARKER, 64)?.as_deref() != Some(MARKER_BYTES) {
                return Err(AccessError::Corrupt);
            }
            result.state(&lease)?;
        }
        Ok((result, Administration(session)))
    }
    fn authorize(&self, capability: &Arc<()>) -> Result<(), AccessError> {
        if Arc::ptr_eq(capability, &self.session) {
            Ok(())
        } else {
            Err(AccessError::Permission)
        }
    }
    pub fn view_access(&self, administration: &Administration) -> Result<ViewAccess, AccessError> {
        self.authorize(&administration.0)?;
        Ok(ViewAccess(self.session.clone()))
    }
    pub fn raw_access(&self, administration: &Administration) -> Result<RawAccess, AccessError> {
        self.authorize(&administration.0)?;
        Ok(RawAccess(self.session.clone()))
    }
    fn state(&self, lease: &RootLease<'_>) -> Result<State, AccessError> {
        if lease.read_control(MARKER, 64)?.as_deref() != Some(MARKER_BYTES) {
            return Err(AccessError::Corrupt);
        }
        let bytes = lease
            .read_control(CONTROL, CONTROL_LIMIT)?
            .ok_or(AccessError::Corrupt)?;
        let value =
            codeguard_adapters::parse_unique_json(&bytes).map_err(|_| AccessError::Corrupt)?;
        let state: State = serde_json::from_value(value).map_err(|_| AccessError::Corrupt)?;
        if state.version != VERSION
            || state.namespace_digest != self.namespace
            || state.policy != self.policy
            || state.max_entries != self.limits.max_entries
            || state.max_total_bytes != self.limits.max_total_bytes
            || state.watermark < 0
            || state.records.len() > 256
        {
            return Err(AccessError::Corrupt);
        }
        for (key, record) in &state.records {
            if !hex(key)
                || !super::valid_digest(&record.receipt_digest)
                || record.created_at < 0
                || record.created_at > state.watermark
                || record.expires_at <= record.created_at
                || record
                    .expires_at
                    .checked_sub(record.created_at)
                    .is_none_or(|n| n as u64 > self.policy.max_age_seconds)
            {
                return Err(AccessError::Corrupt);
            }
        }
        Ok(state)
    }
    fn save(&self, lease: &RootLease<'_>, state: &State) -> Result<(), AccessError> {
        // Stream budget precedes allocation of the encoded checkpoint.
        let size = consumer::serialized_size(state).map_err(|_| AccessError::Budget)?;
        if size as u64 > CONTROL_LIMIT {
            return Err(AccessError::Budget);
        }
        let bytes = serde_json::to_vec(state).map_err(|_| AccessError::Corrupt)?;
        lease.replace_control(CONTROL, &bytes, CONTROL_LIMIT)?;
        Ok(())
    }
    fn observe(&self, lease: &RootLease<'_>, now: i64) -> Result<State, AccessError> {
        let mut state = self.state(lease)?;
        if now < state.watermark || now < 0 {
            return Err(AccessError::ClockRollback);
        }
        if now > state.watermark {
            state.watermark = now;
            self.save(lease, &state)?;
        }
        Ok(state)
    }
    /// Immutable payload publication followed by retention registration. On registration failure,
    /// payload is an inaccessible counted orphan, never automatically eligible evidence.
    pub fn publish(
        &self,
        administration: &Administration,
        evidence: AuditEvidence<'_>,
        now: i64,
        retain_seconds: u64,
    ) -> Result<Receipt, AccessError> {
        self.authorize(&administration.0)?;
        if now < 0 || retain_seconds == 0 || retain_seconds > self.policy.max_age_seconds {
            return Err(AccessError::InvalidInput);
        }
        let expires_at = now
            .checked_add(retain_seconds as i64)
            .ok_or(AccessError::InvalidInput)?;
        {
            let lease = self.controls.lease()?;
            let state = self.observe(&lease, now)?;
            if state.records.len() >= 256 {
                return Err(AccessError::Budget);
            }
        }
        let receipt = self.audit.stage(evidence)?.publish()?;
        let lease = self.controls.lease()?;
        let mut state = self.observe(&lease, now)?;
        if state.records.len() >= 256 || state.records.contains_key(receipt.key()) {
            return Err(AccessError::Budget);
        }
        state.records.insert(
            receipt.key().into(),
            Record {
                receipt_digest: receipt_digest(&receipt)?,
                created_at: now,
                expires_at,
                purged: false,
            },
        );
        self.save(&lease, &state)?;
        Ok(receipt)
    }
    fn load(
        &self,
        lease: &RootLease<'_>,
        receipt: &Receipt,
        now: i64,
    ) -> Result<EnvelopeOutput, AccessError> {
        let expected = receipt_digest(receipt)?;
        let state = self.observe(lease, now)?;
        let record = state
            .records
            .get(receipt.key())
            .ok_or(AccessError::Missing)?;
        if record.receipt_digest != expected {
            return Err(AccessError::Corrupt);
        }
        if record.purged {
            return Err(AccessError::Purged);
        }
        if now >= record.expires_at {
            return Err(AccessError::Expired);
        }
        Ok(self.audit.read(receipt)?)
    }
    pub fn read_raw(
        &self,
        access: &RawAccess,
        receipt: &Receipt,
        now: i64,
    ) -> Result<EnvelopeOutput, AccessError> {
        self.authorize(&access.0)?;
        let lease = self.controls.lease()?;
        self.load(&lease, receipt, now)
    }
    pub fn view(
        &self,
        access: &ViewAccess,
        receipt: &Receipt,
        now: i64,
    ) -> Result<PublicView, AccessError> {
        self.authorize(&access.0)?;
        let lease = self.controls.lease()?;
        match self.load(&lease, receipt, now) {
            Ok(output) => Ok(PublicView {
                version: "codeguard.audit-public/v1alpha1",
                availability: Availability::Available,
                run_status: Some(output.envelope().run_status.clone()),
                decision: output.envelope().decision.clone(),
            }),
            Err(AccessError::Expired | AccessError::Purged | AccessError::Missing) => {
                Ok(PublicView {
                    version: "codeguard.audit-public/v1alpha1",
                    availability: Availability::Unavailable,
                    run_status: None,
                    decision: None,
                })
            }
            Err(error) => Err(error),
        }
    }
    /// Fresh store-mediated eligibility. Lease spans validation and the provider query; no cache.
    /// This does not replace current GG tree/provenance verification by the candidate host.
    pub fn consume(
        &self,
        access: &RawAccess,
        receipt: &Receipt,
        expected: &ExpectedConsumption,
        provider: &dyn AuthorityProvider,
        now: i64,
        cause: Option<&str>,
    ) -> Result<Consumption, AccessError> {
        self.authorize(&access.0)?;
        consumer::serialized_size(expected).map_err(|_| AccessError::Budget)?;
        if cause.is_some_and(|text| text.len() > 16_384) {
            return Err(AccessError::Budget);
        }
        let lease = self.controls.lease()?;
        let output = self.load(&lease, receipt, now)?;
        consumer::consume(&output, expected, provider, now, cause)
            .map_err(|_| AccessError::Consumption)
    }
    /// Tombstone is durable before any leaf deletion. A deletion failure leaves it in force.
    pub fn purge(
        &self,
        administration: &Administration,
        receipt: &Receipt,
        now: i64,
    ) -> Result<(), AccessError> {
        self.authorize(&administration.0)?;
        let expected = receipt_digest(receipt)?;
        let lease = self.controls.lease()?;
        let mut state = self.observe(&lease, now)?;
        let record = state
            .records
            .get_mut(receipt.key())
            .ok_or(AccessError::Missing)?;
        if record.receipt_digest != expected {
            return Err(AccessError::Corrupt);
        }
        record.purged = true;
        self.save(&lease, &state)?;
        lease.purge(receipt.key())?;
        Ok(())
    }
}
