# 计划语言的显式能力缺口

COBOL、ArkTS、Metal 在旧注册表中均为 `planned`，当前六类别能力均为 `gap`。项目含 `.cbl`、`.ets`、`.metal` 源码时，`codeguard check all --format=json` 仍发现这些语言，但不启动空适配器或伪造无问题结果；每个语言的六类别候选标明 `legacy_status=planned`、`capability_status=gap`、`status=not_integrated`、`reason=planned_language_adapter_gap` 和下一步，整次请求退出 3、交付 incomplete、必需义务仍未冻结。跨平台能力不一致时，候选的单一能力字段为 null，调用方应查询平台能力矩阵。

集成测试 `planned_language_gaps` 使用真实临时项目，逐一核对三语言的六个候选、待解析条件以及 `capabilities` 查询中的 planned/gap。测试先因检查反馈缺少显式状态而失败，补充协议后通过。`check_feedback` 协议升至 0.17；`legacy_status` 只是迁移标签，不能证明任何原生能力或授权跳过义务。
