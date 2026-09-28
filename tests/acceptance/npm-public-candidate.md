# npm public candidate — 2026-09-28

The first `@partme.ai/codeguard` candidate is a single-host package for `darwin-arm64`, built from this working tree with `cargo build --release --locked --offline -p codeguard-cli`. It is not a multi-platform release or a quality-gate certification.

| Check | Result |
| --- | --- |
| Rust binary identity | `cli_version=0.1.0`, `target=macos_arm64`; `build_identity=null` |
| Candidate tarball | `release/npm/codeguard-public-0.1.0-darwin-arm64.tgz`; SHA-256 `444b877f23829cf5efba01d306d0612278ef713ac19a4f28bca9fc29e60f925e` |
| Package contents | Node launcher, native Rust executable, package metadata, README, Apache-2.0 text, NOTICE; no lifecycle scripts |
| Local package execution | Fresh local install and `npm exec --offline --package <tarball> -- codeguard --version --format json` returned the matching identity |
| Registry publication | `npm publish ./release/npm/codeguard-public-0.1.0-darwin-arm64.tgz --access public --tag latest` succeeded: `+ @partme.ai/codeguard@0.1.0`. `npm access get status` reported `public`; `npm dist-tag ls` reported `latest: 0.1.0`. |
| Fresh registry execution | With a new npm cache, `npx --yes @partme.ai/codeguard --version --format json` exited 0 and reported `cli_version=0.1.0`, `target=macos_arm64`, `build_identity=null`. |

Earlier publish attempts, including a retry at 2026-09-28 14:11 UTC, returned HTTP 403 because the available publishing credential did not satisfy npm's 2FA policy. After the user configured a new credential, publication succeeded. Immediately afterward the package metadata endpoint briefly returned 404 while the exact version, tarball and dist-tag were available. A later fresh-cache `npm view` returned version `0.1.0` and `latest: 0.1.0`, followed by the successful fresh-cache `npx` run above. No token value was printed or retained in this record.

The published package is restricted by npm `os=darwin` and `cpu=arm64`; other hosts require separate builds and a revised package layout. It was built from an uncommitted working tree, so the source revision is not yet bound to an immutable release. This verifies the published command on Apple Silicon macOS only, not the full S13 release or quality-gate contract.
