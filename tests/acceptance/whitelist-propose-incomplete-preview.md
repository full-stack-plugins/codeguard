# 误报白名单 propose 缺证据预览（2026-09-25）

`codeguard rules whitelist propose <finding-id> [path] --format json` 已能读取本地稳定任务。预览协议现为 `whitelist_proposal_preview:0.2.0`：从最新的本地 Ruff 报告读取已同步、源码与配置仍匹配的该 finding，展示报告/工作区/发现/源码及本轮工具、配置摘要；这些仅是 `local_unverified` 观察，不是批准。最新报告无目标、未同步、损坏或源码/配置已变化时，不回退到旧报告。结果列出原工具复检命令与仍缺的适配器、rulepack、人工裁定和可信策略来源；固定 `candidate=null`、`authority=unverified`、`gate_effect=none`、`not_evaluated`、退出 3。它不向 `codeguard/decisions/` 写文件。环境 blocker、未知任务也不能生成候选。此命令尚不能生成可评审的完整白名单条目，更不能批准或放行。

目标测试先因命令不存在失败；实现后 `whitelist_propose_contract` 的 2 项普通测试与固定 Ruff 0.16.8 的 1 项显式真实测试通过，后者覆盖当前观察、未同步较新报告、源码/配置变化和新扫描问题消失。协议见 `schemas/whitelist-proposal-preview.schema.json`。接下来必须锁定适配器与受批准 rulepack 身份，并完成可信策略/批准来源、人工误报裁定、候选生成和全格式门禁；OpenSpec 4.9 保持未完成。

后续局部升级见 [Ruff 候选规则映射](ruff-rulepack-observation.md)：预览协议 0.3 额外展示未批准规则包摘要，并明确将 `approved_rulepack_identity` 列为缺失证据；上述 0.2 记录保留为历史验收。

候选查询、观察身份和纠错候选读取现复用运行时的有界普通文件读取器；Unix 最终路径以 `O_NOFOLLOW` 打开，按已打开的文件句柄核对类型和大小。`whitelist_command_contract` 覆盖候选链接和观察链接均返回未完成且门禁效果为 `none`。这一改动仅收紧本地输入读取，不提供可信批准来源。

2026-09-25 复验：`cargo test --workspace --offline -q`、`cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo fmt --all --check` 通过；`CODEGUARD_RUFF_BIN=/Library/Frameworks/Python.framework/Versions/3.13/bin/ruff cargo test --offline -q -p codeguard-cli --test whitelist_propose_contract -- --ignored` 使用 Ruff 0.16.8 运行 2 项真实复检/纠错用例并通过。真实工具结果仍只证明本地证据与未授权预览行为，不是受保护审批或完整交付门禁验收。

2026-09-28 增量：`rules whitelist propose` 接受可选的绝对 `--ruff-tool`。指定时按有界普通文件读取当前字节并对比本地扫描报告中的 `tool_sha256`；不同或不可读取时返回 `native_tool_changed_since_scan` / `native_tool_unavailable`，不显示旧观察、不生成候选、退出 3。相同工具仍只给 `evidence_incomplete`，不执行 Ruff、不签发批准。相对或重复参数在读取任务前退出 2。目标测试先因新参数不支持而失败；实现后普通契约、2 项显式 Ruff 0.16.8 用例、全工作区 `cargo test --workspace --offline -q`、Clippy 全目标 `-D warnings` 与 fmt check 均通过。没有指定工具时，旧摘要仍只是未获信任的本地观察；完整工具来源、规则包批准和门禁仍待完成。
