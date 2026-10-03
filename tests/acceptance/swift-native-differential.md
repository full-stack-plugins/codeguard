# Swift 6.4 原生语法差分发现固定 WASM 漏检

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的精度阻塞，父任务仍未完成。

同一组 13 份 `.swift` 源码分别交给现存 Apple Swift 6.4 的 `swiftc -frontend -parse` 与公开 `grammar probe swift` 隔离 WASM worker。8 份合法、5 份破损；原生标注均由编译器确认。12 份分类一致，1 份破损源码不一致：

```swift
func f(_ x: ) {}
```

编译器在第 1 行第 13 列报告 `expected parameter type following ':'` 并退出 1；固定 WASM 返回零恢复节点。常规 CI 单例回归确认这一已知漏检不会获得 `grammar_qualified=true` 或交付通过，显式原生测试锁定 13 例中只存在这个已知不一致项，以便 grammar 修复时主动重新评估。它是当前候选的**漏报反例**，不是合法源码误报，也不能靠删除测试或加白名单放过。

本机 `swiftc --version` 为 Apple Swift 6.4，`/usr/bin/swiftc` SHA-256 `34129c71a01a74f7f3b2443521519b2e5447553fa187f5fcafaaf8c42cc192e2`。运行：

```bash
CODEGUARD_SWIFTC_BIN=/usr/bin/swiftc \
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test swift_native_differential -- --ignored --nocapture
```

本机显式差分 1/1 通过，用时 68.56 秒；其通过表示**成功识别并保留漏检**，不表示 Swift 语法验收通过。修复需针对固定来源 grammar 及版本/方言语料，重新构建可追溯 WASM，再重复原生差分、资源及发行包验证。当前不得将 Swift WASM 作为 lint 或语法 clean 的依据。
