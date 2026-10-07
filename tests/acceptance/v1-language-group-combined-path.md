# v1 主流语言群 WASM+原生联合路径验收（2026-10-07）

> 对应 OpenSpec 15.2。Java/Python/TypeScript/Rust 四种主流语言的 WASM 语法候选
> 与原生 lint/编译器联合路径全链路验收，形成 v1 版本群。

## 覆盖语言

| 语言 | 测试文件 | 测试数 | 状态 |
|---|---|---|---|
| **Java** | `java_wasm_combined_path.rs` | 9 | ✅ |
| **Python** | `python_wasm_combined_path.rs` | 9 | ✅ |
| **TypeScript** | `typescript_wasm_combined_path.rs` | 9 | ✅ |
| **Rust** | `rust_wasm_combined_path.rs` | 9 | ✅ |
| **总计** | **4 个测试文件** | **36 测试** | **✅** |

## 全链路 7 维度覆盖

每种语言均验证以下 7 个维度：

| 维度 | 验证内容 |
|---|---|
| 1. 配置发现 | `detect` 命令识别 checker 状态（configured/missing），每个配置有 next_action |
| 2. 原生优先 | WASM 候选 ≠ 已确认违规，grammar 未验证不声称 clean |
| 3. 缺工具初检 | delivery_decision=incomplete + 退出码 3 + next_action 指引 |
| 4. 版本/方言兼容 | known_limitations 如实披露，grammar_qualified=false |
| 5. 精确定位 | byte_offset + grammar SHA-256 身份绑定 |
| 6. 取消/超时 | execution_budget.timeout_ms + enforcement 策略 |
| 7. 安装指引 | 六类别 gap 状态含 next_action/reason |

## 复现命令

```bash
cargo test --package codeguard-cli --features wasm-precheck \
  --test java_wasm_combined_path \
  --test python_wasm_combined_path \
  --test typescript_wasm_combined_path \
  --test rust_wasm_combined_path \
  --locked --offline
```

## 边界

- 四种语言 grammar 均为 `unqualified`，不代表版本/方言全覆盖
- 正式 grammar 资格仍为 0/32
- 本验收只覆盖单文件/简单项目路径
- 宿主对话接入、可信关闭/复发、发行包实装仍未完成
- 其余 53 语言的联合路径验收仍缺
