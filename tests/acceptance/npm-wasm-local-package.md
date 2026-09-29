# 32 份 WASM 候选的本地 npm 包局部验收

日期：2026-09-30。对应既有 OpenSpec S14.18、S14.19 的打包增量；两项任务仍未完成。

## 行为与命令

`scripts/pack-npm-local.mjs --require-wasm` 在创建 tarball 前核对二进制的 32 份候选身份、固定清单的语言与 SHA-256，并让 Zig worker 解析真实源码。`--public` 自动执行相同门禁，同时保留已有的干净 checkout 和构建提交身份核对。默认无 `wasm-precheck` 特性的二进制被拒，不能被打成声称带有 WASM 的包。Node 入口仍只转发参数给 Rust。

本机 Apple Silicon macOS 使用以下指令验证：

```bash
cargo build --locked --offline -p codeguard-cli --features wasm-precheck
CODEGUARD_NO_WASM_BIN=/tmp/codeguard-default-for-wasm-pack-af20dd8 \
CODEGUARD_WASM_BIN="$PWD/target/debug/codeguard" \
node --test tests/npm_pack_wasm.test.mjs
```

测试先要求一个单独保存的默认构建作为拒绝反例；上述 `/tmp` 路径只是本次本机临时证据，不是仓库或 CI 的固定路径。CI 先构建默认二进制并复制到 `$RUNNER_TEMP`，再构建 WASM 特性二进制，给同一测试传入两条实际路径。

本机结果：2/2 项通过。负例确认无 WASM worker 的二进制在打包前被拒。正例产出 private 本地 tarball，用独立缓存执行 `npm exec --offline --package <tarball> -- codeguard grammar status --format=json`，得到 32 份候选、0 份已验收发行；随后同一离线安装的 Rust 程序分别解析 Zig 和 Dart 源码，返回退出码 3、固定 grammar 摘要、`grammar_qualified=false`、`native.status=not_run`、`delivery_decision=not_evaluated`。测试中的临时源码采用物理路径，避免 macOS `/var` 别名路径触发源码路径安全检查。首次按别名路径测试失败后已修正，并再次通过。

这是**本地单平台包可运行性**，不是 32 种语言全部通过 npm 包逐一解析、独立语法 oracle、误报/漏报、原生优先完整路由、Linux 包验收或公开 npm 发布。已发布的 `@partme.ai/codeguard@0.1.2` 不含本次 WASM 特性；同一版本不能再次发布为新能力。待 CI 实际结果和新版本发行证据到位后，再决定是否更新对应发布任务。S14.18、S14.19 不勾选。
