# 公共CLI语言别名局部验收

对应 introduce-rust-codeguard-cli 2.1。新增共享Rust语言归一函数，公开检查类别/check的语言位置及plan的语言位置在分发前归一；只支持py/rs/ts/rb/kt/erl/golang/c++/c#九项显式映射，全部目标已在57项注册表中。grammar探针、任务参数、路径和工具路径不改写；不猜测JavaScript/TSX/bash方言，不改变能力或planned状态。

初始只读plan别名测试因未知语言ID失败，接线后九项均返回规范身份且项目目录没有新增文件。公开lint/check/cve/comments/build入口六组别名/规范名对照保留相同report_type/schema/delivery状态。CVE第一次对照因遗漏必需pip-audit上下文失败，测试补充不存在的显式工具与版本后恢复，产品参数约束不放宽。三项单元检查验证仅语言位置变化、grammar/路径/未知拼写保留、所有映射目标均注册。

三个CLI目标10通过/0失败/0忽略，别名单元3通过/0失败。CLI全目标严格Clippy、分层、diff及OpenSpec strict通过。环境缺工具不执行工具；测试不证明原生规则准确率或完整交付。TDD与日志摘要见 evidence/language-alias-cli-2026-10-06.json。没有运行用户Erlang草稿或全WASM测试，用户草稿摘要不变。

```bash
cargo test --offline --locked -p codeguard-cli --test language_alias_cli --test plan_preview_cli --test help_command_contract
cargo test --offline --locked -p codeguard-cli --lib language_alias::tests
cargo clippy --offline --locked -p codeguard-cli --all-targets -- -D warnings
```

2.1完整可信plan、全量义务/DAG、未实现检查类别与多平台仍缺，父任务保持开放。别名是已实现路由能力，不用于声称完整命令目录已实现。
