# Local candidate host acceptance

Task 4.4 profile: `codeguard-gg-candidate-host` 0.1.0, the existing unpublished
Rust library in `integrations/gg-candidate-host`. This is an explicitly authorized
local caller, not an MCP server, IDE plugin, remote identity service, or deployment.
The controller supplies exact candidate bytes, repository root, target and tool
pins; no ambient host discovery or automatic activation occurs.

The actual probe is
`candidate_binding::actual_local_host_reentry_cancel_and_credential_boundary`.
It deliberately requires native pins, a fake parent credential sentinel and a new
capture directory. Run it explicitly; the default ignored result is not acceptance:

```sh
CODEGUARD_CANDIDATE_PRODUCER=/absolute/pinned-codeguard \
CODEGUARD_CANDIDATE_PRODUCER_SHA256=b7576afa78f5ab6fdc8133650a1c75dc113357ac927e553b4f5c67731eea3a8c \
CODEGUARD_CANDIDATE_RUFF=/absolute/pinned-ruff \
CODEGUARD_HOST_CREDENTIAL_SENTINEL=fixture-credential-must-not-inherit \
CODEGUARD_CANDIDATE_CAPTURE_DIR=/absolute/new-existing-empty-directory \
cargo test --locked --offline \
  --manifest-path integrations/gg-candidate-host/Cargo.toml \
  --test candidate_binding actual_local_host_reentry_cancel_and_credential_boundary \
  -- --ignored --exact --test-threads=1
```

## Verified capability boundary

| Capability | Probe and bounded claim |
| --- | --- |
| Actual candidate | Creates real Git SHA-1 objects, freezes GG candidate, copies the exact committed Python file and Ruff config into private storage. Existing candidate tests separately cover SHA-256 and queue identities. |
| Actual native execution | Preserved CodeGuard source `f5661d51dd1b2629f3b87b726fb09a9060ce7958`, binary SHA above, official Ruff 0.16.8 SHA `f3eb080f0173e1a5884fabcc2c836f5a0ebea35fccf252e718fc71b17f821ef7`; original native exit 3 is retained on both real invocations. Only F401 projection is qualified. |
| Reentry | Two independent prepare/capture/project calls around a cancelled call retain distinct native run IDs and the same full candidate. The first raw bytes remain available and reprojectable. This is sequential reentry, not a concurrent host scheduling claim. |
| Cancellation | Pre-set cancellation returns no capture; the next fresh call works. This does not measure in-flight cancellation latency or promise interrupts during pin hashing. |
| Credentials | Fake credential exists in the test parent. A real `/usr/bin/env` mechanism probe through the same runtime with the same empty environment yields zero bytes. Host capture constructs that empty environment; native output and provenance also contain no sentinel. No identity authentication or OS sandbox is implied. |
| Source side effects | Byte-for-byte recursive source repository comparison includes index, refs and all `.git` files before/after both native calls, cancellation and evidence export. Private temp projects and GG object copies are allowed; source repository writes are not. |
| Authority | Provenance remains `authorization: not-evaluated`. Technical ALLOW supplies no merge grant or authenticated approval. |
| Artifact boundary | Host output has native plus candidate domain artifacts. It is not passed off as the SDK consumer's separate single-domain format. No cross-format compatibility claim. |

The first actually checked platform is Linux x86_64 with Rust 1.99.0 (b940084d7)
(the workspace declares Rust 1.90; this probe does not establish minimum-version
compatibility). Host lock SHA256:
`f216bf0108a806f82c08a1f1c358212dc7c5927a3e74050021f1fefeb872857b`.
Dependency source checkouts for this probe: GitGuard
`20f3a82e3858c8bdffe752e6314a97bef82f063a`, GuardEngine
`68f3c1fdaac962cd7e0d1d154f60bb1ee74fbed1`. Earlier README pins describe earlier
acceptance, not substitutes for these recorded sources. No package publication,
other platform, unknown tool combination, or complete native language matrix is
claimed.

Actual output is retained externally in `codeguard-host-local-capture/`:
`host-first/` and `host-reentry/` each contain original native stdout, expected
candidate, source/config, contract/facts/report/envelope, provenance and actual
Git bundle; `host-summary.json` records both run IDs, hashes and checked boundaries.
The author command log is `codeguard-host-local.log`. Independent review is required
before task 4.4 registration. No native behavior or host implementation changed to
make this probe pass; it checks the already implemented bounded host.
