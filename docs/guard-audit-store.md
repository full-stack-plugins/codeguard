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

This slice is **3.5 only**. Opaque delegated administration/raw-access separation,
public redacted views and retention/purge are the subsequent 3.6 slice. No public
view or retention guarantee is inferred from a surviving manifest. Fresh current
attempt/CAS publication is a separate 3.4 task. Actual GG candidate/tree/provenance
revalidation remains the candidate host's responsibility.
