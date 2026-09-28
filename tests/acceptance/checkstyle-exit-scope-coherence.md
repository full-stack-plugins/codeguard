# Checkstyle 原生退出与范围一致性

对应 OpenSpec 6.3。evaluate_checkstyle_report 接收原 XML、独立版本、冻结的完整文件列表及正常退出码；范围缺失/重复/不符、异常、坏报告及退出冲突保持未完成。解析得到的有效诊断保留为局部证据，不把这些条件转换为源码问题。只有本地一致性值，不授予 freshness、规则、工具身份、白名单或交付权威。

根据 [10.21.4 Main](https://github.com/checkstyle/checkstyle/blob/checkstyle-10.21.4/src/main/java/com/puppycrawl/tools/checkstyle/Main.java) 的原生计数退出，当前只对该版本 Unix 判定错误数低八位与退出相符。warning/info 退出零仍保留诊断；256 条 error 的退出截断不得变成零问题。其它版本/平台保留 exit_contract_unverified，不套用未经验收的契约。

目标测试先因缺 API 编译失败，实现后四项涵盖原生 warning/info、精确 error 数/退出、完整文件范围、信号/无退出、异常/版本失配及 Unix 256 计数。256 样本为协议夹具，不是原生 256 条错误执行证据。

已明确运行固定 Checkstyle 10.21.4/Java 21，经统一 runtime 使用原规则进行三轮：error 违规、完整文档零诊断、相同三条诊断改为 warning 且退出零。三轮均用实际 XML 调用新判定，1 项真实测试通过。原项目配置与输入身份/范围权威、其它注释变体、正式 CLI/任务/白名单/门禁仍缺，6.3 不勾选。
