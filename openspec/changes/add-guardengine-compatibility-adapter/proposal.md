# CodeGuard opt-in GuardEngine compatibility adapter

## Why

CodeGuard 已有四 crate Rust workspace、版本化原生反馈、局部检查与修复工作台，但没有 GuardEngine 适配器。检查源码基线 `b499f13922647d4bf1e2344eaa5c37c450e302b5`；当前文档集成为 `509006d12c0b03eb661f3d809fd89a04cc78cb25`。`check_command.rs` 正常聚合反馈仍退出3；`Verdict` 和 `legacy_v1_protocol` 不能按 GuardEngine 数字直接解释。新接入需要无损投影和独立作用域证明，不能借此重写成熟实现或宣称完整 native coverage。

依据：[架构](../../../docs/architecture.md)、[技术设计](../../../docs/technical-design.md)、[共享草案](../../../docs/integration-contract.md)、[跨仓路线图](../../guard-roadmap.md)。此变更只规划，未实施、未完成任务，不改变已发布 npm 制品或当前命令。

## What Changes

- 增加默认关闭、只读既有证据的投影模块；按 command + flags + report schema + scope 登记能力，不隐式运行工具/安装/修复/同步任务。
- 冻结原始退出码、JSON/SARIF、stderr、导出和副作用兼容矩阵；只有新增明确适配入口采用0/2/3/4，原生取消130与历史入口数字不改写。
- 在可证明范围内生成严格 `guard.partme.ai/v1alpha1` facts/contract 输入，保留本域 finding；完整性不足为 partial/BLOCK，工具故障和取消单列，未支持投影不得生成假 ALLOW。
- 使用独立 `guard.integration/v1alpha1` 草案承载已冻结调用绑定、覆盖、摘要及批准引用；绑定前错误无信封，绑定后错误 decision=null。报告重算不授予身份、批准或合并权。
- 为控制器消费提供只读接口、失效/审计证据及并行/合并队列负例；逐步 shadow→opt-in，适配器可单独回滚。

## Capabilities

### New Capabilities

- `native-evidence-projection`：命令感知兼容、严格读取、已证明范围、关系投影与运行状态映射。
- `bound-projection-consumption`：候选绑定、认证引用消费、失效、只读审计、并行及迁移验收。

### Modified Capabilities

无；不修改已有 capability 或 `introduce-rust-codeguard-cli`。

## Impact

未来拟增加 `crates/codeguard-cli/src/guard_integration/`、独立输入 schema、fixture 和测试，具体新命令由本变更冻结；不改 native report schema、不搬迁领域算法。GuardEngine 依赖形式需 ADR，未安装/选定新 SDK。

[旧变更任务](../introduce-rust-codeguard-cli/tasks.md)继续拥有原生语言/CLI/repair/平台：2.3/2.4 聚合和冻结义务、2.5/2.9 报告/导出、3.5/3.7 输入绑定、4.5 批准政策、5.4/5.5 native freshness/conformance、6–8语言及9.10任务闭环。本变更仅要求所选范围具备相应已验证切片，不要求整个旧变更完成；缺口由其原 owner 解决，本适配器保持 unsupported/incomplete。

阶段依赖：CG-ADAPTER 等待 [GuardEngine change](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts) 的 GE-CONTRACT/GE-ADAPTER；本地 fixture调查可并行。受信消费等 GE-TRUST；独立生产发布等 GE-RELEASE。精确 Git 绑定可消费 [GitGuard change](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance) 的只读 GG-CANDIDATE，不依赖其写执行器或 FlowGuard 批准才开始本地投影。
