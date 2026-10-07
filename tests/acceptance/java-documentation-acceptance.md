# Java 详细文档注释验收（15.3）

> 对应 OpenSpec 15.3。按 Java 语言规范检查 Javadoc 用途、参数、返回、
> 错误及行为契约；覆盖类型/方法/字段 Javadoc 及 Maven 项目配置；
> 裸标签或空注释不能冒充合规。

## 测试结果

| 测试 | 覆盖维度 | 状态 |
|---|---|---|
| `complete_javadoc_passes` | 完整 Javadoc（用途/参数/返回）通过 | ✅ |
| `missing_method_documentation_detected` | 缺方法文档检出 | ✅ |
| `missing_type_documentation_detected` | 缺类型文档检出 | ✅ |
| `bare_tag_cannot_impersonate_compliance` | 裸标签/空注释不冒充合规 | ✅ |
| `line_comment_cannot_impersonate_javadoc` | 行注释不冒充 Javadoc | ✅ |
| `maven_project_javadoc_check` | Maven 项目配置下检查 | ✅ |
| `missing_param_tag_detected` | 缺 @param 标签检出 | ✅ |
| `missing_return_tag_detected` | 缺 @return 标签检出 | ✅ |
| **总计** | **8/8** | **✅** |

## 全链路验证详情

### 1. 用途/参数/返回完整性
- 完整 Javadoc（类描述 + 方法描述 + @param + @return）不产生缺文档 finding
- 缺方法文档、缺类型文档分别被检出

### 2. 裸标签/空注释不冒充合规
- 空 Javadoc（`/** */`）被检出
- 裸标签（`@param a` 无描述）被检出
- 普通行注释（`// comment`）不能替代 Javadoc

### 3. 项目配置
- Maven 项目（pom.xml + maven-javadoc-plugin）正确处理
- 检查命令返回有效 report_type

### 4. 标签完整性
- 缺 @param 被检出
- 缺 @return 被检出

## 复现命令

```bash
cargo test --package codeguard-cli --features wasm-precheck \
  --test java_documentation_acceptance --locked --offline
```

## 边界

- 仅覆盖文件级和 Maven 项目级检查，Gradle 项目配置另行验收
- 详细文档规则（@throws/@since/@deprecated 等）的完整规则集未验收
- 原生 Javadoc 工具（javac/javadoc）集成需 --java-home 参数
- 宿主对话接入、可信关闭/复发、发行包实装仍未完成
