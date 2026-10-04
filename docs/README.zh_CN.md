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
| [grammar 开发评测](Codeguard-Grammar-Evaluation.zh_CN.md) | 固定 358 例、32 语言、35 来源组回放，逐组差异与未知 |
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


### 当前公开候选：0.1.4

`@partme.ai/codeguard@0.1.4` 已从干净源码 `1cd458f6e01a44a74388243e964e3f45290ac18e` 发布，限 Apple Silicon macOS。包含全部 32 份可执行但未验收的 grammar、指定编辑文件检查、稳定原生确认任务、原生复检与 next 指引。注册表摘要、新缓存 npx、公开包真实 Zig 0.16.0 修复链路及相同源码 Linux CI 已通过。普通 CLI 的零诊断不能在缺可信策略时关闭任务；限定 Zig SDK 是源码集成 API，npm 不暴露自批命令。插件启用、真实宿主、完整精度、多平台与完整门禁仍未完成。此前 0.1.3 证据保留为历史快照。见 [0.1.4 验收](../tests/acceptance/npm-0.1.4-candidate.md)。

原生发现直接生成任务、当前证据和复检协议见[Codeguard-Native-Repair-Workflow](Codeguard-Native-Repair-Workflow.zh_CN.md)。

### Kotlin 首次原生观察（开发源码）

`check all . --kotlinc-tool ABS_PATH` 和确认保存的 `file_changed` 接受相同显式工具；省略时发现调用环境 PATH。已选工具失败保留原生阻塞，不改走 WASM；仅缺工具时使用候选初检。最多 64 份普通 `.kt` 共用请求截止时间，不在此路径编译 `.kts`。重复扫描更新同一稳定任务，原生零诊断不自动关闭任务。

新增聚合0.42.0、保存 Hook0.11.0、扫描0.1/0.2、原生首次事实0.4.0、原生来源复检0.6.0/任务反馈0.17.0，以及首次原生简报0.10.0；已有 WASM 来源任务继续沿用旧协议。见[首次原生验收](../tests/acceptance/kotlin-native-first.md)。完整项目 lint、编译器/JDK 身份、可信关闭和公开发行仍待完成。

### Swift 原生优先单文件反馈（开发源码）

`codeguard lint swift FILE.swift [--swift-tool ABS_PATH] [--timeout 30s] --format=json` 优先选择显式编译器，否则调用环境绝对 PATH 中首个普通可执行 `swiftc`。当前适配 Apple Swift 6.4，对冻结 stdin 执行 frontend parse；仅缺工具时使用内置 WASM 候选，选定工具失败保留原生阻塞。位置为 UTF-8 字节列；源码或入口目标变化撤回旧诊断，超时/取消不按源码违规解释。

`swift-lint-feedback`0.1.0 区分未完成/隐藏恢复所需的原生确认，与完整零恢复后的推荐准备。它不授予完整 lint 或交付通过：SwiftLint、注释、类型、依赖/安全和项目构建仍有义务。工具身份只覆盖所选入口，不代表整个工具链。Swift 项目原生观察已按下文接线；原生任务连接、完整语言及发行验收仍未完成；公开 npm0.1.4不包含本增量。见[单文件验收](../tests/acceptance/swift-native-single-file.md)。

### Swift 项目原生观察（开发源码）

`check all . [--swift-tool ABS_PATH] --format=json` 对最多 64 份普通 Swift 文件在同一截止时间内执行冻结原生 parse，报告未观察文件；原生选择失败不回退，只有缺工具保留 WASM。`check-feedback`0.43.0 / `swift-parse-scan`0.1.0 保留当前字节位置、工具选择及复检 argv。源码或工具变化撤回位置，零诊断不等于完整 SwiftLint、类型或构建通过。项目原生结果的任务同步明确为 `not_connected`，未伪造任务引用；保存 Hook 的首次原生扫描与该连接仍待完成。见[项目观察验收](../tests/acceptance/swift-native-project.md)。
