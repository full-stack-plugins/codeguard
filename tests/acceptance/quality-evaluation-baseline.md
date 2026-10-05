# 分层质量评测计算切片

Rust Core 的 `evaluate_quality` 是不读文件、不调用工具的纯计算器。每条案例按 cohort、语言、类别、适配器分层，独立 holdout 不并入控制样本；使用独立裁定的精确 finding 实例集合计算 TP/FP/FN，只在相同目标覆盖且双方完整时计入精度和召回。工具故障被误判完整、真问题空报告和预置问题漏报分别计数；争议、未裁定、覆盖变化和未完成案例不会通过删除分母来提高精度。零实际发现的 precision 为不可估计，逐层样本不足为 `InsufficientEvidence`；整体结果不能优于任一失败或证据不足的层。

目标测试先因模块缺失失败；随后覆盖 TP/FP/FN、95% Wilson 区间、零分母、争议/覆盖差异、工具故障误判、漏报、holdout 隔离及坏门槛/重复样本。这里只验证计算语义，不证明案例 oracle、策略门槛或 holdout 已获独立批准。正式任务 12.2 的冻结配置、12.11 的真实回放和归档仍需实现；本切片不能用作“误报率已经达标”的发布声明。

2026-10-04：新增 `OracleDecision::Regression` 与独立 `regression_labeled_cases`，开发回归标签不计入 `adjudicated_cases`。即使该层具有足够预测和 Wilson 区间，也因缺独立裁定保持证据不足；已知漏报等失败仍优先保留。对应红测先因枚举/字段缺失失败，再经计算测试通过，避免开发回放将本地标签映射成授权 oracle。全 32 grammar 的实际开发回放与边界见 [验收记录](grammar-regression-evaluation.md)。
