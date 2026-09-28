# Codeguard 文档导航

[English](README.md) · [项目 README](../README.zh-CN.md)

本目录是 Rust Codeguard 的统一设计文档入口。原 `rust-cli/` 的 11 份文件已整合到主文档与独立专题；不再维护第二套架构和技术方案。本文及各专题以 2026-09-28 的 HEAD 加工作树为观察范围，目标设计不能当作已发行功能。

## 阅读顺序与唯一职责

1. [README](../README.zh-CN.md)：安装、可用能力、快速开始和故障排查。
2. [架构](Codeguard-Architecture.zh_CN.md)：系统边界、组件、领域语义、执行路径和关键决策。
3. [技术方案](Codeguard-Technical-Design.zh_CN.md)：公共技术契约、检测族目录、统一报告示例及实施路线。
4. 以下专题：保留独立业务与工程细节；不另维护任务勾选状态。

| 专题 | 独立职责 |
|---|---|
| [逐命令契约](Codeguard-Command-Reference.zh_CN.md) | C01–C36、参数、预算、退出语义与副作用 |
| [项目初始化](Codeguard-Project-Initialization.zh_CN.md) | 静态画像、模块关系、受管文件与 AGENTS 刷新 |
| [持久修复工作流](Codeguard-Remediation-Workflow.zh_CN.md) | 稳定问题、任务简报、租约、尝试、复检与复发 |
| [误报治理](Codeguard-False-Positive-Governance.zh_CN.md) | 纠错分流、精确白名单、生命周期与反馈 |
| [原生适配器契约](Codeguard-Adapter-Contracts.zh_CN.md) | 原生配置与报告、Java/CVE/安全边界 |
| [信任与分发](Codeguard-Trust-and-Distribution.zh_CN.md) | 签名、修订链、工具包、npm 与 grammar 资产 |
| [验收与发布](Codeguard-Validation-and-Rollout.zh_CN.md) | 语言矩阵、F01–F26、精度目标、宿主与发布证据 |
| [旧协议兼容](Codeguard-Legacy-Compatibility.zh_CN.md) | 旧 CLI/MCP/Hook 映射与 2026-09-24 审计 |

## 当前、目标和历史

- 当前局部能力以源码、测试和带范围的验收记录为依据；公开命令以实际 dispatcher 为准。
- WASM native-first 兜底、完整可信门禁、正式修复关闭和宿主自动闭环仍含未完成项。
- [CAPABILITIES](CAPABILITIES.md) 是生成的完整能力槽矩阵；槽位 gap 不否定某条局部原生路径已经实现。
- C01–C36 是命令设计编号，F01–F26 是故障验收编号，均不是功能完成标记。
- 中英文专题共享边界与结论。中文保留逐项详细表、参数和历史证据；英文提供对应工程契约并明确链接到同一明细。

## 规格与证据

[OpenSpec proposal](../openspec/changes/introduce-rust-codeguard-cli/proposal.md)、[specs](../openspec/changes/introduce-rust-codeguard-cli/specs)、[唯一任务清单](../openspec/changes/introduce-rust-codeguard-cli/tasks.md)、[实现覆盖](../openspec/changes/introduce-rust-codeguard-cli/implementation-coverage.md)、[验收记录](../tests/acceptance)。文档整合不更改任务完成状态，也不执行规格归档。

[整合对照与冲突处理](../openspec/changes/introduce-rust-codeguard-cli/documentation-consolidation.md)记录旧文件去向及已消除的冲突；迁移清单保留原始路径和摘要作为历史来源。
