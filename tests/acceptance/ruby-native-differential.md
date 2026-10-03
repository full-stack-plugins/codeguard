# Ruby 2.6.10 原生语法与内置 WASM 的局部差分验收

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的局部精度证据，父任务仍未完成。

固定 13 份 Ruby 2.6 语法样例，用现存 `ruby -c -` 通过 stdin 做独立语法对照，并调用公开 `grammar probe ruby` 的隔离 WASM worker 检查同一字节。测试核对原生版本、退出码、候选实际解析、恢复节点分类与未验收状态。原生对照需显式提供 `CODEGUARD_RUBY_BIN`；同一固定语料的 worker 回归进入常规 CI。

本机 Ruby 为 `2.6.10p210`，`/usr/bin/ruby` SHA-256 `8404c9d5fd2200e82c2520c8d9c1badf2a164c0da7c350b54e86cb0025ad9b50`。运行：

```bash
CODEGUARD_RUBY_BIN=/usr/bin/ruby \
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test ruby_native_differential -- --ignored --nocapture
```

结果：常规 worker 回归 1/1、显式原生差分 1/1 通过；8 份合法与 5 份破损样例分类一致。语料覆盖类和模块、块、哈希、插值、heredoc、后置条件，以及缺少 `end`、未闭合字符串、破损定义/调用和缺失表达式。报告始终 `grammar_qualified=false`、`delivery_decision=not_evaluated`，退出码 3。

这仅是 Ruby 2.6.10 的窄范围语法分类证据；不覆盖其它 Ruby 版本、项目 DSL/模板、语义、规则 lint、系统误报漏报率或公开发行。
