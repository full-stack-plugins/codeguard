# Go 1.23.4 原生 gofmt 与内置 WASM 的局部差分验收

日期：2026-10-03。对应既有 OpenSpec 14.4、14.17、14.19 的局部精度证据，三项任务仍未完成。

新增显式忽略的真实原生测试 `go_native_differential`。它要求现存 `CODEGUARD_GO_BIN` 与同目录的 `CODEGUARD_GOFMT_BIN`，先核对 `go version` 为 Go 1.23.4，再将同一 UTF-8 源码送入 `gofmt -e` 的 stdin 与公开 `grammar probe go` 隔离 worker。测试要求原生退出码为 0 或 2、候选实际解析且没有输入路径错误，并逐例比较原生成功与 WASM 恢复节点的有无。候选仍必须返回 `grammar_qualified=false`、`delivery_decision=not_evaluated` 与退出码 3。

本机工具为 Go 1.23.4 darwin/arm64，`go` SHA-256 `5900a8e0942f88b7bacf2b44b397b4950c8dc9f850a16ae1249e1636860dd6ef`，`gofmt` SHA-256 `f0e9b77df6dcfeec18d33bd9305ff034c28aa2142d3a4fa3aa44e62a44c89e3b`。执行：

```bash
CODEGUARD_GO_BIN=/usr/local/go/bin/go CODEGUARD_GOFMT_BIN=/usr/local/go/bin/gofmt \
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test go_native_differential -- --ignored --nocapture
```

结果：真实原生差分测试 1/1 通过，常规 CI 可运行的 WASM 固定语料回归 1/1 通过，13 份样例分类一致。8 份合法样例覆盖空函数、泛型、整数 range、Unicode 标识符、原始字符串、复合字面量、接口和 type switch；5 份破损样例覆盖缺少大括号/圆括号、未闭合字符串、缺失初始化表达式和破损 if。测试规范化临时目录路径，避免 macOS `/var` 别名被 CLI 的符号链接防护拒绝；并断言报告无输入错误，防止“未执行”被误算为一致。

`gofmt` 只提供语法解析对照，不执行类型检查、`go vet`、测试或完整构建。此结果不证明 Go 其它版本、构建标签、生成代码、项目配置、规模化误报漏报率或公开发行资格。其余 31 种 grammar 的精度不受该局部证据覆盖。
