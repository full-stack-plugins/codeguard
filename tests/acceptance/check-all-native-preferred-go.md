# `check all` 的 Go 原生包范围优先局部验收

日期：2026-10-03。对应 OpenSpec 14.6、14.7、14.19 的局部增量，父任务仍未完成。

原 `check all` 即使本轮 Go 1.23.4 `go vet` 完成，也对所有 `.go` 文件重复运行候选 WASM。直接按整体 vet 状态跳过又会遗漏构建标签排除的文件。RED 的真实 Go 项目包含普通 `main.go` 和 `//go:build never` 下的破损 `excluded.go`，期望仅前者由原生检查覆盖；旧实现的 `native_preferred_count` 为 0。

现在候选阶段在原生 vet 之后，复核同一工具 SHA-256、Go 版本、全部模块身份及源码快照，再用受控 `go list -json ./...` 按模块取得默认构建中的普通/测试源码与排除源码。只对清单中选中的、源码字节仍匹配的文件跳过重复 WASM；排除或未列出的 Go 文件仍运行候选 worker。任一包清单越界、工具/源码变化、模块状态不完整、超时或清单解析失败，都不授予跳过。清单按模块一次运行，不对每个文件重复启动 `go list`。原生发现保留，整体仍 `incomplete`、退出码 3；默认宿主平台和构建标签之外的覆盖不宣称完成。

验收：本机 Go 1.23.4 的显式真实集成测试 1/1 通过：`main.go` 的原生 `printf` 发现完整保留并被原生优先计数，带构建标签且有恢复节点的 `excluded.go` 仍出现在候选观察；伪造越界包路径的常规集成回归 1/1 通过，vet 局部状态虽完整也不能跳过。相关 `check_all_grammar_candidates` 4/4 通过，包括四组项目实际调用全部 32 份候选。默认和 WASM 特性构建的 Clippy 全目标 `-D warnings`、格式及 OpenSpec strict 均通过。显式命令：

```bash
CODEGUARD_GO_BIN=/usr/local/go/bin/go \
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test check_all_native_preferred_go -- --ignored --nocapture
```

Go 的原生 vet/默认包清单不是全平台、全部构建标签或全部 lint 规则证明；工具制品和策略仍未获发行资格。其它语言的逐文件原生优先、任务/宿主反馈及正式语法能力验收仍缺，因此 14.6、14.7、14.19 不勾选。
