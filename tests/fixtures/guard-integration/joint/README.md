# Retained real four-candidate CodeGuard inputs

TRANSPORT.json pins 38 files from the independently executed FlowGuard joint
capture at source1fd70a1: four actual native CodeGuard reports, their original
contexts/mappings/engine artifacts, original GG candidate snapshots, producer
manifest and a transport Git bundle. Native CGf5661d51/schema0.13.0/exit3 and
projection72d80ae details remain in the original producer manifest. This test does
not reexecute native tools or alter native report bytes/run IDs.

The source.bundle is the already reviewed GitGuard transport (SHA256
43dd660e5bf4c4a2d9958b81e7c91c58a96d0343613bc0e1051274772901a548).
Its candidate/base objects match the original repository. Runtime verification
clones it, verifies actual commit objects, source bytes versus native source
bindings, exact M1 parent composition and changed M2 base/member order. Original
captured envelopes are reconstructed and compared before any new context exists.

The joint test then creates a **new controller baseline projection** for each
native report, using the shared project baseline digest. It preserves the native
bytes and original run ID, but intentionally changes the new envelope's baseline
binding and authenticates its new digest through test-only authority records.
This is not a claim that the original native reports carried baseline approval or
that captured producer identity has been authenticated in production. The original
files remain unchanged and are rehashed at the end.

A/B threads start behind a barrier; a controller-owned Mutex serializes complete
AttemptHistory observations using separate task/requirement targets. Original
logical worktreeId values remain fixture-worktree. No extra independent physical
worktree isolation is claimed here; the immutable bundled candidates and existing
snapshot identities are what the local fixture checks.

The test records A/B/M1/M2 projections, native hashes, frozen expectations and
actual store receipts; M1 can qualify as standalone evidence but cannot regain
current publication after M2. All raw captured inputs and commands remain pinned
in this corpus. `CODEGUARD_JOINT_OUTPUT=/absolute/new-file.json` retains the actual
combined result; it uses create_new and never overwrites previous evidence.

The historical FG invalidation drift helper is not used by this test. CodeGuard
policy enforcement semantics and original 57-language commands are unchanged.
