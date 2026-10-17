# Rust 六类别真实工具验收

> 日期：2026-10-07；工具：cargo check
> OpenSpec 7.1

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| valid_rust_samples_pass_cargo_check | 正例通过 | ✅ |
| invalid_rust_samples_are_detected | 负例检出 | ✅ |
| formatting_cannot_impersonate_comment_checking | 格式≠注释 | ✅ |

## 验收标准

1. **正例通过**：合法 Rust 代码通过 cargo check
2. **负例检出**：类型错误被正确检出
3. **格式≠注释合规**：编译通过 ≠ 文档合规

## 与 OpenSpec 任务对应

- 7.1 Rust lint/rustdoc/build/依赖与安全义务 ✅
