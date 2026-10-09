# Explicit local adapter rollout and rollback

The four steps below compose existing entrypoints; they are not a new phase
machine, native mode, policy switch, or identity service. `guard-project` and
`guard-project-ruff` remain explicit commands. Existing native dispatch does not
implicitly call the adapter or enable engine checks. A controller that requires
integration evidence must keep that requirement when it stops the adapter.

`PreparedConsumption::consume_required(Option<&EnvelopeOutput>, provider, now,
cause)` makes that requirement explicit at the existing authenticated SDK boundary.
`None` returns `required integration evidence missing` before provider callbacks;
`Some` calls the unchanged `consume`, preserving the complete frozen context,
artifact verification and current authority checks. There is no configuration
boolean that turns missing required evidence into success. Existing `consume`
callers retain their behavior. This method does not create or withdraw grants.

| Step | Actual exercised behavior | Required consumption |
| --- | --- | --- |
| Default/native | The ordinary compiled native `--version` entry runs without artifact writes. The retained real native Ruff stdout is unchanged; `shadow::compare(raw,None)` supplies only its digest. | Missing integration evidence fails; authority calls remain0. |
| Shadow | Existing reader/policy/projector replays the fixed real Ruff0.13 capture and `shadow::compare` recomputes its engine report against exactly the same raw bytes. | Technical ALLOW is not an authorization; shadow supplies no consumption output and required missing evidence still fails. No authority call. |
| Explicit opt-in | Actual `guard-project-ruff` process reads the fixed invocation/context/mapping/contract/raw and returns0 with original nativeExit3. Its envelope and all four inline artifact bytes exactly match SDK projection. | Only then does the controller pass `Some(output)` to its prepared consumer; current fixture provider is queried and eligibility evaluated. CLI0 alone is never treated as authenticated authority. |
| Stop adapter | Controller no longer submits adapter evidence; existing files and frozen required policy are retained. Native `--version` stdout/stderr/status equal the original call. | `None` fails without an authority call. Re-enabling `Some` after provider revocation rechecks and refuses; retained historical ALLOW does not override it. |

Tests use the already fixed genuine native capture and explicitly local fixture
AuthorityProvider. This is actual library/CLI composition over immutable evidence,
not a repeated native scan, remote production identity, or shared MCP host. The
host's separate two-domain artifacts are not accepted as SDK single-domain input.
The CLI's output is compared with the library's full same-byte output before
consumption, rather than substituting an unrelated permissive fixture.

Targeted test `guard_integration_rollback` includes the existing freshness fixture
regressions and two new tests. One exercises the four steps with real CLI processes;
one compares direct `consume` with `consume_required(Some(...))` for BLOCK,
partial/tool failure and unsatisfied REVIEW. No technical decision is repaired.
`guard_integration_trust` covers non-null baseline/current approval; shadow and
historical compatibility regressions retain their existing scope. Existing
historical compatibility tests bind the original23 command captures and native
exit families; no57-language native matrix is rerun or claimed complete.

A missing required projection is a controller consumption failure, not a change
to the original native numeric exit or a deletion of stored evidence. Disable
never changes the contract, baseline, current identity, source, mapping or required
scope to obtain success. Accepted local source package594644d remains immutable;
this later additive API is a separate source revision, not a silently replaced
binary or package.

Author verification: `cargo test --locked --offline -p codeguard-cli --test
 guard_integration_rollback --test guard_integration_trust --test
 guard_integration_shadow --test guard_integration_compat` yielded15 passed,
0 failed,1 existing ignored native recapture test. A second targeted rollback run
with `CODEGUARD_ROLLOUT_CAPTURE_DIR=/absolute/new-directory` yielded6 passed,
0 ignored and exported the unchanged native input, exact CLI output/configuration,
native-version stdout/stderr and `summary.json`. No pre-existing capture directory
is overwritten. External evidence: `codeguard-rollback-{red-api,green,green-capture}.log`
and `codeguard-rollback-local-capture/`. Fixed-source independent review is still
required before task5.2 registration.
