# Native environment recheck — 2026-10-09

Baseline PR31 `958a106636ad8430e40e45075e49c6eea1496629`, repair baseline `f0a7bf7`. This Linuxx86_64 run does not qualify other hosts/platforms or all57languages. No product scanner/formatter was changed.

The baseline whole suite had2626pass/107fail/189ignored. Seven failed targets were rerun with real tools and workspace caches:

| Tool | Actual version/source | Scope |
|---|---|---|
| Go | official go.dev archive go1.27.2, SHA256 `ecbadb99091a3f46e31f5f934b068b1864eafa7995211b39eaddf76996045fe5` | Real Go replaces unrelated `/usr/bin/go`; isolated GOCACHE/GOMODCACHE |
| OpenJDK | Debian21.0.12.1, official package-index SHA256 verified | javac supplied through PATH |
| Prettier | npm3.6.2, package-lock integrity recorded | CSS/HTML, writable npm_config_cache |
| TypeScript | npm5.9.3, package-lock integrity recorded | tsc; existing tests also execute their own npm install |
| Python-Markdown | PyPI3.9, isolated venv | Markdown parsing, not a claim malformed fences are rejected |
| yamllint | PyPI1.37.1, isolated venv | Real YAML lint |

CSS/HTML/TypeScript/Markdown/YAML/Java20tests passed after tool/cache repair. Go initially retained one failure: two tests delete the third test's `go-comment-test` directory; concurrent actual vet reported module missing, while serial control passed3/3. Replaced shared/deleted directories with per-process/per-call exclusive directories. Invalid Go now requires an actual source location diagnostic rather than arbitrary nonzero. Three tests passed and10parallel-default repeat runs all passed. This is an existing PR31 test isolation defect, not a production Go scanner regression.

C/C++ acceptance gained explicit `CODEGUARD_TEST_CLANG` / `CODEGUARD_TEST_CLANGXX` tool-path selection; defaults remain `/usr/bin/clang` and `/usr/bin/clang++`. Actual DebianClang19.1.7 official packages were downloaded and hash-verified. It runs with an empty environment, but the existing product profile intentionally accepts only `Apple clang version 21.0.0 (clang-2100.3.34.2)`. The three native-success cases correctly remain failing with `clang_version_unverified`; other3boundary cases pass. This exposes an unsupported Linux/tool-version profile, not missing shared libraries. No version guard weakened and no Apple/native support claimed from Debian output.

Reproduction uses an isolated toolchain prefix:

```sh
source /workspace/guard-toolchain/native-env.sh
cargo test -p codeguard-cli --test go_six_category_acceptance \
 --test css_six_category_acceptance --test html_six_category_acceptance \
 --test typescript_six_category_acceptance --test markdown_six_category_acceptance \
 --test yaml_six_category_acceptance --test java_six_category_acceptance --no-fail-fast
```

Evidence in the implementation ledger: `codeguard-environment-{retest,final}.log`, `codeguard-go-{serial-control,green}.log`, `codeguard-go-repeat.json`, `codeguard-clang-green.log` (historical filename; contains3failed/3passed, NOT a green result). Tool provenance files/package locks are under `/workspace/guard-toolchain`. Other29failed targets remain unverified due to previously recorded tool absence; no mass skip or assumed pass. Historical Python timing/ETXTBSY root causes remain unresolved; this Go race diagnosis must not be applied to those unrelated tests.
