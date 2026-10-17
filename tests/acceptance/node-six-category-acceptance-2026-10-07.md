# Node.js 六类别真实工具验收

> 日期：2026-10-07；工具：node --check
> OpenSpec 7.3

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| valid_node_samples_pass_syntax_check | 正例通过 | ✅ |
| invalid_node_samples_are_detected | 负例检出 | ✅ |
| formatting_cannot_impersonate_comment_checking | 格式≠注释 | ✅ |

## 验收标准

1. **正例通过**：合法 JS 代码通过 node --check
2. **负例检出**：语法错误被正确检出
3. **格式≠注释合规**：语法正确 ≠ 文档合规

## 与 OpenSpec 任务对应

- 7.3 Node/TypeScript/JavaScript adapter ✅
