//! Bounded process-local attempt history. A current record is not cached eligibility.
use super::{
    Receipt,
    access::{AccessError, Administration, AuditAccess, RawAccess},
    digest, valid_digest,
};
use crate::guard_integration::{
    consumer::{self, Consumption, ExpectedConsumption},
    envelope::EnvelopeOutput,
};
pub use guardengine::integration::attempt_store::AppendOutcome as ImportOutcome;
use guardengine::integration::{
    Coverage, CoverageStatus, EvidenceProfile, InvocationDraft, Producer, RunBinding, RunStatus,
    attempt_store::{AttemptRecord, AttemptStore, InMemoryAttemptStore, StoreError, Target},
    eligibility::AuthorityProvider,
    prepare_attempt,
};
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};
const MAX_ATTEMPTS: usize = 256;
const MAX_WORK_BYTES: usize = 64 * 1024;
#[derive(Debug)]
pub enum AttemptError {
    Budget,
    InvalidInput,
    ForeignTicket,
    Binding,
    NotCurrent,
    Access(AccessError),
    Cas(StoreError),
}
impl From<AccessError> for AttemptError {
    fn from(error: AccessError) -> Self {
        Self::Access(error)
    }
}
impl From<StoreError> for AttemptError {
    fn from(error: StoreError) -> Self {
        Self::Cas(error)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttemptTarget {
    repo: String,
    task: String,
    worktree: String,
    requirements: Vec<String>,
}
impl AttemptTarget {
    fn engine(&self) -> Target {
        Target {
            repo_id: self.repo.clone(),
            task_id: self.task.clone(),
            requirement_ids: self.requirements.clone(),
        }
    }
}
/// Frozen controller input. No deserialization and no mutable/public binding fields.
pub struct AttemptWork {
    binding: RunBinding,
    producer: Producer,
    scopes: Vec<String>,
    contract_digest: String,
    mapping_digest: String,
    target: AttemptTarget,
    digest: String,
}
fn field(value: &str) -> Result<(), AttemptError> {
    if value.trim().is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
        Err(AttemptError::Budget)
    } else {
        Ok(())
    }
}
fn metadata(value: &impl Serialize) -> Result<(), AttemptError> {
    if consumer::serialized_size(value).map_err(|_| AttemptError::Budget)? > MAX_WORK_BYTES {
        Err(AttemptError::Budget)
    } else {
        Ok(())
    }
}
impl AttemptWork {
    pub fn freeze(
        binding: &RunBinding,
        producer: &Producer,
        required_scopes: &[String],
        contract_digest: &str,
        mapping_digest: &str,
    ) -> Result<Self, AttemptError> {
        if binding.requirement_ids.is_empty()
            || binding.requirement_ids.len() > 64
            || required_scopes.is_empty()
            || required_scopes.len() > 64
        {
            return Err(AttemptError::Budget);
        }
        for value in [
            &binding.repo_id,
            &binding.task_id,
            &binding.worktree_id,
            &binding.candidate_oid,
            &binding.base_oid,
            &binding.source_snapshot_digest,
            &producer.guard,
            &producer.version,
            &producer.analyzer_id,
            &producer.analyzer_version,
        ] {
            field(value)?;
        }
        for value in binding
            .requirement_ids
            .iter()
            .chain(required_scopes)
            .chain(binding.merge_group_id.iter())
            .chain(binding.baseline_digest.iter())
        {
            field(value)?;
        }
        if !valid_digest(contract_digest) || !valid_digest(mapping_digest) {
            return Err(AttemptError::InvalidInput);
        }
        let input = (
            binding,
            producer,
            required_scopes,
            contract_digest,
            mapping_digest,
        );
        metadata(&input)?;
        // Reuse GE structural validation after borrowed admission. This internal template is
        // never serialized as a run or used as evidence/producer authority.
        prepare_attempt(InvocationDraft {
            run_id: "codeguard-cas-validation".into(),
            producer: Some(producer.clone()),
            binding: Some(binding.clone()),
            coverage: Some(Coverage {
                status: CoverageStatus::Partial,
                required_scopes: required_scopes.to_vec(),
                observed_scopes: vec![],
                missing_scopes: required_scopes.to_vec(),
            }),
            profile: Some(EvidenceProfile::EngineBacked),
            started_at: "2000-01-01T00:00:00Z".into(),
        })
        .map_err(|_| AttemptError::InvalidInput)?;
        let digest = digest(&serde_json::to_vec(&input).map_err(|_| AttemptError::InvalidInput)?);
        Ok(Self {
            binding: binding.clone(),
            producer: producer.clone(),
            scopes: required_scopes.to_vec(),
            contract_digest: contract_digest.into(),
            mapping_digest: mapping_digest.into(),
            target: AttemptTarget {
                repo: binding.repo_id.clone(),
                task: binding.task_id.clone(),
                worktree: binding.worktree_id.clone(),
                requirements: binding.requirement_ids.clone(),
            },
            digest,
        })
    }
    pub fn target(&self) -> &AttemptTarget {
        &self.target
    }
}
/// A ticket can be retained/replayed but not constructed, mutated or deserialized by a caller.
/// ```compile_fail
/// use codeguard_cli::guard_integration::audit::attempts::AttemptTicket;
/// fn overwrite(ticket: &mut AttemptTicket) { ticket.generation = 999; }
/// ```
/// ```compile_fail
/// use codeguard_cli::guard_integration::audit::attempts::AttemptTicket;
/// let forged: AttemptTicket = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone)]
pub struct AttemptTicket {
    owner: Arc<()>,
    run_id: String,
    generation: u64,
    target: AttemptTarget,
    work_digest: String,
}
impl AttemptTicket {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}
struct Registration {
    work: AttemptWork,
    generation: u64,
}
struct Imported {
    receipt: Receipt,
    receipt_digest: String,
    envelope_digest: String,
}
/// Admin-only borrowed metadata, never a raw artifact or cached authorization result.
pub struct AttemptObservation<'a> {
    pub run_id: &'a str,
    pub generation: u64,
    pub envelope_digest: &'a str,
}
/// One controller-owned process-local history. Exclusive mutable borrowing serializes mutation.
/// A fresh instance is not durable recovery or a globally current attempt authority.
pub struct AttemptHistory<'a> {
    access: &'a AuditAccess,
    owner: Arc<()>,
    stores: BTreeMap<AttemptTarget, InMemoryAttemptStore>,
    registrations: BTreeMap<String, Registration>,
    imported: BTreeMap<String, Imported>,
}
impl<'a> AttemptHistory<'a> {
    pub fn new(access: &'a AuditAccess, admin: &Administration) -> Result<Self, AttemptError> {
        access.check_admin(admin)?;
        Ok(Self {
            access,
            owner: Arc::new(()),
            stores: BTreeMap::new(),
            registrations: BTreeMap::new(),
            imported: BTreeMap::new(),
        })
    }
    pub fn register(
        &mut self,
        admin: &Administration,
        run_id: &str,
        work: AttemptWork,
        expected_generation: u64,
    ) -> Result<AttemptTicket, AttemptError> {
        self.access.check_admin(admin)?;
        if run_id.is_empty()
            || run_id.len() > 128
            || !run_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err(AttemptError::InvalidInput);
        }
        if self.registrations.contains_key(run_id) {
            return Err(StoreError::Conflict.into());
        }
        if self.registrations.len() >= MAX_ATTEMPTS {
            return Err(AttemptError::Budget);
        }
        // Target is derived only from the private, validated full binding; no independently
        // supplied GE target can diverge from a ticket's domain identity.
        let engine_target = work.target.engine();
        let generation = match self.stores.get_mut(&work.target) {
            Some(store) => store.advance(
                engine_target,
                expected_generation,
                work.digest.clone(),
                run_id.into(),
            )?,
            None => {
                let mut store = InMemoryAttemptStore::default();
                let generation = store.advance(
                    engine_target,
                    expected_generation,
                    work.digest.clone(),
                    run_id.into(),
                )?;
                self.stores.insert(work.target.clone(), store);
                generation
            }
        };
        let ticket = AttemptTicket {
            owner: self.owner.clone(),
            run_id: run_id.into(),
            generation,
            target: work.target.clone(),
            work_digest: work.digest.clone(),
        };
        self.registrations
            .insert(run_id.into(), Registration { work, generation });
        Ok(ticket)
    }
    fn registration(&self, ticket: &AttemptTicket) -> Result<&Registration, AttemptError> {
        if !Arc::ptr_eq(&ticket.owner, &self.owner) {
            return Err(AttemptError::ForeignTicket);
        }
        let registration = self
            .registrations
            .get(&ticket.run_id)
            .ok_or(AttemptError::ForeignTicket)?;
        if registration.generation != ticket.generation
            || registration.work.target != ticket.target
            || registration.work.digest != ticket.work_digest
        {
            return Err(AttemptError::ForeignTicket);
        }
        Ok(registration)
    }
    fn record(
        &self,
        ticket: &AttemptTicket,
        output: &EnvelopeOutput,
    ) -> Result<AttemptRecord, AttemptError> {
        let work = &self.registration(ticket)?.work;
        let envelope = output.envelope();
        if envelope.run_id != ticket.run_id
            || envelope.binding != work.binding
            || envelope.producer != work.producer
            || envelope.coverage.required_scopes != work.scopes
        {
            return Err(AttemptError::Binding);
        }
        if envelope.run_status == RunStatus::Completed {
            if output.mapping_digest() != Some(work.mapping_digest.as_str())
                || envelope
                    .artifacts
                    .contract
                    .as_ref()
                    .map(|a| a.digest.as_str())
                    != Some(work.contract_digest.as_str())
            {
                return Err(AttemptError::Binding);
            }
        } else if output.mapping_digest().is_some()
            || output.contract_bytes().is_some()
            || output.facts_bytes().is_some()
            || output.report_bytes().is_some()
        {
            return Err(AttemptError::Binding);
        }
        consumer::budget_output(output, envelope).map_err(|_| AttemptError::Budget)?;
        Ok(AttemptRecord {
            run_id: ticket.run_id.clone(),
            target: work.target.engine(),
            generation: ticket.generation,
            content_digest: work.digest.clone(),
            envelope_digest: digest(
                &serde_json::to_vec(envelope).map_err(|_| AttemptError::Binding)?,
            ),
            eligible: false,
        })
    }
    /// Only already-retained evidence may enter history. Replay still rechecks raw access,
    /// retention, all blob bytes and binding; an expired prior import is not a read shortcut.
    pub fn import(
        &mut self,
        admin: &Administration,
        raw: &RawAccess,
        ticket: &AttemptTicket,
        receipt: &Receipt,
        now: i64,
    ) -> Result<ImportOutcome, AttemptError> {
        self.access.check_admin(admin)?;
        self.registration(ticket)?;
        let output = self.access.read_raw(raw, receipt, now)?;
        let record = self.record(ticket, &output)?;
        // read_raw validated receipt fields while borrowed before this serialization/clone.
        let receipt_digest =
            digest(&serde_json::to_vec(receipt).map_err(|_| AttemptError::Binding)?);
        if self.imported.get(&ticket.run_id).is_some_and(|old| {
            old.receipt_digest != receipt_digest || old.envelope_digest != record.envelope_digest
        }) {
            return Err(StoreError::Conflict.into());
        }
        let owned = Imported {
            receipt: receipt.clone(),
            receipt_digest,
            envelope_digest: record.envelope_digest.clone(),
        };
        let outcome = self
            .stores
            .get_mut(&ticket.target)
            .ok_or(AttemptError::Binding)?
            .append(record)?;
        if outcome == ImportOutcome::Inserted {
            self.imported.insert(ticket.run_id.clone(), owned);
        }
        Ok(outcome)
    }
    pub fn publish(
        &mut self,
        admin: &Administration,
        raw: &RawAccess,
        ticket: &AttemptTicket,
        now: i64,
    ) -> Result<(), AttemptError> {
        self.access.check_admin(admin)?;
        self.registration(ticket)?;
        let imported = self
            .imported
            .get(&ticket.run_id)
            .ok_or(StoreError::UnknownAttempt)?;
        let output = self.access.read_raw(raw, &imported.receipt, now)?;
        if self.record(ticket, &output)?.envelope_digest != imported.envelope_digest {
            return Err(StoreError::Conflict.into());
        }
        self.stores
            .get_mut(&ticket.target)
            .ok_or(AttemptError::Binding)?
            .publish(&ticket.run_id)?;
        Ok(())
    }
    pub fn current(
        &self,
        admin: &Administration,
        target: &AttemptTarget,
    ) -> Result<Option<AttemptObservation<'_>>, AttemptError> {
        self.access.check_admin(admin)?;
        Ok(self
            .stores
            .get(target)
            .and_then(|s| s.current(&target.engine()))
            .map(observation))
    }
    pub fn history(
        &self,
        admin: &Administration,
        target: &AttemptTarget,
    ) -> Result<Vec<AttemptObservation<'_>>, AttemptError> {
        self.access.check_admin(admin)?;
        Ok(self
            .stores
            .get(target)
            .map(|s| {
                s.history(&target.engine())
                    .into_iter()
                    .map(observation)
                    .collect()
            })
            .unwrap_or_default())
    }
    /// Retained evidence and authority are queried anew; holding `&self` excludes local mutation
    /// for this call. Caller-owned Mutex can serialize this complete operation across threads.
    pub fn consume_current(
        &self,
        raw: &RawAccess,
        expected: &ExpectedConsumption,
        provider: &dyn AuthorityProvider,
        now: i64,
        cause: Option<&str>,
    ) -> Result<Consumption, AttemptError> {
        self.access.check_raw(raw)?;
        consumer::serialized_size(expected).map_err(|_| AttemptError::Budget)?;
        if cause.is_some_and(|s| s.len() > 16_384) {
            return Err(AttemptError::Budget);
        }
        let work = AttemptWork::freeze(
            &expected.policy.binding,
            &expected.policy.producer,
            &expected.policy.required_scopes,
            &expected.policy.contract_digest,
            &expected.mapping_digest,
        )?;
        let current = self
            .stores
            .get(&work.target)
            .and_then(|s| s.current(&work.target.engine()))
            .ok_or(AttemptError::NotCurrent)?;
        if current.run_id != expected.run_id
            || current.content_digest != work.digest
            || current.envelope_digest != expected.envelope_digest
        {
            return Err(AttemptError::Binding);
        }
        let imported = self
            .imported
            .get(&current.run_id)
            .ok_or(AttemptError::Binding)?;
        Ok(self
            .access
            .consume(raw, &imported.receipt, expected, provider, now, cause)?)
    }
}
fn observation(record: &AttemptRecord) -> AttemptObservation<'_> {
    AttemptObservation {
        run_id: &record.run_id,
        generation: record.generation,
        envelope_digest: &record.envelope_digest,
    }
}
