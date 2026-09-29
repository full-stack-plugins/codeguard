# 语法初检后的原生准备动作：局部验收

对应 OpenSpec `introduce-rust-codeguard-cli` S14.8 与 `syntax-precheck` 的原生准备要求。

2026-09-29：先增加 `syntax_setup_guidance` 六项反例测试，因 core 尚无策略接口而编译失败；实现后定向测试 6/6 通过。`cargo test --workspace --all-features -- --test-threads=1 -q`、`cargo clippy --workspace --all-features --all-targets -- -D warnings` 与 `openspec validate introduce-rust-codeguard-cli --strict` 均退出 0。

| 输入 | 纯策略动作 |
|---|---|
| 缺可选工具、非空完整且全部合格的 clean | 仅推荐原生工具 |
| 已有必需义务，或疑似/未完成/不支持/未执行初检 | 必须恢复工具并确认 |
| 配置无效 | 修复原生配置并确认，不重复建议安装 |
| 原生执行失败 | 恢复原生执行并确认，保留局部原生结果 |
| 无适用原生适配器 | 要求明确能力决策，不推荐不存在的工具 |
| 只篡改 `status=clean`，但空范围或未验收 | 仍为必需确认 |

该函数不执行工具、不更改任务、不签发交付结论。上游原生工具发现、配置来源与模块义务尚未接入此策略；Java/TypeScript 候选入口的“缺显式参数”不能被当作“工具确实未安装”。CLI 报告、稳定任务和真实宿主对话仍需独立验收。
