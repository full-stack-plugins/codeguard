# Rust 1.85 与 WASM 传递依赖兼容修复

日期：2026-10-04。延续既有 OpenSpec 1.2、11.1、12 的编译基线和 `binary-distribution` 的分层发行验收；不提高已声明的 Rust 1.85 最低版本，也不把该修复当作所有平台验收。

## 已复现问题与修复

[脱敏前后观察](evidence/rust-msrv-dependency-regression-2026-10-04.json)保留源锁身份和实际红/绿日志摘要。冻结 `cargo metadata --locked --offline --features wasm-precheck --filter-platform aarch64-apple-darwin` 的活动依赖图发现，工作区 rust-version=1.85，而 tree-sitter 0.25.10 的传递依赖 tree-sitter-language 0.1.8 声明 rust-version=1.90。当前 stable 的构建成功不能发现这一矛盾。

新增 Rust 回归对当前目标、实际选中特性和锁定活动节点检查最低版本；在原锁上实际失败：`tree-sitter-language 0.1.8 requires Rust 1.90`，0 passed/1 failed。修复将 runtime 的 WASM 特性显式约束 tree-sitter-language=0.1.7，其[发布 Cargo.toml](https://docs.rs/crate/tree-sitter-language/0.1.7/source/Cargo.toml)声明 Rust 1.77；保留 tree-sitter 0.25.10 与全部 grammar 字节。Cargo.lock 只改变该版本/摘要和 runtime 的显式依赖，没有更新其它平台依赖。

修复后同一静态回归通过：217 个选中节点，没有高于 1.85 的已声明最低版本，仍有 45 个节点没有最低版本声明。后者不解释为相容证明。crate 方向约束仅允许该解析器伴随依赖处于 runtime 普通依赖，core/adapters/CLI 和 dev 类型反例继续拒绝。

## 实际测试与边界

- 首轮修复后 WASM 静态最低版本与依赖方向：8 passed/0 failed/0 ignored；随后增加伴随依赖的跨 crate 反例，最终结果下方追加。
- runtime 的 WASM grammar 加载、错误恢复及 Dart 上游三目标：18 passed/0 failed/0 ignored，包含 32 份固定 grammar 的分批加载和基本语法控制。Kotlin/Swift 的隐藏缺失仍明确未完成，Erlang/VB.NET 已知差异没有删除或规避。
- 本机仅安装当前 stable，未安装或运行 Rust 1.85；新增独立 CI job 在临时 Linux runner 安装 1.85.0，并对默认/WASM 的 workspace/all-targets 执行 locked check。远端 job 的完成状态必须按本批新提交独立核验。

静态 metadata 不证明无声明依赖、语言特性、最低标准库 API 或跨平台构建相容；CI 的 check 也不能替代二进制运行、所有目标平台、宿主、检测精度和正式发行。公开 npm/插件锁保持不变，Erlang RED 草稿保持未提交。总体目标继续未完成。

## 本批终态

默认 workspace/all-targets 为 221 组，1238 passed/0 failed/113 ignored；其后仅最低版本检查测试补充 patch 版本反例，先 RED 后修正，最终默认与 WASM 的最低版本/方向目标分别各 9 passed/0 failed/0 ignored。WASM 的五项 CLI 目标另为 32 passed/0 failed/0 ignored，runtime 三目标 18 passed/0 failed/0 ignored；这些范围有重叠，不合计为产品验收规模。

更新依赖后的 32 grammar、358 例、35 来源组已全部实际回放，1 passed/0 failed/0 ignored，707.51 秒。原始 JSON 行只提取并保留换行，见[完整回放](evidence/grammar-cohorts-msrv-2026-10-04.json)，SHA-256 `4688c2f7abc2c0b633e7906d2c1f6e8b41f5714cb411f681a32c31fc8a348c6a`，程序摘要 `677221f47eccfc0cf0feb3458c4cd73ef7a9fc7a99448287032e9875580da718`。计数与先前回归一致：TP/FP/FN/TN 73/1/10/269，3 unknown、2 pending；只加计数，不混算不同来源精度。原始报告仍 incomplete/fails_fixture_threshold、资格零；没有执行新的原生 oracle 或独立 holdout，没有修复已知 Erlang/VB.NET 差异。

最终默认/WASM 全目标 Clippy -D warnings、fmt、分层、OpenSpec strict、208 schema 元定义、50 份历史实际报告/150 个伪造变体，以及新 358 例原始报告的 schema 与跨字段检查通过。全部 grammar/source schema 原件未改；未提交 Erlang 草稿摘要保持 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`。

## 日志身份

- `/tmp/codeguard-msrv-regression-red.log`：SHA-256 `b31e47b6405edd7a5bfeeffaad4eee621809218c2989fc3ed2f79b1a321cb937`。
- `/tmp/codeguard-msrv-patch-version-red.log`：SHA-256 `674fbf18f85cc3f3493e2d27183fdf6885512406055b1eb98f5a73335180b16b`。
- `/tmp/codeguard-msrv-feature-green.log`：SHA-256 `4db541210acd67df30d7bdbb2b14163c5801987d29a28cae89eb55b283ea6467`。
- `/tmp/codeguard-msrv-workspace-final.log`：SHA-256 `8852951c5f86a792459be09828924620886bb23fc423ab0ff9fe51c97eb6690d`。
- `/tmp/codeguard-msrv-guards-default-final.log`：SHA-256 `effdf0db2b48b56f3fb9c8fd717cb3aaa5a015e6e9a7f016e9ff01ae30eaf6d7`。
- `/tmp/codeguard-msrv-guards-feature-final.log`：SHA-256 `b40eca7a2b984a9fe2e0d3eeaca7c67887b40d16bc5a98067b5241d03d437bd3`。
- `/tmp/codeguard-msrv-wasm-runtime-regression.log`：SHA-256 `055baa01b5cc08723d1041bdf13b3f715f866d01e432e8f0211adaf47ede7b4c`。
- `/tmp/codeguard-msrv-feature-final.log`：SHA-256 `7323b4b96b306e6cb71d3e14694473fad88301b3896eba983a03d39be0060a52`。
- `/tmp/codeguard-msrv-grammar-replay-final.log`：SHA-256 `36b0453e9198b5bb7fbc5426a1bf5033793453868bc58b396fab22295521d96f`。
- `/tmp/codeguard-msrv-default-clippy-final.log`：SHA-256 `5b8fede94114b9ae38b8529df9844adfbd214676cd24d9260f9417d6c4c0f2ec`。
- `/tmp/codeguard-msrv-feature-clippy-final.log`：SHA-256 `7c5a8f9258ee6ed3ee7c06e42bdaec46eb74d8bad05007f259f310651599dc92`。
- `/tmp/codeguard-msrv-fmt-final.log`：SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。
- `/tmp/codeguard-msrv-layering-final.log`：SHA-256 `280c626d05d03e0944fca548fce5a1e9039200ab2cdd1f3d68f64c12e236de36`。
- `/tmp/codeguard-msrv-schema-document-validation.log`：SHA-256 `0870721e2755b45856056b728eabec0fccf8c522fa6aade804599de0d386ecd1`。
- `/tmp/codeguard-msrv-openspec-final.log`：SHA-256 `cf3ec12601576e4a3dd335699d867edc30029dd07ee9fdcd6396379524b1fa4b`。

Swift 源提交 `15a91a7b9291837e816f41d7934d68b4d083677e` 的[远端 CI](https://github.com/full-stack-plugins/codeguard/actions/runs/37195556160)已成功，包括全量语法回放、npm 安装及默认测试。该 CI 对应旧依赖锁；不能代替本批新锁和新最低工具链 job 的远端结果。

## 2026-10-04 远端终态补证

提交 `4d620f7e2e4c668f52b2e1083fba4e1c14b526d2` 的 [CI 37198192252](https://github.com/full-stack-plugins/codeguard/actions/runs/37198192252) 已终态成功。Linux 实际 Rust 1.85.0 默认及 WASM 全目标 locked check、完整语法回放和安装验收均完成；固定元数据与日志摘要见[公开证据](evidence/rust-msrv-linux-ci-2026-10-04.json)。本证据不包含当前未提交的首次原生指引修正，也不证明完整产品、全部平台或 grammar 资格验收。
