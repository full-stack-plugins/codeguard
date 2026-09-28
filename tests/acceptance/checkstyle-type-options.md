# 原生类型文档参数与修复工作台验收

Java 21、固定 Checkstyle 10.21.4，API 与可接受 token 从本地制品 javap 核对。MissingJavadocType/JavadocType 的 scope/excludeScope 和五种类型 token 原样保留。MissingJavadocType 专属 skipAnnotations，JavadocType 支持 allowedAnnotations、allowMissingParamTags、allowUnknownTags、authorFormat/versionFormat。正则解释由原生工具完成，不做 Rust 替代规则。

类型参数绑定测试先因拒绝 MissingJavadocType 合法配置失败，修改后绑定九项/静态配置四项通过。类型专属属性借给方法检查器及 METHOD_DEF 借给类型检查均拒绝。

## 显式原生矩阵

original_type_options_select_native_documentation_tasks 使用公开外部类型、公开/私有嵌套类型、带 SkipDocs 注解类型、record 与 interface。缺类型文档的四组结果：

| MissingJavadocType 原配置 | 原生诊断数 |
|---|---:|
| scope=public | 5 |
| public + skipAnnotations=SkipDocs | 4 |
| private + excludeScope=public | 1 |
| tokens=RECORD_DEF | 1 |

随后源码切换为有概要 Javadoc 的 record（缺参数标签、带 SkipDocs）和嵌套类型（带自定义未知标签）。

| JavadocType 原配置 | 原生诊断数 |
|---|---:|
| scope=public | 2 |
| public + allowMissingParamTags=true | 1 |
| public + allowUnknownTags=true | 1 |
| public + 两项允许同时 true | 0 |
| public + allowedAnnotations=SkipDocs | 1 |
| tokens=RECORD_DEF | 1 |

诊断保留自定义规则 ID，没有配置 blocker 或额外 Rust 诊断。恢复 typeTags 首次配置，补 record 参数说明并移除未知标签后，task verify 为 candidate_absent_unverified_policy、事件保存，所有历史事实保持 open。十轮扫描与一轮复检通过，58.02 秒。

## 证据边界

上述十一轮为已执行原生测试，作者/版本正则参数只验证静态配置接受与模块约束，未运行完整原生格式正反例。tokens 原生执行仅覆盖 record 选择，五类 token 身份来自固定制品及静态绑定，并不等于五类全部语法均验收。完整项目有效模型、其它 Scope 枚举、原生配置引用资源、批准必需规则覆盖与正式门禁仍未完成。
