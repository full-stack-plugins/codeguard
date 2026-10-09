# 六仓现状与事实源快照

采集：2026-10-09。本地只读源代码与规格检查；没有 fetch/pull、建立或更新索引、运行产品回归或访问真实宿主/服务端保护。本表区分“文件/源码存在”和“完整验收”。

## 1. Git 基线

| 仓库 | 本地 HEAD | 分支与原始状态 |
|---|---|---|
| codegraph-plugin | cf4f82a300540671f3d0e32e6dcdda40f69237f1 | main，开始时干净 |
| codeguard-plugin | 7f67e346c3c95ca83b72e791d6871f76bf0beed9 | main，开始时干净 |
| codeguard | 958a106636ad8430e40e45075e49c6eea1496629 | feat/rust-hook-prompt-guidance；已有 .gitignore 修改与未跟踪 .php-cs-fixer.dist.php，保持原样 |
| codereview-plugin | 81d7ff6e0100d6717d9065f1b7622c9c9defe568 | main，开始时干净，落后本地已知 origin/main 1；未拉取 |
| flowguard-plugin | b810fe1df8055e915efedcab78acc7efa19f7505 | main，开始时干净，落后本地已知 origin/main 1；未拉取 |
| gitflow-plugin | 28aa6690869a625a2a24b4556c5936762a6810bc | main，开始时干净 |

远端跟踪引用可能过期，因此不称“已核实远端最新”。实施前只刷新相关文件/引用；不覆盖本轮或其他人新增修改。

## 2. 已有能力与不能推导的结论

| 仓库 | 当前可核实事实 | 本次规划补齐 | 不能直接声称 |
|---|---|---|---|
| codegraph-plugin | AGENTS/README 限定纯上游客户端、原文提示和 SessionStart；`scripts/codegraph_lib` 实现文档维护 | provider 能力/覆盖/版本契约与事实适配协作 | 有查询输出就有完整代码关系或架构通过 |
| codeguard-plugin | Rust 生命周期 change、`hooks/rust_runtime_dispatch.cjs` 与固定运行时；旧 Python Git/MCP 并存 | 独立工程协议、模式、去重与宿主桥接 | Rust 全面替换、真实宿主/发行全部完成 |
| codeguard | Cargo workspace 四 crate；runtime 已使用 Tokio，Serde/JSON；core `delivery_gate.rs` 有 `evaluate_delivery` 与完整性/目标集合检查 | 统一契约、OPA、证据、授权执行、架构规则及跨插件协议 | 所有输入已由可信主体核验、所有语言/平台资格已完成 |
| codereview-plugin | `runtime.py` / `consent.py` / `git_snapshot.py` / `protocol.py`；`docs/protocol.md` 明确 advisory | 精确候选类型、设计契约上下文、统一审查证据 | success=代码通过，source=user=不可伪造批准 |
| flowguard-plugin | `governance.py` / `stage_docs.py` / `evidence.py`；已有 `codereview_evidence.py` 消费者 | receipt、统一动作接线、协调与独立强制等级 | 没有集成消费者，或已存在强制隔离 |
| gitflow-plugin | README 0.2.0；`scripts/gitflow/ci.py` 从 base 读取规则，`rules.py` 元数据规则，service 标 quality not_evaluated | 任务范围、实际候选、跨守卫证据和执行授权 | Git allow=完整工程通过、配置文件存在=远端保护已启用 |

既有 CodeGuard `introduce-rust-codeguard-cli` 本轮 OpenSpec list 计数为 322/354；这是任务勾选快照，不是本轮运行认证。当前 docs 已有较新的 v1 candidate 描述，不能继续沿用旧记忆中的完成数量或 grammar 资格数字。生产资格以当前源绑定原始报告、未完成任务与实际门禁分别核验。

## 3. 每仓规格事实源

| 仓库 | 既有体系 | 本轮规划归属 |
|---|---|---|
| codegraph-plugin | OpenSpec；历史 config schema 为 openspec/v1 | 新 change `integrate-engineering-guard-facts` 显式 spec-driven；不改旧配置 |
| codeguard-plugin | OpenSpec；三条未归档 change 保留 | 新 change `integrate-engineering-guard-host` |
| codeguard | OpenSpec；introduce-rust-codeguard-cli 保留 | 新 change `add-engineering-guard-core`，只拥有新增跨插件能力 |
| codereview-plugin | OpenSpec，现行四项 specs | 新 change `integrate-engineering-guard-review` |
| flowguard-plugin | Superpowers 十阶段权威规格/计划 | 原规格第 7 节、原计划第 5 节追加 |
| gitflow-plugin | OpenSpec 增量 + 旧 Superpowers 兼容基线 | 新 change `integrate-engineering-guard-git` |

未初始化任何新体系，未 sync/archive。CodeGraph 初始 change 的旧格式不在本轮修复范围；新增 change 使用当前 CLI 可验证格式，不强制刷新已有文件。

## 4. 工具检查

OpenSpec CLI 1.13.1 可用；相关 propose 技能已读取。OPA 不在 PATH，本轮不安装。CodeGraph CLI 可用；六目标中 codereview-plugin 有现成索引，status 显示 31 files/404 nodes/1107 edges、3 added/1 modified，构建引擎版本旧；用于定位后回读关键当前源码，不 sync。其余五仓没有 `.codegraph/`，直接按规范读取相关源码，未创建索引。

工具可用、项目集成和产品验收是不同状态。其余计划依赖的版本、许可证、目标平台兼容性在各任务实施前锁定，不能把选型表视为已安装清单。
