# Zig 0.16.0 原生 AST 与内置 WASM 的局部差分验收

日期：2026-10-03。对应既有 OpenSpec 14.4、14.17、14.19 的局部精度证据，任务均未完成。

新增被显式忽略的真实原生测试 `zig_native_differential`。运行时须提供现存 `CODEGUARD_ZIG_BIN`；测试先核对 `zig version` 为 0.16.0，再将完全相同的 UTF-8 源码分别送入 `zig ast-check` 的 stdin 和 CodeGuard 公开 `grammar probe zig` 的隔离 WASM worker。独立原生结果退出码 0/1 与候选恢复节点的有无逐例比较。测试还核对候选始终 `grammar_qualified=false`、`delivery_decision=not_evaluated`，避免差分一致被误当成交付通过。

本机所用 Zig 为 Homebrew 0.16.0，程序 SHA-256 `71cc3995a7586753ebf82c66dfb8bef43df446517550678781834586a960f8c9`。执行：

```bash
CODEGUARD_ZIG_BIN=/opt/homebrew/bin/zig \
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test zig_native_differential -- --ignored --nocapture
```

结果：1 项真实原生差分测试通过，11 份样例分类一致，其中 7 份合法样例覆盖空 struct/enum/union/opaque、函数、中文字符串和嵌套容器；4 份破损样例覆盖缺失大括号/右括号/初始化表达式和未闭合字符串。实际 worker 参与对照，而非只测试裸 parser 或资产清单。

这只能证明上述 11 个 Zig 0.16.0 样例的一致性。`ast-check` 不进行完整类型检查、构建或 lint；没有其他 Zig 版本、项目配置、宏/嵌入方言、系统误报率/漏报率、冷暖性能或公开发行验收。CFQuery 与 VB.NET 的既知精度缺口也不因 Zig 通过而改变。完整 S14.17、14.19 仍不勾选。
