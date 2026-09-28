# `plan` 只读预览验收

`codeguard plan <lint|comments|dependencies|cve|security|build|check> <language|all> [path] --format json` 已接入 Rust CLI。它先校验命令类别与 57 语言注册表中的 canonical ID，再静态观察项目；错误类别或未知语言在读取项目之前返回 2。

公开 `schemas/plan-preview.schema.json` 固定 `report_type=plan_preview`、`planning_status=incomplete`、`quality_decision=not_evaluated`、`policy_identity=null`、`obligations=[]`。输出列出选定类别、观察到的语言、检查器配置和仅供后续求解的候选任务。候选任务的命令为 null，不代表已选定原生工具或具备可执行适配器。缺可信策略、工具锁及内容身份时退出 3，不生成需要受信任政策身份的正式 `CheckPlan` 1.1，也不产生质量 allow。

`plan_preview_cli` 的首两项集成测试先因命令不存在失败，接线后通过：固定 Ruff 项目可观察到已配置检查器；PATH 中伪造的 `ruff` 可执行文件未运行，项目根目录无新增文件；错误类别和未知语言均为用法错误。随后补充第三项：固定 P3C POM 在 `plan lint java` 中只成为静态候选，不生成命令或质量通过。`cargo test --workspace --offline -q`、`cargo clippy --workspace --all-targets --offline -- -D warnings` 及 `cargo fmt --all` 通过。

尚缺可信策略/工具锁来源、全量义务和 DAG 生成、预估资源与所有 C06 验收。OpenSpec 2.1、2.4、2.7 保持未完成。
