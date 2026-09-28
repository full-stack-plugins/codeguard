# 初始化画像对话反馈

范围：OpenSpec 9.25 的静态摘要反馈切片。完整 readiness、准备任务和宿主接线仍缺，不勾选整项。

init_plan 0.4 在 dry-run/apply 返回相同结构的 profile_summary：语言与源码/清单数量、构建根/清单引用、逐清单目标声明及未知项。human 直接投影同一结构；所有路径/字段以 JSON 字符串转义，不可信换行/ESC 不能伪造终端行。已创建/冲突路径也改为转义显示。摘要不含原始清单和本机根路径；本机版本 not_probed、构建模型 not_resolved、架构 unknown。初始化仍不证明准备完成或质量通过。历史 init_plan 0.3 schema 单独保留。

新增 dry-run 用例先因旧输出 0.3 无摘要而失败。实现后验收 dry-run 不创建 codeguard/AGENTS、dry-run/apply 摘要一致、JSON/human 同一语言目标和未知状态、本机根路径不公开、含换行/ESC 的真实项目目录不能生成 FORGED_STATUS 行。上述行为以真实 CLI 临时项目验证，不是伪造报告输入。

真实 dry-run 输出通过 JSON Schema，6 个伪造工具就绪/架构确认/模型已解析/目标确认/缺摘要/交付 allow 反例拒绝，历史 0.3 正例有效。schema 验证辅助不作为 Python 产品实现。

受影响 CLI 契约 74 项（init 36、detect 20、check-plan/config 各 6、边界/plan 各 3）与 CLI 库 18 项通过。workspace all-target Clippy（-D warnings）、格式和 OpenSpec strict/diff 检查终态通过。没有运行全 workspace/所有原生检查/宿主对话链；readiness 仍 unknown、交付 not_evaluated，完整计划继续实施。
