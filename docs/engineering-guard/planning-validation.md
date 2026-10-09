# 规划验证记录

日期：2026-10-09（Asia/Shanghai）。本记录只证明规划产物的结构、追溯和工作区保护检查，不证明工程守卫已经实现或通过运行验收。

## 1. 规格结构

使用本机 OpenSpec CLI 1.13.1，在各自仓库运行下列命令，五项均通过严格校验：

| 仓库 | 实际校验命令 | 结果 |
|---|---|---|
| codeguard | `openspec validate add-engineering-guard-core --strict` | 通过 |
| codegraph-plugin | `openspec validate integrate-engineering-guard-facts --strict` | 通过 |
| codeguard-plugin | `openspec validate integrate-engineering-guard-host --strict` | 通过 |
| codereview-plugin | `openspec validate integrate-engineering-guard-review --strict` | 通过 |
| gitflow-plugin | `openspec validate integrate-engineering-guard-git --strict` | 通过 |

CodeGraph 原有根配置的旧 schema 不受当前 CLI 支持；仅为本次新 change 指定 `spec-driven`，没有替换根配置。FlowGuard 继续使用既有 Superpowers 规格/计划，未运行 OpenSpec 初始化；其新增条目纳入下述跨仓检查。

## 2. 跨仓追溯与依赖

对新规格和原生任务账本进行程序化检查，结果如下：

- 50 条唯一需求：42 条 OpenSpec Requirement，8 条 FlowGuard 增量需求；每条均有实施任务关联。
- 78 项新增任务：CodeGraph 8、CodeGuard 宿主 10、CodeGuard 内核 28、CodeReview 10、FlowGuard 12、GitFlow 10；全部保持未勾选。
- 152 条任务依赖引用均能解析，拓扑排序通过，没有依赖环。
- 36 个跨仓验收场景见验收矩阵；场景为待实施的验收设计，没有标记为已执行或通过。
- 新增 Markdown 的本地文件链接及代码围栏闭合检查通过；另检查 FlowGuard 三份既有文档新增章节中的本地链接。

追溯检查验证 ID、引用和任务覆盖，不自动证明需求语义完整、估算准确或设计合理。任务依赖的权威仍是各仓原生账本；跨仓表不另设完成状态。

## 3. 变更与保护

本轮只增加规划 Markdown、新 change 的 `.openspec.yaml`，并追加 FlowGuard 既有规格、计划和路线图。六仓 `HEAD` 未改变；未创建或切换分支，未提交、推送、安装依赖或发布。

CodeGuard 开始时已有 `.gitignore` 修改及未跟踪的 `.php-cs-fixer.dist.php`，文件 SHA-256 与开始快照一致。既有语言迁移任务、历史验收状态及其他项目实现没有被新计划覆盖。FlowGuard 既有三份文档保留原文，只在末尾追加本轮内容。

六仓 `git diff --check` 通过；新增文档另检查末尾换行和行尾空白，避免只检查已跟踪文件而遗漏新文件。当前本地分支及远端认知边界记录于仓库基线；本轮没有 fetch/pull，不声称这是远端最新状态。

## 4. 尚未执行的验证

未运行产品构建、原生语言测试、OPA 策略执行、真实宿主 Hook/MCP、受保护分支/CI、合入执行器或生产验证。拟议 CLI、协议 schema、fixture、crate、授权和证据链尚需按任务实现；本机 OPA 未发现可执行文件，本轮未安装。

实施后须逐项取得当前源码、候选、契约和工具版本绑定的真实证据。规划校验通过不能替代验收矩阵中的任何运行门禁。
