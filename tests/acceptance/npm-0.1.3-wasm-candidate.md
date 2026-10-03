# npm 0.1.3：32 份 WASM 候选的公开包局部验收

日期：2026-10-03。对应 OpenSpec `introduce-rust-codeguard-cli` 的 13.4、14.18、14.19 局部增量。这是 **候选资产可从公开 npm 包调用** 的证据，不是 32 种语言的 lint 精度验收。

## 源码与打包

- 干净源码提交：`7900a1a8ccd5b4f5b54de8b5db3ccba16a356d3b`。用 `CODEGUARD_BUILD_SHA=<该提交> CARGO_INCREMENTAL=0 cargo build --release --locked -p codeguard-cli --features wasm-precheck` 构建 Apple Silicon macOS 程序，再运行 `node scripts/pack-npm-local.mjs --public target/release/codeguard`。
- 打包器拒绝无 WASM worker 的默认程序，核对 32 个固定 grammar 身份并执行真实 Zig 探针。每份上游许可证和 CodeGraph 许可证先按 `grammars/manifest.json` 的 SHA-256 核对，再随包写入 `grammar-licenses/`；缺失、漂移、清单冲突或 tarball 遗漏均拒绝打包。
- 本机 Node 测试先因包内缺许可证失败，修复后 `tests/npm_pack_wasm.test.mjs` 为 3/3 通过：离线安装包中 Zig、Dart 探针返回候选状态；四组 `check all` 的 `candidate_observed` 并集恰好覆盖全部 32 种，`skipped_count=0`，组报告保持 `incomplete` 与退出码 3。
- 工作区测试 `cargo test --workspace --all-targets --locked` 成功；WASM 运行层 12 个 grammar 加载测试和 Dart 上游语料测试、CLI 全 32 路由 5 个测试及公开探针 2 个测试成功。相同源码提交的 [Linux CI 37104976461](https://github.com/full-stack-plugins/codeguard/actions/runs/37104976461) 通过 worker 边界、包内 npm 测试及工作区测试。

## 注册表与实装

- `npm publish ./release/npm/codeguard-public-0.1.3-darwin-arm64.tgz --access public --tag latest --ignore-scripts` 成功；`@partme.ai/codeguard@0.1.3` 为当前 `latest`，仅支持 `darwin-arm64`。
- 注册表地址：`https://registry.npmjs.org/@partme.ai/codeguard/-/codeguard-0.1.3.tgz`。SHA-512 integrity：`sha512-RdhW/Tvf6a+7ZhRQFzSCJL87olzKmCjzQfyiqPZ3djTOodbdP7YfchT9VOFq/d7i+g95AME1mQC4HilHdb6WXQ==`。
- 注册表下载包与本机 tarball 的 SHA-256 同为 `37509cdd108fee631eebb9ebcd794a6e8d63652801b3522625d0909c3da2ae12`；包内二进制与本机构建的 SHA-256 同为 `04bdea8c489693a8e97b451df8d0c2b7a89ba82372db5a4d1fed277ba19b0f4d`。包共 38 个文件，其中 `grammar-licenses/` 有 32 个许可证文件（TypeScript 与 TSX 共用一份上游许可证，另含 CodeGraph 许可证）。
- 全新 npm 缓存从注册表安装并运行 `codeguard --version --format json`，返回 `cli_version=0.1.3`、`target=macos_arm64`、上述源码提交。相同安装下 `codeguard grammar status --format=json` 返回 `candidate_count=32`、`released_count=0`、`delivery_decision=not_evaluated`、退出码 3。

## 未完成边界

公开包证明 32 份固定资产可安装、可由 Rust worker 调用，并能经有界 `check all` 进入候选观察。它不证明所有语言/版本/方言的误报与漏报率、完整原生优先路由、每种语言的独立 lint、任务自动复检关闭、宿主对话反馈、其它平台、签名来源或可复现构建。已知 Swift 漏检、CFQuery 漏检、VB.NET 误报和 COBOL 成本问题仍保留；所有 grammar 的 `grammar_qualified=false`，语法能力 `released_count=0`。13.4、14.18、14.19 均不据此勾选。

`codeguard-plugin` 的显式候选运行时仍锁定 npm 0.1.2，且当前校验器只接受旧包的 6 个成员、把 tarball 和下载流限制在 8 MiB、把解压后的二进制限制在 32 MiB；0.1.3 包有 38 个成员、约 12.1 MB，二进制约 100.1 MB，不能直接替换锁文件。默认 Hook 仍使用旧 Python 入口。插件升级须单独调整包成员/大小/许可证校验、锁定摘要和候选事件测试，再遵循插件仓版本与市场发布流程；本次发行不声称插件宿主已使用 WASM。

2026-10-03 插件后续进展：`codeguard-plugin` 本地提交 `7bfb07a` 已把显式候选运行时锁定到 0.1.3，并验证 38 个精确成员、32 份许可证、注册表与离线安装。其 `exec check all ABS_PROJECT` 可调用 CLI 的原生优先统一检查；插件候选测试 5/5 通过。本仓再次用离线 npm 包执行 32 语种 `check all` 路由测试 1/1 通过。插件 `main` 受 GitHub PR 规则保护，本地提交尚未发布为插件 tag/市场版本；默认 Hook 仍是 Python，自动对话反馈与逐语言精度验收未完成。原段记录的是 CLI 0.1.3 初发时的插件状态，不是此后插件的当前状态。
