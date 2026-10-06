# Four-core production acceptance plan

The target includes 57 registered languages and 228 core obligations. This repository snapshot grants no qualification: versions and dialects remain unqualified, and all five platforms are candidate targets.

```bash
codeguard capabilities --acceptance-plan --format=json
codeguard capabilities java --acceptance-plan --format=json
```

The query does not scan projects, execute tools, load WASM, or evaluate delivery. The legacy 0.2 inventory stays unchanged. Filtering preserves the full 228 obligations; legacy category/platform filters cannot be combined with this view. Reports retain `qualification: not_granted` and `delivery_decision: not_evaluated`.

Syntax requires separate WASM and native acceptance. Parsing cannot substitute for detailed documentation, development conventions, or vulnerability scans. Java Maven and Gradle are independent paths: Gradle CVE currently has configuration observation only. Partial Maven evidence is not production qualification. Ecosystems lacking native CVE engines require explicit deployment/dependency scanning boundaries; do not invent official lint tools.

All 32 grammar candidates are mapped. TypeScript owns JavaScript/TSX; CFML owns CFQuery/CFScript. Candidate assets do not imply released capability: qualification remains 0/32. Preliminary results cannot discharge missing native obligations.

`rulepacks/production_acceptance_plan_v1.json` maps build ecosystems, adapters, evidence, existing OpenSpec tasks, and blockers. The Rust audit checks referenced source hashes and task existence; it does not prove complete production behavior:

```bash
cargo run --offline --locked -p codeguard-cli --example audit_production_acceptance_plan
```

The sole specification source remains `openspec/changes/introduce-rust-codeguard-cli`, including S15.1–15.7. S15.1 stays open until concrete version/dialect/platform coverage, independent precision and real host results are complete. The following rows are obligations, not advertised support. Detailed dialect requirements remain in the canonical snapshot.

| Language | Build/runtime ecosystems | WASM candidates |
|---|---|---|
| java | maven, gradle | java |
| rust | cargo | rust |
| typescript | npm, deno | typescript, javascript, tsx |
| python | pip, poetry, uv, pdm | python |
| go | go-modules | go |
| csharp | dotnet | csharp |
| kotlin | gradle, maven | kotlin |
| swift | swift-package-manager, xcode | swift |
| php | composer | php |
| ruby | bundler, rubygems | ruby |
| scala | sbt, scala-cli | scala |
| shell | standalone-shell, enclosing-project | — |
| dockerfile | container-build | — |
| yaml | enclosing-project | — |
| elixir | mix | — |
| css | node-stylesheet-project | — |
| c | standalone, cmake, conan, vcpkg | c |
| cpp | standalone, cmake, conan, vcpkg | cpp |
| objc | xcode, clang-project | objc |
| dart | pub, flutter | dart |
| vue | npm-vue | — |
| svelte | npm-svelte | — |
| astro | npm-astro | — |
| solidity | foundry, hardhat | solidity |
| terraform | terraform-providers-modules | terraform |
| nix | nix-flakes, nix-expression-project | nix |
| html | web-project | — |
| sql | database-dialect-project | — |
| graphql | schema-client-project | — |
| protobuf | buf, protoc-build | — |
| markdown | documentation-project | — |
| toml | enclosing-project | — |
| haskell | cabal, stack | — |
| ocaml | dune, opam | — |
| fsharp | dotnet | — |
| perl | cpan, perl-project | — |
| groovy | gradle, maven | — |
| clojure | tools-deps, leiningen | — |
| powershell | powershell-modules | — |
| zig | zig-build | zig |
| nim | nimble | — |
| crystal | shards | — |
| julia | julia-project | — |
| elm | elm-packages | — |
| lua | luarocks, embedded-host | lua |
| luau | luau-project, embedded-host | luau |
| pascal | freepascal, delphi | pascal |
| r | renv, r-package | r |
| cfml | cfml-engine-project | cfml, cfquery, cfscript |
| cobol | gnucobol, enterprise-cobol | cobol |
| vbnet | dotnet | vbnet |
| erlang | rebar3, otp-application | erlang |
| arkts | hvigor-ohpm | arkts |
| metal | xcode-metal | — |
| liquid | shopify-theme, embedded-template-host | — |
| cuda | cmake-cuda, nvcc-project | — |
| ansible | ansible-collections | — |
