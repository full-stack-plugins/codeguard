# Read-only SDK consumption

`guard_integration::consumer::consume` accepts a sealed `EnvelopeOutput`, independent
controller `ExpectedConsumption`, an explicit `AuthorityProvider`, current UTC Unix
time, and an optional predecessor cause. It performs no repository discovery,
installation, native invocation, network operation, notification, or repair.
An injected authority provider is an external port: its authentication, freshness,
and side effects are the controller's responsibility. The tests use a fixture
provider and do not establish a production identity provider (task 3.2).

The controller freezes these expected inputs separately from the observation:

- Full GE eligibility policy: binding, producer/analyzer, required scopes,
  contract digest, action and producer/approval principal policy.
- Expected run ID, semantic mapping digest, envelope digest, and raw native digest.

Digests use `sha256:` plus lowercase hex. Envelope digest hashes the JSON produced
by `serde_json::to_vec(envelope)`. Mapping digest hashes the registered typed mapping
serialization and travels privately through Projection to EnvelopeOutput; it is
not reconstructed from a caller's claimed observed string. Ruff required scope
identity includes the frozen source, target, configuration, tool and producer.

Before GE serializes inputs, the consumer stream-counts envelope and expected
metadata (1 MiB each), checks cause (16 KiB), raw domain (1 MiB), and contract,
facts and report (GE's 16 MiB per artifact). It checks expected identities and
actual raw-domain artifact SHA, then invokes GE to validate and recompute engine
artifacts and evaluate eligibility against freshly queried authority. No cached
result is reused. The returned freshness key hashes complete expectations plus
GE's current audit, including run/envelope/raw/mapping and policy identity. It is
an observation identifier, never permission to skip revalidation or reuse a run.

Eligibility failure does not rewrite the technical report or decision. Partial
scope remains BLOCK. Original SDK outputs without approval references remain
MissingApproval for review results. The additive `with_approval_refs` and
`consume_attached` path below supports locally verified references; it does not
issue approvals or implement a production authority service. Unsupported/native-only
output cannot become a completed engine result here.

This is the **SDK local profile**, not the Git candidate-host provenance consumer.
It cannot inspect actual Git state or prove the controller's labels. A host must
revalidate its independently current GG snapshot, clean repository and full
candidate identity through the accepted candidate host; old CandidateProjection
objects do not establish current Git provenance. A future host consumer must
also validate its additional candidate.json artifact and full GG semantics.
Neither this SDK API nor its passing fixture tests replace that responsibility.

Tests mutate every RunBinding field and the other identity expectations, check
fresh producer revocation/unavailability/expiry and incomplete/missing-approval
outcomes, and preserve original report bytes. The explicit side-effect test
compares a real Git fixture's source, .codeguard events, refs and index before and
after ten consumptions. Linux strace records no child invocation, network call,
write-open or filesystem mutation in that test. Native commands and wire output
are unchanged; only private in-memory mapping metadata was added.

## Bounded approval-reference attachment

A protected controller may call `output.with_approval_refs(&references)` to create
an `ApprovalAttachment` borrowing all original contract/facts/report/domain bytes.
Only a complete, completed, engine-recomputed REQUIRE_APPROVAL result permits
attachment. ALLOW, BLOCK/partial, error and cancelled outputs are refused.
The original envelope remains unchanged. The new envelope changes only approval
references; technical report and REQUIRE_APPROVAL decision remain unchanged.

References must be sorted and unique, at most 64 entries, each nonempty and at
most 1024 UTF-8 bytes with no control characters, at most 16 KiB total. Actual
serialized metadata including escaping must remain within 1 MiB; these checks
precede cloning the envelope. An empty list is valid and means no approval.
References are opaque identifiers, never grants or authority claims.

The controller independently updates its expected envelope digest and uses
`consumer::consume_attached`. The provider must authenticate the **new** envelope
digest; a record authenticating the old envelope is rejected. Every call freshly
queries applicable approvals using GE's unchanged action, purpose, principal,
binding, contract and validity rules. Expired/revoked/untrusted/unavailable records
remain ineligible. Changing or removing references invalidates the old expectation
and old authenticated producer record. No cross-run or approval cache exists.

Local fixture tests demonstrate eligible approved review while preserving the
technical REQUIRE_APPROVAL decision, plus changed/expired/revoked/provider-error
counterexamples. These fixtures authenticate only explicitly fixed test digests
and do not establish production authority (3.2), issue a grant, or authorize host
writes. The SDK/GG provenance separation described above still applies.
