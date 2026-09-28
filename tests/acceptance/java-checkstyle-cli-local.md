# Java Checkstyle 正式局部命令

对应 OpenSpec 6.3。正式入口为 `codeguard lint java FILE --checker checkstyle --java-tool PATH --checkstyle-jar PATH --config PATH --format json`。三个工具/配置选项显式选择，不寻找或下载默认工具，不替换原配置。缺前置返回结构反馈与退出 3；重复/未知选项或格式返回 2。取消返回 130。

当前只识别标准内置 DTD 的 Checker/TreeWalker/MissingJavadocType/MissingJavadocMethod/JavadocType/JavadocMethod 静态注释配置，以及字面 id/severity。动态属性、外部资源/实体、未知模块或其它配置保留 configuration_context_unresolved，不删去用户配置后执行默认规则。脚本 Java 入口缺隔离时拒绝。这里是局部支持边界，不表示完整 Checkstyle 配置适配完成。

源码和原配置按当前字节复制到私有目录；原 Java/JAR 及快照四个摘要进入已实现 runtime 服务，结束另复核原输入。诊断映射回原源码路径，保留完整原生 source、行/可选列、严重度；原消息及私有快照路径不进入公开投影。反馈 0.1.0 固定 checker_identity=unverified、coverage_proven=false、delivery_decision=not_evaluated；即使局部零诊断也不返回普通质量通过。

命令行为先因未知检查器而返回 2，目标断言失败；接入后缺前置与参数契约通过。实际 Checkstyle 10.21.4/Java 21 从正式 binary 运行公共类型缺注释样本，显示 1 条 publicType 原生诊断，退出 3。两份实际缺前置/原生报告通过 JSON schema，4 个伪造工具/覆盖/退出/交付状态反例拒绝。Javadoc CLI 既有回归和静态配置识别正反例通过。

完整配置/资源闭包与 JDK 运行时、网络隔离、项目完整模型/范围及可信策略、其它注释变体、check java/check all、任务/白名单/门禁还未接通，6.3 不勾选。
