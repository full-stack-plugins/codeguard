# JavadocMethod 原始标签检查配置验收

固定 Java 21 和 Checkstyle 10.21.4。配置 API 与可接受 token 已从本地固定制品的 javap 输出核对，不借用新版行为。JavadocMethod 支持 accessModifiers、allowMissingParamTags、allowMissingReturnTag、validateThrows、allowedAnnotations 和 METHOD_DEF/CTOR_DEF/ANNOTATION_FIELD_DEF/COMPACT_CTOR_DEF；未知值及跨模块专属属性保持未完成。

## TDD 与原生行为

method_tag_configuration_retains_native_options_and_context 先因拒绝合法配置失败。适配后八项绑定与四项静态配置回归通过。

真实 CLI/workbench 测试 original_method_tag_options_control_native_diagnostics_and_repair 使用公开 compute/skipped、私有 helper 与公开 throws 方法。均带概要 Javadoc，缺参数/返回或异常标签，原配置 accessModifiers=public、自定义 ID=methodTags：

| 原配置追加参数 | 原生诊断数量 |
|---|---:|
| 无 | 4 |
| allowMissingParamTags=true | 2 |
| allowMissingReturnTag=true | 2 |
| 两项均 true | 0 |
| allowedAnnotations=SkipDocs | 2 |
| validateThrows=true | 5 |

私有 helper 不被额外补报，原生诊断的规则 ID 保留。全程使用原 XML，不为允许缺标签的配置生成额外 Rust 违规；未完成覆盖标记保留。恢复首次配置后，task verify 确认原问题仍在；实际补全两处公开方法参数和返回标签后，复检为 candidate_absent_unverified_policy，事件保存，所有事实仍 open。

六次原生扫描与两次复检共八轮通过，57.23 秒。此验收不证明所有访问修饰符、所有 token 语法、异常类型继承、有效 Maven/Gradle 项目配置或受批准规则覆盖。允许缺标签是原生配置语义，不是误报白名单批准；正式关闭与项目门禁仍待实现。
