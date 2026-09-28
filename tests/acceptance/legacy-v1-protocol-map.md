# legacy-v1 具名入口映射验收（2026-09-28）

规格：插件 `openspec/changes/introduce-rust-codeguard-cli` 的 2.6、`verdict-integrity`、`execution-kernel` 和 `hook-protocol`。权威映射表在插件 `docs/rust-cli/legacy-v1-protocol-map.zh-CN.md`；Rust 产品中的 `legacy_v1_protocol` 仅接收已归类旧信号并返回旧数字，所有投影的新版交付决策固定为 `not_evaluated`。

参数测试覆盖旧 check/CVE/Dockerfile 的不同混合优先级、fix 的 formatter 成功/失败/未配置/显式 dry-run/无语言/无改动/无适用文件、detect/init/java-plan/未知命令、PreToolUse 与观察 Hook，以及各旧 argparse 入口的数字。未知入口信号组合拒绝猜测。关键反例：同一 `FAIL+UNVERIFIED` 在 check/CVE 返回旧 2，在 Dockerfile 返回旧 1；fix 的 `PLANNED` 若并非显式 dry-run，会被旧 CLI 记为失败 1；fix 空结果无从区分入口前的“无语言”旧 1 与“无适用文件”旧 0，Rust 映射拒绝猜测；旧 CVE 参数错误为 3，旧 PreToolUse 拦截为 2，二者均不能透传成新版 CLI 语义。`java-plan` 的正常状态是 `PLANNED/SKIPPED`，0 仅代表计划入口完成。

验证：`cargo test --offline -p codeguard-cli --test legacy_v1_protocol_contract` 3/3 通过；全 Rust 工作区 158 组、941 项通过、96 项按条件忽略，all-targets Clippy `-D warnings` 通过。旧插件的相关 12 组 `unittest` 共 164 项通过，另 `tests.test_mcp_server` 7/7 通过。运行旧插件测试时将 PATH 前置已有的 `/opt/anaconda3/bin`，以确保 `bin/codeguard` 所调用的 `python3` 是 Python 3.13.5。未前置时系统 Python 3.9 在旧 `scripts/codeguard/execution.py` 的 `zip(..., strict=True)` 抛错，导致一项 `fix` 用例失败；这是旧运行时兼容性缺口，不是 Rust 投影的通过证据。

此项只完成映射表与参数契约。C35 真正的 `compat legacy-v1` 命令、Python 旧实现调用/迁移策略、四个 MCP 工具和五类 Hook 的 Rust 宿主接线，以及新版交付门禁仍由后续任务验收。当前投影不能签发 allow。
