# `check all` 的 Python 原生优先局部验收

日期：2026-09-29。对应 OpenSpec 14.6、14.7 的局部增量，父任务仍未完成。

源码可选构建在 `check all` 的原生节点之后，仅当本轮 Ruff 对同一路径返回 `passed`、`findings` 或 `suppressed`，且报告源码 SHA-256 与候选阶段重新读取的字节一致、工具和配置摘要均存在时，跳过该 Python 文件的重复 WASM。`incomplete`、路径不一致或源码变化不能触发跳过。该跳过只代表本轮原生 Python 语法解析已执行，不代表其它质量类别通过。

真实 CLI 集成样例包含 Python 原生 F401 与 Zig 源码：F401 保留于 `native_results`；Python 不出现在候选 `observations`，`native_preferred_count=1`；Zig 仍由隔离 worker 解析，整体 `delivery_decision=incomplete` 且退出码 3。另有单元反例覆盖原生状态不完整、路径错配、源码摘要变化和非 Python 扩展名。聚合协议从 0.32.0 升至封闭的 0.33.0，并归档旧 schema；默认无 WASM 构建也输出新的计数字段 0。

这不等于 32 种语言均已具备逐模块原生优先选择。Java P3C 等规范检查不能单独确认语法；Go、Rust、TypeScript 等原生能力的项目范围和源码身份仍待逐语言对照。语法规则版本语料、任务/宿主反馈和发行验收也尚未完成。

验证：`cargo test -p codeguard-cli --features wasm-precheck` 完整测试集退出 0（需要外部工具的用例按原标记忽略）；32 grammar 分批真实 CLI 回归通过。本机 Ruff 0.16.8 的原生用例补加计数与不重复解析断言后，经显式 `CODEGUARD_RUFF_BIN` 重跑通过。真实 Zig 项目输出通过 Draft 2020-12 的 0.33.0 Schema，伪造 `allow` 被拒；Clippy 全目标 `-D warnings`、OpenSpec strict 与 crate 分层检查通过。默认无 WASM 构建的受影响聚合、Java/P3C 与 npm 测试均退出 0。
