# Javadoc 项目模式选择及无关单文件阻塞纠偏

对应 OpenSpec 6.3，正式 Java 注释能力未完成。

明确提供 Maven 执行上下文时，项目 check 的 Javadoc 只选择原 POM 多文件探针；每个文件保留路径/配置身份，但单文件 observation 为空并指向 maven_multifile_probe_selected。前置缺失或原 POM 不适用保留多文件具体原因，不回退到孤立 JDK。未选择 Maven 时仍支持原 JDK 单文件局部模式及显式 lint java --checker javadoc。多文件观察数量来自本轮原生结果，coverage_proven=false、项目归属未核验和 delivery_decision=not_evaluated 保持。

真实反例：First 源码引用同项目 Second 时，旧 check 在 Maven 多文件结果之外额外运行两次 JDK 单文件探针，First 因找不到 Second 返回 native_execution_incomplete。新增目标断言先失败，修正后本机 Maven/JDK 21/固定离线插件仓的缺注释及文档完整两轮成功：只保留多文件原生诊断及本轮观察，不再产生该单文件失败。第二次运行用测试编译的显式报告目录保留原始 JSON，两个实际原生反馈通过 schema，四个混合模式/错检查器反例拒绝。

项目反馈升级 0.23.0，Javadoc 子报告升级 0.3.0，新增 probe_mode 与相符 checker_id。旧 check-feedback-v0.22.schema.json 独立保留并拒绝新版输出；新版 schema 禁止 Maven 模式嵌入单文件 observation 或伪装 JDK 检查器。模拟 Maven 前置缺失的哨兵确认不会启动 JDK；报告退出 3，不把空 observation 或仅单轮零诊断当作项目通过。

初次并行回归出现两个失败：schema 测试仍期待旧 Javadoc 0.2，已更新；取消用例未在其两秒窗口内观察到两个原生任务启动，单独重跑通过，不能凭重跑判定根因已解决。完整结果须结合随后串行回归，取消窗口问题保留为未定位限制。

Checkstyle、生效模型/完整依赖类路径、生成代码、Lombok/record/inheritDoc 全集、正式任务/白名单/门禁仍缺；当前简单 POM 多文件探针不扩展为所有 Maven 工程完成。未运行全 workspace，6.3 不勾选。

最终验证：Java check 24、partial 13（随后串行）、Javadoc CLI 4，共 41 项普通测试通过；另 3 项明确执行的真实 Maven/JDK 测试通过。2 个实际 Maven 报告与 1 个实际 JDK 单文件报告通过新 schema，4 个混合模式/错检查器反例被拒，旧 schema 拒绝新版。CLI all-target Clippy -D warnings、格式、OpenSpec 严格校验和插件 diff 检查通过。取消用例先前一次并行启动窗口失败仍未归因，不作为已解决问题。
