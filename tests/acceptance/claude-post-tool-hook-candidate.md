# Claude Code 保存事件 Rust 适配：局部验收

对应 OpenSpec `introduce-rust-codeguard-cli` S11.17 与 `hook-protocol` 的宿主事件要求。

2026-09-29：先增加 CLI 测试，入口未存在时 3 项失败；实现 `hook claude post-tool-use PATH --timeout DURATION --format=json` 后，候选测试 6 项通过，原 `hook_execute_cli` 17 项通过、2 项依赖真实工具而保持 ignored。本机现有 Ruff 0.16.8 的单独真实工具测试通过：`Write` 成功事件、`changed.py` 中 F401，宿主 `additionalContext` 包含规则 ID，同时不包含宿主传入的源码内容或原始工具消息。

完整工作区 `cargo test --workspace --all-features -- --test-threads=1 -q` 退出 0；补上 Unix 条件编译保护后，两个受影响的 Hook 测试组再次通过（6/17 项，另外 1/2 项为显式真实工具测试）。`cargo clippy --workspace --all-features --all-targets -- -D warnings` 退出 0。

此入口在同一 Rust 进程调用已存在的事件执行器。它从至多 1 MiB 的 Claude JSON 选择事件类型、工具名、cwd 和绝对文件路径；校验项目内普通文件，拒绝重复 JSON 键、超预算、缺失/越界/符号链接目标及缺少成功响应的输入。反馈只投影安全文件名、原生规则 ID、诊断数量和未完成状态，最多 1200 字符。宿主进程退出 0 表示软反馈返回，不能作为源码检查通过或交付许可。

后续增量把 `session-start`、`post-tool-use-failure` 和 `stop` 映射到同一 Rust 事件执行器。测试先因入口不支持而失败：启动事件不应执行 lint，失败写入不应运行源码检查或回显任意错误文字；Stop 无待办只提示完整检查，有稳定任务才给首次一次继续指引，`stop_hook_active=true` 不再唤醒，也不读取可编辑 Markdown 或模型结尾文本为指令。实现后候选组 10 项通过、1 项真实 Ruff 用例保留显式 ignored；原 `hook_execute_cli` 17 项通过、2 项 ignored。

扩展后完整工作区 `cargo test --workspace --all-features -- --test-threads=1 -q` 退出 0，显式设置本机 Ruff 路径后再次运行被默认忽略的真实保存事件 F401 测试，1 项通过。全目标 Clippy、三个改动 Rust 文件的 rustfmt 检查、OpenSpec 严格校验与 `git diff --check` 通过。整仓 `cargo fmt --all -- --check` 仍会报告其它未改文件的历史格式差异，本轮未扩大格式修改范围。

本次只通过模拟 Claude JSON 与真实 Ruff 进程验证 CLI 候选，**没有在已安装 Claude Code 中触发插件 Hook**。独立 `codeguard-plugin` 尚未绑定本次 Rust 二进制；提交/推送/CI、其它宿主与缓存身份去重均未接线。因此 S11.17 及宿主验收仍未完成。

2026-09-29 增量：为 `UserPromptSubmit` 增加固定的检查时机指引。测试先在 `hook execute` 0.4 报告上失败，随后 Rust 事件执行器新增 `read_only_intent_guidance`，Claude 适配器接受真实事件名和字符串提示，只输出固定、有界的 `additionalContext`，不回显或解析提示词。对抗测试分别发送提交祈使语与普通问题，两者获得完全相同的宿主反馈；显式传入会留下标记文件的假 Ruff 工具，确认事件未调用检查器。格式错误的提示字段保留“未运行”反馈。外层报告升至 0.5，0.4 schema 单独留存；本段没有证明默认插件 Hook 已切换，也不证明 Git/CI 门禁完成。

本增量的 `cargo test --workspace --all-features -- --test-threads=1 -q` 退出 0；最终源码的 `claude_hook_cli` 为 12 passed/1 ignored，`hook_execute_cli` 为 18 passed/2 ignored，显式设置本机 Ruff 0.16.8 后真实 Claude 保存事件用例 1 passed。`cargo clippy --workspace --all-features --all-targets -- -D warnings`、改动 Rust 文件的 rustfmt 检查、`openspec validate introduce-rust-codeguard-cli --strict`、两个 Hook schema 的 JSON 解析及 `git diff --check` 均退出 0。全套测试中的 ignored 用例没有计为通过。

2026-10-04 实际宿主证据追加：[Claude 缺运行时分支](claude-host-missing-runtime.md)首次证明真实已安装宿主自动触发 SessionStart/UserPromptSubmit/Stop，并把未完成原因送入实际模型对话；会话内加载插件源码，仍未验证实际 PostToolUse 保存事件或市场安装。上面的历史模拟结果保留，不能将本次有限范围升级为完整宿主验收。
