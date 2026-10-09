# Explicit local Linux audit storage (3.5)

`guard_integration::audit::AuditStore` is an opt-in SDK API. It does not add a
native command, default path, current pointer, execution, repair, installation,
network operation or implicit write. The caller explicitly supplies an existing
absolute current-owner directory with mode 0700 and resource limits. All path
components are opened without following symlinks. The directory descriptor pins
the store even if its pathname is subsequently moved or replaced.

`stage(AuditEvidence::Output(&output))` and the corresponding `Attached` variant
accept sealed SDK evidence. They check borrowed budgets, recompute engine
artifacts, verify native-domain references, and write fixed named blobs plus a
manifest. Approval attachments keep their original technical decision and bytes.
Failure/cancelled evidence can be audited without becoming completed engine
results. No native finding is lost from the borrowed in-memory output when a
write, quota check or publication fails.

The runtime filesystem primitive stays in codeguard-runtime using its existing
libc dependency. The opt-in CLI adapter owns CodeGuard semantics and reuses GE
validation/verification. Neither the generic primitive nor the adapter copies
TG/GG domain policy. GE's existing envelope-only publisher is not claimed to
satisfy these descriptor-relative blob guarantees.

## Publication and verified reads

Every stage uses a fresh directory-open description and a nonblocking flock.
Independent handles, threads and reopened stores scan actual persisted byte
counts under that lock; crashed staging and unknown private entries count toward
quotas. All writers must use the controller's selected limits. A stage uses
exclusive descriptor-relative creation, current-owner mode-0600 regular files
with one link, and actual read-back comparison against the original bytes. File
and staging-directory fsync precede renameat2(RENAME_NOREPLACE); root fsync follows.
Existing reports, user files, symlinks and earlier attempts are never overwritten.
Dropping an unpublished stage removes only its own descriptor-relative files and
removes the staging directory entry only if it still names the same inode.

A post-rename root fsync failure returns `PublicationIndeterminate`: the final
bundle may exist. Retain `staged.receipt().clone()` before publishing and explicitly
read/reconcile it; do not overwrite or silently declare success. An ordinary
retry also cannot replace the existing final path. Interrupted or crash-orphan
staging is not published evidence. Tests exercise real OS sink/fsync errors and
Linux syscalls, not physical power loss.

A receipt is consistency metadata, not an access grant or authenticity proof.
`read(&receipt)` reads only fixed names from the pinned root; public artifact URIs
are never filesystem paths. It rechecks file type, owner, mode, link count,
per-file/aggregate bounds, actual byte counts, manifest/receipt hashes, all blob
hashes, envelope references, native digest and engine recomputation. Missing,
truncated, tampered or unsafe required artifacts fail. Restored artifacts move
into an SDK output without copying raw payloads; they still require independent
current expectations and fresh authority via `consumer::consume`. Reading never
makes them eligible or proves current Git state.

## Limits and qualifications

- Linux local controller-owned filesystem profile; other OSes explicitly refuse.
- At most 256 stored entries and 512 MiB total controller-selected quota. Staging
  uses the same quota. Existing files are counted from actual bounded reads.
- At most 8 files per generic bundle, 16 MiB per generic file, 50 MiB per bundle.
- CodeGuard uses fixed manifest/envelope/native/contract/facts/report names;
  manifest 16 KiB, envelope and native 1 MiB each, engine artifacts 16 MiB each.
  Borrowed preflight reserves the manifest's full 16 KiB before hashing/cloning;
  this conservative allowance may reject a near-limit record that would encode
  smaller. Persisted quota accounting uses actual bytes, not manifest claims.
- Same-UID/root malicious actors, hostile filesystems and production identity are
  outside this profile. Directory permissions protect against other users;
  digests/receipts do not establish an external authenticated principal.

## Retention and delegated access (3.6)

`audit::access::AuditAccess::open` is the explicit retained-evidence interface.
The original `AuditStore` remains a trusted controller primitive, not a delegated
or tenant security endpoint. A host must keep raw filesystem access and this
low-level primitive out of its delegated/public interface.

Opening an existing private root binds one namespace and immutable retention
policy and returns opaque `Administration`. The controller can separately issue
`ViewAccess` and `RawAccess`. These types have no public constructors or
serialization/deserialization; a receipt or namespace string is not a grant.
They are bound to this particular open session. Another root or reopening the
same root rejects old grants. This is a local capability fixture/host boundary,
not authentication of an external person or production authorization provider.

The public view whitelist is exactly `version`, `availability`, `run_status`,
and `decision`. Values are a fixed version string and fixed enums (or null).
There are no messages, paths, source, environment, IDs, digests, labels, references,
or arbitrary producer text. Expired/purged/unregistered records yield an
unavailable view without a technical decision. Corrupt/unsafe storage yields an
error with no raw payload. Original raw bytes are only restored with `RawAccess`;
redaction does not edit or replace the persisted original.

A protected controller supplies the clock to every retained operation. Do not
accept `now` directly from a delegated caller's request. Successful observation
advances a durable root-wide high-water mark before returning evidence. Lower
times fail across handles and reopen. An expired read also advances the mark;
turning the clock back cannot revive evidence. Checkpoint failure returns no
evidence. Atomic checkpoint rename followed by failed root fsync reports
`PublicationIndeterminate`; there is no success claim or rollback write. Tests
cover process-visible filesystem semantics, not physical power-loss recovery.

Retention is configured at namespace creation (1 second through 366 days maximum)
and per publication (nonzero, no longer than the namespace maximum). Expiry is
exclusive: `now >= expires_at` is unavailable. Namespace, retention policy and
resource limits must match on reopen. The initialization marker is durable before
the first checkpoint; absent/corrupt metadata never silently initializes over
existing records or a previously initialized root. Interrupted initialization
fails closed and needs explicit controller recovery outside this API.

The bounded checkpoint holds at most 256 record entries and 128 KiB, including
persistent tombstones. Publication reserves a root slot and 128 KiB of peak-byte
capacity for checkpoint replacement. The namespace requires at least four entry
slots and more than 256 KiB total storage quota. Actual runtime accounting still
counts all physical control, payload, orphan and staging bytes. There is no
silent eviction of clock state/tombstones when limits are reached.

`publish` writes an immutable verified bundle, then registers its retention data.
A registration failure leaves a counted orphan that the controlled API cannot
consume. `purge` first durably tombstones the retained receipt, then deletes its
validated files by descriptor under the same root lease. An interrupted or failed
physical deletion leaves the tombstone active, including if old bundle bytes
are restored later. There is no implicit sweeper, scheduler, installation or
native-command invocation.

`consume` requires raw access and rereads retention metadata and every required
artifact on each call. A whole-root lease spans time/retention validation, blob
reads and the fresh GE authority query. Concurrent purge/checkpoint operations
fail busy rather than qualifying evidence from stale open descriptors. Missing,
expired or purged evidence produces no eligible result. The underlying technical
decision and original report bytes never change. This explicit retention API
performs filesystem writes for its durable clock; the existing pure SDK consumer
and all default/native commands retain their previous behavior.

Purge cannot erase an `EnvelopeOutput` already held in caller memory or globally
revoke a GE approval. Fresh store-mediated consumption is the enforced boundary;
callers bypassing it with retained in-memory evidence are outside this access
API. Current attempt/CAS remains separate task 3.4. Actual GG tree/provenance
revalidation remains the candidate host's responsibility, and production
identity-provider task 3.2 remains independent.

## Process-local attempts and binding CAS (3.4)

`audit::attempts::AttemptHistory` is an explicit controller-owned local history
borrowed from one `AuditAccess` session. It reuses GuardEngine's
`InMemoryAttemptStore` for generation advance, append/replay and publication.
There is no durable checkpoint, globally current result, cross-process CAS or
restart recovery. A new history is a new controller context, not permission to
revive previous eligibility.

`AttemptWork::freeze` admits borrowed input before cloning or hashing: at most
64 requirements and 64 scopes, each field at most 1 KiB, and the complete encoded
work descriptor at most 64 KiB. It freezes the full binding, producer, required
scopes, contract digest and mapping digest and reuses GE's structural preparation
validation. Ruff scopes already incorporate profile, tool, config, source and
native producer identity. No report-supplied descriptor should replace the
controller's independent frozen work. Actual Git state/provenance is still the
candidate host's responsibility.

The private target contains actual repo, task, worktree and sorted requirement
IDs. GE Target has no worktree field, so each full CG target owns a separate GE
store. The adapter does not rewrite or fabricate a wire binding to add worktree
isolation. At most 256 registrations exist in one history. Run IDs are unique
across that history, bounded to 128 ASCII letters/digits/dash/underscore/dot.

`register` requires Administration and a compare-and-swap expected generation.
Success clears the prior current pointer immediately, before a result exists.
It returns a private non-serializable ticket bound to this history instance,
exact run ID, full target, work digest and assigned generation. Neither ticket
fields nor a target's fields can be changed by callers.

`import` requires Administration, a separate RawAccess grant and an already
retained Receipt. It always rechecks retention, original blobs, actual run ID
and registered binding before using GE append. Identical receipt/envelope bytes
replay idempotently; changed bytes under the same attempt conflict and do not
change history. Failed append never reports acceptance. No alternate raw JSON
import, output relabeling or retention bypass is added. Approval references must
be frozen into the imported envelope; changing them is a conflicting envelope
revision, while revocation/status changes remain fresh provider observations.

`publish` rechecks retained evidence and lets GE compare its generation to the
latest registered generation. A late head1 success may enter historical records,
but cannot replace head2's current BLOCK. Error and partial observations can be
recorded without being promoted to engine-backed complete results. The generic
GE record's eligibility flag is always false: no authorization is cached.

`current` and `history` expose bounded borrowed metadata to Administration only;
they do not return artifacts or assert availability/eligibility. Such metadata
may outlive expired or purged evidence. `consume_current` separately checks raw
permission, current run/work/envelope and the independent expected descriptor,
then delegates to the existing retained consume path for all artifacts and a
fresh authority query. Missing, expired, purged or changed evidence cannot yield
an eligible result; approval revocation does not rewrite the technical decision
or original native/report bytes.

Mutations require exclusive mutable access. A caller-owned Mutex can serialize
complete operations across threads; tests exercise real competing threads with
one expected-generation winner. This is local serialization, not a multiprocess
lock protocol. The immutable storage and retention APIs retain their existing
semantics, and all native commands/default paths remain unchanged.
