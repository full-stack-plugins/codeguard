# Checkstyle 注释变体与原生例外

对应 OpenSpec 6.3。静态配置现接受 MissingJavadocMethod 与 JavadocType 原模块，与已有 MissingJavadocType/JavadocMethod 一并运行；不改写配置、不关闭规则。新增模块配置识别先失败，加入已验收模块后通过。

真实 10.21.4/Java 21 正式 CLI 样本包含 record 缺组件 @param 与补全、缺方法文档、带参数返回的继承方法 {@inheritDoc}、@Override 默认例外、@Generated 默认例外及移除注解后的缺类型文档。所有结果仍 coverage_proven=false、交付未评估；原工具的默认例外不被 Codeguard 解释成白名单批准或完整覆盖。

两次目标断言失败带来独立观察：类/方法同在一行时，原工具返回 methodTags 参数/返回诊断；分开类声明但保留单行方法体时，缺方法文档规则可能没有诊断。10.21.4 MissingJavadocMethod 源码的行数计算/默认 minLineCount 解释后一行为。保留这两个样本，再用多行方法体验证 missingMethod；不通过删除诊断或自动白名单使测试通过。精确语义误报裁定仍须独立复现/审核。

另外明确调用本机 Lombok 1.18.38 的 delombok，JAR 本地 SHA-256 为 1e1e427c36ff63c44fd30ef292d9e773ea3154460ab6265d3fed7e6f5bc50fb9。原 @Getter 源码没有显式 getter，局部扫描零诊断；实际展开后确认 getValue() 存在，再用正式 CLI 检出原配置的 missingMethod，路径指向展开文件，不能把它错报到原注解行。delombok 只在显式验收调用中执行，产品没有自动生成、编译或改源；本地摘要不是发行批准。

这些有界样本不覆盖所有继承、record 构造器、Lombok 注解/版本/编译类路径、生成代码来源和资源闭包。完整项目与规则/范围、任务/白名单/门禁仍缺，6.3 不勾选。
