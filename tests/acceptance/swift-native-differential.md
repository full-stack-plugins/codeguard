# Swift 6.4 原生语法差分与隐藏错误覆盖

初始日期：2026-10-03。更新：2026-10-04。对应 OpenSpec 14.4、14.17、14.19 的精度阻塞，父任务仍未完成。

## 当前结论

2026-10-04 重新检查固定 WASM 的真实树：`func f(_ x: ) {}` 的根节点 `has_error=true`，S-expression 含隐藏 `MISSING "simple_identifier_token1"`，但遍历没有可枚举的缺失节点。既有 Rust 扫描器已将这种情况标为 `truncated=true`；CLI 的 `precheck.truncated_files=1`、整体 `incomplete`，不是语法通过。

旧差分仅按 `recoveries.is_empty()` 分类，忽略完整性，因而把此例算成漏报。这轮将测试改为：先看扫描完整性，再给出合法/非法分类。13 例保留原有 8 合法、5 非法原生标签；**12 例可判定且与原生一致，1 例未知（bad_param）**。未知样例仍占总语料分母，不能算作一致、通过或已确认源码违规；也没有修好或替换 grammar。

本机真实 Apple Swift 6.4 对照重新运行：1 passed、0 failed、0 ignored，用时 64.60 秒；所有源码原字节未变。固定语料回归 1 passed、0 failed、1 ignored；新增运行时隐藏错误回归所在文件 5 passed、0 failed。CI 加入固定语料和恢复扫描回归，真实 Swift 工具测试仍须显式选择；当前源码尚未进入新发行包。

## 历史测量与纠正依据

同一组 13 份 `.swift` 源码分别交给现存 Apple Swift 6.4 的 `swiftc -frontend -parse` 与公开 `grammar probe swift` 隔离 WASM worker。8 份合法、5 份破损；原生标注均由编译器确认。12 份分类一致，1 份破损源码不一致：

```swift
func f(_ x: ) {}
```

编译器在第 1 行第 13 列报告 `expected parameter type following ':'` 并退出 1；固定 WASM 返回零恢复节点。初始统计把这一项记为漏报；当前统计按上面的完整性信号将其纠正为未知。原生证据及固定 grammar 身份保留，不能靠删除测试或加白名单放过。

本机 `swiftc --version` 为 Apple Swift 6.4，`/usr/bin/swiftc` SHA-256 `34129c71a01a74f7f3b2443521519b2e5447553fa187f5fcafaaf8c42cc192e2`。运行：

```bash
CODEGUARD_SWIFTC_BIN=/usr/bin/swiftc \
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test swift_native_differential -- --ignored --nocapture
```

本机显式差分 1/1 通过，用时 68.56 秒；其通过表示**成功识别并保留漏检**，不表示 Swift 语法验收通过。修复需针对固定来源 grammar 及版本/方言语料，重新构建可追溯 WASM，再重复原生差分、资源及发行包验证。当前不得将 Swift WASM 作为 lint 或语法 clean 的依据。
