# 原生工具准备状态候选：局部验收（2026-09-29）

OpenSpec 14.5 要求在建议安装工具前识别项目本地工具、Wrapper、版本与无效配置。`PATH` 上没有命令不能证明工具未安装；配置损坏不能变成反复安装建议。adapters 提供纯只读分类，CLI 现在通过不跟随链接的观察端口读取项目固定路径，并在 `detect` 0.4.0 与 `check_feedback` 0.31.0 的 `discovery.native_tool_candidates` 输出候选；仍未执行任何原生工具。旧发现 0.3.0 与聚合 0.30.0 schema 已独立保留。

| 场景 | 候选状态 | 不能推断的结论 |
|---|---|---|
| ESLint 10 的本地包身份和入口均可见 | `local_candidate_requires_native_probe` | Node、ESLint 和项目规则本轮已可执行 |
| ESLint 仅在依赖中声明，或本地包与声明版本冲突 | `declared_local_entry_unobserved` / `local_version_conflicts_with_declaration` | 需要重新安装或代码违规 |
| ESLint 配置、项目清单或本地包身份损坏；入口是链接 | 分别标记配置、清单、身份或入口问题 | PATH 缺失、源码违规 |
| Maven Wrapper 脚本和固定 Maven 3 发行配置可见 | `wrapper_candidate_requires_native_probe` | Wrapper 可安全执行或 Maven 检查通过 |
| Maven Wrapper 配置重复、脚本是链接、仅见单侧文件、Maven 4 | 分别标记配置、脚本、证据不完整或适配器版本缺口 | 应重复安装 Maven 或按 Maven 3 放行 |

TDD：两个测试文件先因缺少候选 API 编译失败，随后实现纯分类。执行 `CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-adapters --test eslint_local_readiness --test maven_wrapper_readiness`，共 6 个测试通过；`cargo test --workspace --locked --features codeguard-runtime/wasm-precheck --quiet` 及同特性的全目标 Clippy `-D warnings` 也通过。真实外部工具样本仍按其显式环境要求跳过。测试不访问 PATH、不执行脚本、不下载包；“待原生核验”是候选状态，不是 clean/ready 证明。

CLI 真实目录样本先在发现报告 0.3.0 上失败，再验证本地 ESLint 10 包身份、普通入口、Maven 3 Wrapper 配置和有界路径观察；`node_modules` 普通源码递归被排除并计数。链接包和 `.mvn` 目录返回不可信阻塞、退出 3；链接 ESLint 配置被归为配置问题，不发重复安装建议。候选只公开版本和状态，不公开项目原始依赖声明；human 输出也明确标记“未执行”。

聚合 `check all` 样本进一步验证：本地 ESLint 候选进入 `discovery.native_tool_candidates`，聚合版本为 `0.31.0`，义务仍为 unresolved、退出 3；候选不会成为执行结果或 allow。`cargo test --workspace --locked --features codeguard-runtime/wasm-precheck --quiet` 在当前改动上退出 0；全目标 Clippy `-D warnings` 与 `scripts/check_layering.py` 通过。一次并行定向测试的 Python CVE 输出上限样本先返回另一种 incomplete 原因，单独重跑通过；全量回归通过，但这一时序波动仍需后续稳定性排查。

局限：尚未覆盖 pnpm/Yarn 布局、Maven 非 Wrapper 路径、Windows `mvnw.cmd`、显式工具和其他语言工具。原生版本/配置探针、native-first 调度、WASM 兜底及真实宿主对话反馈仍需实现；因此 14.5 和 14.6 保持未完成。
