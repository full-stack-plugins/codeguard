# Local joint queue and current authenticated consumption

Task4.3 acceptance candidate; registration awaits independent review.

The new `guard_integration_joint_queue` test consumes retained actual A/B/M1/M2
native reports and original Git objects. See the pinned fixture README for source
identities and the distinction between original native execution and newly created
controller baseline projections. No native rescan or production approval is
claimed.

`AttemptHistory::consume_current_prepared` extends the existing current-consumer
port, using the same exact current target/run/work/envelope checks and the same
retained access/expiry/bytes loader, then the independently frozen
PreparedConsumption baseline validation. The current-record helper is shared with
the older consumer. It introduces no store, scheduler or identity service. The
older non-null-baseline consumer remains unsupported and fails closed. A borrowed
history or caller-owned Mutex covers the complete local observation; no atomic
future execution/merge authorization is returned.

Actual cases:

- A/B concurrent callers retain distinct targets and both qualify with current
  baseline references; neither context can consume the other's evidence.
- Original exact M1 commits compose A/B; M2 changes actual base/group/member order.
  M2 publishes first, late M1 is retained but cannot publish or qualify as current.
  A->M1 and M1->M2 fail. Replacing M2's expected base with M1's cannot reuse its
  stored current receipt.
- After successful scan/consumption, revoking A's baseline makes A ineligible while
  B still qualifies. Store expiry also refuses a still-valid immutable bundle.
- Store reads and final corpus hashes prove original native/envelope bytes remain
  unchanged. The new envelope's baseline context and fixture authority are labeled
  separately from the original capture.
- Existing `guard_integration_trust` tests run alongside the joint test and show
  a valid baseline cannot repair retained actual Ruff missing-tool/partial or
  technical BLOCK. Existing concurrency cases retain new-head BLOCK over old ALLOW,
  review revocation, byte conflicts, access and generation/CAS rejection.

Reproduce with the existing Rust toolchain and an exclusive existing target:

```sh
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
CODEGUARD_JOINT_OUTPUT=/absolute/new-file.json \
cargo test --locked --offline -p codeguard-cli \
  --test guard_integration_joint_queue \
  --test guard_integration_concurrency --test guard_integration_trust
```

The output is evidence of this local fixture profile, not hosted queue-event
identity, actual multiuser/process authorization, new native qualification,
aggregate cross-guard orchestration or deployment. Native core/commands/schema and
57-language adapters are not changed by this slice.
