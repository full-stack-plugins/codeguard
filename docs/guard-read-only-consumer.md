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
scope remains BLOCK. The current sealed SDK output supplies no approval-reference
attachment interface: review results therefore remain MissingApproval. This
slice does not claim an approved review path or production authority service.
Unsupported/native-only output cannot become a completed engine result here.

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
