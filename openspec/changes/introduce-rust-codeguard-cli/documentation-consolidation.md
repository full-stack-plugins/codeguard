# 文档整合记录 — 2026-09-28

## 范围与事实源

已逐份读取旧 rust-cli 目录 11 份文档、现有中英文架构/技术方案/README 及相关 OpenSpec 证据。整合改变文档归属和过期说明，不改实现、规范要求、任务勾选或历史验收结论。HEAD `cd8fb72` 加工作树是此次观察范围，不能把未提交实现归入该提交。

## 文件归属

| 原文件（历史路径） | 当前入口 |
|---|---|
| `docs/rust-cli/architecture.zh-CN.md` | [统一归属](../../../docs/Codeguard-Architecture.zh_CN.md) |
| `docs/rust-cli/technical-design.zh-CN.md` | [统一归属](../../../docs/Codeguard-Technical-Design.zh_CN.md) |
| `docs/rust-cli/command-architecture.zh-CN.md` | [统一归属](../../../docs/Codeguard-Command-Reference.zh_CN.md) |
| `docs/rust-cli/project-initialization.zh-CN.md` | [统一归属](../../../docs/Codeguard-Project-Initialization.zh_CN.md) |
| `docs/rust-cli/remediation-workflow.zh-CN.md` | [统一归属](../../../docs/Codeguard-Remediation-Workflow.zh_CN.md) |
| `docs/rust-cli/false-positive-allowlist.zh-CN.md` | [统一归属](../../../docs/Codeguard-False-Positive-Governance.zh_CN.md) |
| `docs/rust-cli/static-check-catalog.zh-CN.md` | [统一归属](../../../docs/Codeguard-Adapter-Contracts.zh_CN.md) |
| `docs/rust-cli/coverage-and-acceptance.zh-CN.md` | [统一归属](../../../docs/Codeguard-Validation-and-Rollout.zh_CN.md) |
| `docs/rust-cli/legacy-v1-protocol-map.zh-CN.md` | [统一归属](../../../docs/Codeguard-Legacy-Compatibility.zh_CN.md) |
| `docs/rust-cli/legacy-delta-20260924.zh-CN.md` | [统一归属](../../../docs/Codeguard-Legacy-Compatibility.zh_CN.md) |
| `docs/rust-cli/README.md` | [统一归属](../../../docs/README.zh_CN.md) |

签名快照/修订链、工具包核验与发布从旧技术方案和白名单文档抽出为[信任与分发](../../../docs/Codeguard-Trust-and-Distribution.zh_CN.md)。每个专题都有英文配套说明。旧目录移除，不保留并行维护副本；原始文件摘要仍在 migration-manifest.json 中，历史清单不改写。

## 冲突的具体处理

| 冲突 | 统一结论 |
|---|---|
| 旧设计称 core 承担全部编排、clap/Tokio 是现有栈 | 区分逻辑接口与当前四 crate；应用编排在 CLI，参数解析/线程实现按当前源码说明 |
| 初稿报告 1.0/1.3 与当前 1.4 混用 | 当前通用 RunReport 1.4；check_feedback 0.30.0；check_aborted 0.11.0；不把简报节选冒充合法完整报告 |
| 旧状态称无原生适配器或 Rust build 尚未接线 | 当前局部原生路径与完整槽位 gap 分开描述；历史审计不覆盖当前状态 |
| codeguard/ 与 .codeguard/ 混淆 | 项目数据固定 .codeguard/；普通 codeguard/ 源码不能整体跳过；精确受管记录仍接受入库安全检查 |
| 检查一律不自动 sync 与当前 CLI 持久化冲突 | 已接入 CLI 路径保存并同步；显式 work sync 可恢复；完整宿主自动闭环仍待验收 |
| 独立白名单候选/摘要被混同可信生效 | 分开未核验候选、局部签名/链绑定、宿主可信批准和最终门禁；原生发现始终保留 |
| 源码构建与已发布 npm 状态冲突 | 记录 0.1.0 Apple Silicon macOS 范围；不推断源码标签绑定、多平台或宿主验收 |
| S11/S14 的“三个宿主”不同 | S11 Codex/ZCode/Kimi；S14 Claude Code/Codex/Gemini CLI；保留已有任务边界 |
| 离线参数被误认为网络沙箱 | 现有原生离线选项与目标可信隔离分别陈述 |
| WASM 清洁初检被当作 lint/交付通过 | 原生优先、检查范围分离、疑似错误要求原生确认、必需原生义务保留 |

## 验证范围

执行本地链接/锚点、代码块/JSON 样例、文档命名、OpenSpec strict 及差异空白检查。文档修改不构成新增运行时验收；已有测试记录保持其原日期、范围和忽略项。本次不执行规格 sync/archive，不发布或提交代码。

### 完成校验 — 2026-09-29

- 46 份 Markdown 中的 607 个本地链接及锚点检查通过；5 个 JSON 代码块解析通过。
- 48 个 Mermaid 代码块及全部围栏结构检查通过；未执行 Mermaid 图形渲染验收。
- C01–C36 共 36 个独立命令契约、F01–F26 共 26 个故障验收条目保留。
- 中英文 Architecture 文件命名校验：2 份、0 错误。
- `openspec validate --all --strict`：1 项 change 通过、0 失败。
- `git diff --check` 及纳入扫描文档的行尾空白检查通过。
- 当前源码再次确认 RunReport 1.4、check_feedback 0.30.0、check_aborted 0.11.0。没有运行新增原生工具验收，也没有将文档校验计作功能完成。
