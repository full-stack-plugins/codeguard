# Checkstyle 原生 XML 局部解析

对应 OpenSpec 6.3；当前只实现 Rust 纯解析，尚未接通正式 Checkstyle 执行。

依据官方 [报告协议](https://checkstyle.org/result-reports.html) 和 [XMLLogger](https://github.com/checkstyle/checkstyle/blob/master/src/main/java/com/puppycrawl/tools/checkstyle/XMLLogger.java)：保留 error 的完整 source（包括自定义模块 ID）、原生 severity、位置与解码后的消息；无 column 不补造列号，line=0 保持文件级观察。exception 和没有文件归属的根级事件保留为处理未完成，不制造源码违规。有效空文件保留范围列表，不授予规则、覆盖或门禁权威。

测试先因缺 parse_checkstyle_xml API 编译失败；实现后覆盖原生字段、自定义 ID、文件级位置、可选列号、异常分流、零诊断文件、错版本保留证据、未知/损坏结构、重复文件、DTD/编码/数值异常和规模限制。所有样本为手工协议夹具，不是实际原生工具执行证据。

后续已明确执行固定 Checkstyle 10.21.4/Java 21 的真实类型及参数返回正反例，复用统一 runtime 本轮报告/日志，见 checkstyle-native-command.md。该证据与本文件的手工协议夹具分开；真实项目原配置及范围/身份绑定、inheritDoc/record/Lombok/生成代码正反例，以及配置、任务、白名单与门禁接线仍缺，6.3 不勾选。
