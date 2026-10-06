# Gradle Javadoc类别归属修复

对应introduce-rust-codeguard-cli/native-tool-adapters的公开Gradle文档归属要求和15.3。已有0.63实际报告中，missing-native为findings_observed_unverified且3条诊断，documented-native为empty_output_unverified且0条，但Java/comments类别都为not_configured、checker_id=null。这会错误引导安装或修改Maven配置。新公开测试先因checker_id为null而RED；随后修复按显式请求的Gradle适配器归属局部结果。纯静态Gradle配置未知另复现not_configured的RED，现保持configuration_unresolved并给出按最近构建根核验Maven/Gradle配置的指引；不增加原生任务或新字段。

check java/all --gradle-javadoc现在使用check_feedback0.64；不是新命令，不改0.63历史schema/报告。未请求文档检查的模型路径仍0.62；异常反馈仍0.17。已观察原生诊断/空输出且任务成功时，类别为observed_unverified；缺工具、超时、取消等为native_incomplete。两者标识java.gradle.javadoc，并给出读取具体Gradle阻塞或按相同Gradle/JDK/输入原工具复检的指引。此ID表示明确请求的适配器，不保证所有原项目配置已完整覆盖。

实际0.64报告类别摘录如下，省略整体报告，不作为完整协议输入：

```json
{
  "language": "java",
  "category": "comments",
  "checker_id": "java.gradle.javadoc",
  "status": "observed_unverified",
  "reason": "gradle_javadoc_selected_inputs_and_rules_unverified"
}
```

缺工具/取消的同类摘录使用native_incomplete/gradle_javadoc_native_incomplete。具体native.reason仍保存在原生观察内；不能把执行失败当源码违规、配置缺失或重复修改无关源码。原生字段coverage_proven和rule_configuration_complete仍为false；候选类别不能替代Maven/Gradle所有项目义务或签发质量通过。

默认两个Gradle目标12通过、0失败、5条件忽略；Maven/Gradle同根及语言选择两个回归目标另6通过、0失败、1条件忽略；真实已有Gradle8.10.2/JDK21公开条件测试1通过，两组3/0诊断并逐组断言类别、规则及提示，仍退出3。WASM相同两个目标12通过、0失败、5条件忽略，与默认重叠。本批未重跑内部服务四组条件测试或完整工作区，不累计为本次精度验收。

六份实际CLI报告（Java/all缺工具、实际有/无诊断、SIGINT及未运行原生的静态配置未知），五份显式请求通过0.64、静态观察通过既有0.61，见[报告包](evidence/gradle-javadoc-category-reports-2026-10-06.json)。类别的错误状态/检查器/语言/类别/原因及原生覆盖/规则/交付提升共九种伪造被拒绝，旧0.63消费者拒绝新报告；416份schema元定义有效。默认/WASM CLI全目标Clippy、分层/OpenSpec strict/diff通过，见[摘要](evidence/gradle-javadoc-category-2026-10-06.json)。

工作台接线前发现并优先修正此错误，不假装已完成稳定Gradle任务或复检关闭；这两项及完整详细注释规则、完整源码/配置/JDK、多项目/custom doclet和57语言生产矩阵仍待完成。父任务15.3未勾选，语法资格0/32未改变。本批没有安装下载、发布/合并或执行/修改受保护Erlang草稿，未核实本次远端CI结果。
