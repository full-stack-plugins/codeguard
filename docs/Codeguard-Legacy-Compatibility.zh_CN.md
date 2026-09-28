# Codeguard 旧插件兼容与迁移边界

> **文档说明：**专门保留旧 CLI/MCP/Hook 协议及有日期的审计；不描述当前 Rust 可用命令。
>
> **文档版本：**1.2.0 · **最后更新：**2026-09-29 · **状态：**当前切片与目标契约分别标注。

[English](Codeguard-Legacy-Compatibility.md) · [文档导航](README.zh_CN.md) · [架构](Codeguard-Architecture.zh_CN.md) · [技术方案](Codeguard-Technical-Design.zh_CN.md)

本表只描述插件旧入口的既有行为，供 Rust C35 兼容层逐项实现。旧版返回 0、MCP 工具调用成功或 Hook 放行，均**不等于**新版交付 `allow`；Rust 投影固定 `new_delivery_decision=not_evaluated`。新版 CLI 的 0/1/2/3/4/130 协议独立计算，不能直接透传旧数字。来源为 2026-09-24 核对的插件旧入口和应用服务；[旧源码增量](Codeguard-Legacy-Compatibility.zh_CN.md#legacy-audit-20260924)记载与早期设计快照的差异。

## CLI 入口

| 旧入口 | 旧数字和混合优先级 | 0 的实际含义及迁移边界 | 来源 | 验收入口 |
|---|---|---|---|---|
| `bin/codeguard` 默认、`check` | `FAIL→2` 优先于 `UNVERIFIED/PLANNED→1`；仅 PASS/SKIPPED→0；空语言→1；argparse 用法错→2 | 可能只有 SKIPPED，不代表完整项目质量通过；MCP 模式另列 | `bin/codeguard`、`scripts/run_check.py::cli_main`、`scripts/codeguard/verdict.py::exit_status` | Rust `legacy_v1_protocol_contract`；旧 `tests/test_verdict_integrity.py` |
| `fix` | 任一 formatter 失败、未配置而返回 PLANNED、UNVERIFIED 或无识别语言→1；全部成功/跳过/显式 dry-run、无 Git 改动或无适用文件→0；argparse 错→2 | 空 `run_fix` 结果不足以区分“无语言”和“无适用文件”，兼容层必须保留入口分支；formatter 成功不证明文件改变，仍须复检 | `scripts/fix.py::main`、`scripts/codeguard/language_check.py::run_fix` | Rust 参数表；旧 `tests/test_cli_contracts.py`、`tests/test_cli_fix_reporting.py` |
| `cve` | `FAIL→2` 优先于 `UNVERIFIED→1`；空结果→1；全 PASS→0；参数/生态/严重度错→3 | 只是旧阈值下各生态结果；不能用 2 推断新版 CLI 用法错 | `scripts/cve_check.py::main`、`scripts/codeguard/cve_policy.py::aggregate_exit` | Rust 参数表；旧 `tests/test_cve_boundaries.py` |
| `dockerfile` | 任一 UNVERIFIED 或无目标→1，即使同批有 FAIL；完整且有 FAIL→2；全 PASS→0；argparse 错→2 | 混合结果中的风险仍留在报告；退出 1 不得丢掉已确认发现 | `scripts/dockerfile_security.py::main`、`scripts/codeguard/dockerfile_reports.py::aggregate_status` | Rust 参数表；旧 `tests/test_dockerfile_boundaries.py` |
| `detect` | 有根及无根的正常查询→0（无根输出 `[]`）；配置错误→1 | 只是发现操作，`[]` 不能签发空项目质量通过 | `scripts/detect_lang.py::main` | Rust 参数表；旧 `tests/test_cli_contracts.py` |
| `init` | 总是输出人工引导并返回 0，不写项目 | 旧引导完成不等于 Rust `init --apply` 或任何检查完成 | `bin/codeguard` 的 `init` 分支 | Rust 参数表；旧 `tests/test_cli_contracts.py` |
| `java-plan` | `UNVERIFIED→1`，其余计划状态→0；argparse 错→2 | 只读计划，未执行 Java 构建或检查 | `scripts/java_project.py::main` | Rust 参数表；旧 `tests/test_java_project_impact.py` |
| 未知子命令 | wrapper→1；诊断写 stderr | 新版未知命令走自身用法协议，不能套旧 1 | `bin/codeguard` 默认分支 | Rust 参数表 |

`run_check.py --mcp` 是旧 MCP **服务器生命周期**入口：SDK 缺失时进程返回 1，启动服务后退出 0。服务退出 0 不表示任何工具调用或质量检查成功。`check_code_style` 返回逐语言数组，保留 `passed/status/reason/exit_code/log_path`；`auto_fix` 返回修复尝试与复检摘要；`list_languages` 只返回 ID/名称；`analyze_java_impact` 返回只读计划。四种工具返回的是 MCP 文本块中的 JSON payload，**没有**可直接映射的每次调用进程退出码；配置损坏可返回 `UNVERIFIED/exit_code=1` 的工具 payload，未知工具返回 `error` 对象。来源：`scripts/run_check.py::mcp_main`、`scripts/codeguard/check_application.py::mcp_tool_payload`；旧 `tests/test_mcp_server.py` 和 `tests/test_mcp_autofix_identity.py` 继续覆盖真实旧 SDK 接口。C35 的 MCP 接线与宿主验收另由 11.1/11.3 承担，本表不把服务存活变成新版认证。

## 五类 Hook

| 旧事件 | 旧退出/反馈 | 新版边界 | 来源与旧夹具 |
|---|---|---|---|
| SessionStart | 0；stdout 人类可读摘要；未捕获异常 stderr 提示并 exit 0 | 环境信息不等于检查通过 | `hooks/env_check.py`；`tests/test_startup_application.py` |
| UserPromptSubmit | 0；命中时 stdout `hookSpecificOutput.additionalContext`；内部异常 exit 0 | 观察型软反馈不阻断交付 | `hooks/user_prompt_validator.py`；`tests/test_prompt_application.py` |
| PreToolUse Bash | 0 放行或附 `additionalContext` 的未验证；2 拦截并在 stderr 给修复指令；内部异常 exit 0 | 旧 2 是宿主拦截，不是新 CLI 用法错；旧 0 包含 fail-open，不能当 `allow` | `hooks/pre_tool_git_guard.py`、`scripts/codeguard/git_guard_application.py`；`tests/test_git_guard_application.py` |
| PostToolUse Write/Edit/MultiEdit | 0；stdout 可含 JSON 反馈与 systemMessage；内部异常 exit 0 | 保存反馈不承担质量门禁 | `hooks/post_tool_lint.py`；`tests/test_save_application.py` |
| Stop | 0；stdout 会话摘要；内部异常 exit 0 | 历史计数不替代完整交付检查 | `hooks/stop_summary.py`；`tests/test_session_application.py` |

PreToolUse 当前源码还会在无法准确解析 Git 意图、没有可定位仓库等情况下返回 2；这与旧 `hooks/__protocol__.md` 中“仅已确认 lint 失败才 exit 2”的文字不完全一致。C35 必须先按真实旧源码和宿主实测固定此分支，不得以旧文档的概括删掉阻断或把不可确认意图伪称为已确认源码违规。`codeguard.skipGate`/`CODEGUARD_SKIP_GATE` 和 Hook fail-open 仅归 legacy-v1；新版交付门禁按独立完整性与受保护策略判定。

Rust `crates/codeguard-cli/src/legacy_v1_protocol.rs` 只承接**已经归类**的旧信号，拒绝不支持的入口/信号组合。它没有调用 Python、没有运行原生检查器，也不签发质量结果。C35 仍须实现旧 argv、字段、MCP/Hook 宿主接线及每种入口的真实回放；本表与纯映射测试不能替代这些运行时验收。

<a id="legacy-audit-20260924"></a>

## 历史附录：2026-09-24 旧源码差异

这份记录为 OpenSpec 1.1 的**进行中证据**。当时插件 HEAD 为 `9e4adb1`；原设计读取的 `03ebb24` 不是最新源码。两提交间旧运行时代码有 9 个文件变化。`scripts/languages.json` 未变，SHA-256 为 `012370f9d83a2daeb3cc87fef292a7509e85b53540ce9adf1eaf171f0910d251`，仍是 54 stable / 3 planned。逐语言差异与后续验收入口见 [语言迁移表](Codeguard-Validation-and-Rollout.zh_CN.md#2-全量迁移清单)；Rust 工程固定了该清单快照，并在 `codeguard-cli/crates/codeguard-cli/tests/capability_inventory.rs` 核验 当时的能力槽 个显式缺口，不能把旧注册状态当作新能力。

| 旧源码增量 | 新流程归类与依据 | 已有回归样本 / 后续 fixture |
|---|---|---|
| `hooks/__protocol__.md`：旧 skipGate 只豁免语言检查，入库安全仍检查 | **保留 legacy-v1**；新交付完全拒绝 skipGate 自授权。见 hook-protocol、rulepack-governance | `tests/test_skip_gate_safety_scope.py`；新入口 F13/F22 待实现 |
| `scripts/codeguard/gate.py`：采集缓存身份前补 PATH | **保留正确行为**，但新缓存必须绑定真实有效环境身份。见 execution-kernel | `tests/test_skip_gate_safety_scope.py`；新缓存 F09/F10 待实现 |
| `scripts/codeguard/git_guard_application.py`：语言豁免后仍执行安全路径检查 | **legacy-v1 兼容 + 新流程纠偏**：保留安全发现，新入口不得接受豁免。见 hook-protocol、scan-scope-policy | `tests/test_skip_gate_safety_scope.py`；新 Git F16/F18 待实现 |
| `scripts/codeguard/prompt_application.py`：软反馈同样保留安全报告 | **legacy-v1 兼容**；新软反馈不能冒充硬门禁。见 hook-protocol | `tests/test_skip_gate_safety_scope.py`；新宿主 F22 待实现 |
| `scripts/codeguard/toolchain.py`：补 PATH，缺入口时探测 Python 模块并给诊断 | **保留正确诊断**，但不得把模块探测成功当作原命令可执行。见 unified-cli-contract、native-tool-adapters | `tests/test_skip_gate_safety_scope.py`；新 doctor F03 待实现 |
| `scripts/paths.py`：补符号链接解释器真实 scripts 目录 | **保留正确路径发现**，工具仍需锁身份与实际启动证明。见 execution-kernel、rulepack-governance | `tests/test_skip_gate_safety_scope.py`；新 tools verify/doctor 待实现 |
| `scripts/codeguard/reporting.py`：说明旧豁免范围 | **legacy-v1 兼容**；新修复简报不得推荐绕过。见 hook-protocol、remediation-workflow | `tests/test_skip_gate_safety_scope.py`；新 RepairBrief 待实现 |
| `scripts/check_architecture.py`：容纳旧模块新增依赖 | **只保留旧架构校验**；新四 crate 方向由独立检查拒绝反例。见 execution-kernel | `tests/test_architecture.py`；`codeguard-cli/crates/codeguard-cli/tests/crate_boundaries.rs` |
| `scripts/bump-plugin.mjs`：Claude 版本路径写后核对 | **保留发布正确性**；新二进制与插件锁/市场仍需分层验收。见 binary-distribution | `tests/test_plugin_manifests.py`；新发布验收 13.4 待实现 |

旧源码中仍存在新规范明确纠偏的共性行为：通用 `rc=1` 推断 finding、最多 50 文件时的 delta/基线豁免、缺工具旧 Hook 放行、Java Maven verify 代替多项质量义务。它们分别由 native-tool-adapters、verdict-integrity、execution-kernel、hook-protocol 和 language-gate-commands 的新要求替换；当时仅有领域聚合测试，真实新适配器和完整对照 fixture 尚未落地，因此 OpenSpec 1.1 **仍未完成**。
