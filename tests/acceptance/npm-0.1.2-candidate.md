# npm 0.1.2 单平台候选与提示事件验收

对应 OpenSpec `introduce-rust-codeguard-cli` 的 S11.2、S13.4 和 Claude `UserPromptSubmit` 候选切片。本记录只证明 macOS arm64 上这一精确制品及显式插件候选入口，不代表默认 Hook、严格交付门禁或多平台发行已完成。0.1.1 的历史证据保留在 [前次记录](npm-0.1.1-candidate.md)。

2026-09-29：从干净源码提交 `d2ae54355ea0e8b67b7fa42fc66829656db9de01`，用 `CODEGUARD_BUILD_SHA=<该提交> CARGO_INCREMENTAL=0 cargo build --release --locked --offline -p codeguard-cli` 构建，再执行 `node scripts/pack-npm-local.mjs --public target/release/codeguard`。本机 `--version --format=json` 返回 `cli_version=0.1.2`、`target=macos_arm64` 和该构建身份；GitHub [CI 36512426737](https://github.com/full-stack-plugins/codeguard/actions/runs/36512426737) 对同一源码提交成功。

`npm publish ./release/npm/codeguard-public-0.1.2-darwin-arm64.tgz --access public --tag latest --ignore-scripts` 成功。注册表当前 `latest: 0.1.2`，URL 为 `https://registry.npmjs.org/@partme.ai/codeguard/-/codeguard-0.1.2.tgz`，SHA-512 integrity 为 `sha512-3ae3OIZDzqZaoNv2/W3a/Uq4rMMthjpjts/RQvKI5envR+NbyKOD4NTS/w645QFSw8ZDzHUWsnD7DObWFQAEpw==`。独立注册表下载包和本机候选包的 SHA-256 同为 `4411136f2cdf678615335a6a9fe4f3435fe87f82e789b98a1e07be4518587401`；注册表包内程序和本机程序的 SHA-256 同为 `785d8b9abb685ce5a3f4c1de0697d67a669394160bd7fb9a3554019d1d223bca`。全新 npm 缓存运行 `npx --yes @partme.ai/codeguard@0.1.2 --version --format=json`，回报同一版本、平台和源码身份。

插件 `runtime/codeguard.lock.json` 已锁定这些制品字段。显式候选入口的五项 Node 测试通过：错误 tarball 和后续二进制篡改被拒绝，缺运行时不调用 PATH 上的程序或 Python；真实二进制对敌意与普通 `UserPromptSubmit` 提示给出相同的有界指导，并说明源码检查未运行、交付未评估。独立缓存中从注册表安装、验证和运行该候选入口成功。详见插件仓 [候选验收](https://github.com/full-stack-plugins/codeguard-plugin/blob/main/tests/rust-runtime-candidate.md)；在插件变更合并前，该链接仍指向旧版本，不能当作已发布宿主证据。

`build_identity` 来自编译环境，不构成可复现构建或签名证明。当前公开包装未启用可选 `wasm-precheck` 特性。实际 Claude Code 默认 Hook、严格 Git/CI 门禁、其它平台包和完整发布信任链均未验收；S11.2、S13.4 及完整 S13 保持未完成。
