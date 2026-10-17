# Java 六类别真实工具验收

> 日期：2026-10-07；工具：javac 21
> OpenSpec 6.x / 8.x

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| valid_java_samples_pass_compilation | 正例通过 | ✅ |
| invalid_java_samples_are_detected | 负例检出 | ✅ |
| formatting_cannot_impersonate_comment_checking | 格式≠注释 | ✅ |
| public_api_requires_javadoc | 公共 API 需文档 | ✅ |
| override_methods_accept_inheritdoc | 继承文档 | ✅ |

## 验收标准

1. **正例通过**：合法 Java 代码通过 javac 编译
2. **负例检出**：类型错误被正确检出
3. **格式≠注释合规**：编译通过 ≠ 文档合规
4. **公共 API 需文档**：公共方法必须有 Javadoc
5. **继承文档**：@Override 方法可继承父类文档

## 与 OpenSpec 任务对应

- 6.3 Checkstyle/Javadoc 注释检查 ✅
- 8.x Java 六类别适配 ✅
