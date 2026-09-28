# 原生工具准备状态候选：局部验收（2026-09-29）

OpenSpec 14.5 要求在建议安装工具前识别项目本地工具、Wrapper、版本与无效配置。`PATH` 上没有命令不能证明工具未安装；配置损坏不能变成反复安装建议。本切片在 adapters 中增加纯只读分类，输入来自调用方的有界文件内容和无跟随路径类型观察，输出只是候选，未执行任何原生工具。

| 场景 | 候选状态 | 不能推断的结论 |
|---|---|---|
| ESLint 10 的本地包身份和入口均可见 | `local_candidate_requires_native_probe` | Node、ESLint 和项目规则本轮已可执行 |
| ESLint 仅在依赖中声明，或本地包与声明版本冲突 | `declared_local_entry_unobserved` / `local_version_conflicts_with_declaration` | 需要重新安装或代码违规 |
| ESLint 配置、项目清单或本地包身份损坏；入口是链接 | 分别标记配置、清单、身份或入口问题 | PATH 缺失、源码违规 |
| Maven Wrapper 脚本和固定 Maven 3 发行配置可见 | `wrapper_candidate_requires_native_probe` | Wrapper 可安全执行或 Maven 检查通过 |
| Maven Wrapper 配置重复、脚本是链接、仅见单侧文件、Maven 4 | 分别标记配置、脚本、证据不完整或适配器版本缺口 | 应重复安装 Maven 或按 Maven 3 放行 |

TDD：两个测试文件先因缺少候选 API 编译失败，随后实现纯分类。执行 `CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-adapters --test eslint_local_readiness --test maven_wrapper_readiness`，共 6 个测试通过；`cargo test --workspace --locked --features codeguard-runtime/wasm-precheck --quiet` 及同特性的全目标 Clippy `-D warnings` 也通过。真实外部工具样本仍按其显式环境要求跳过。测试不访问 PATH、不执行脚本、不下载包；“待原生核验”是候选状态，不是 clean/ready 证明。

局限：目前尚未通过 `ObservationPort` 从真实项目读取这些输入，也未覆盖 pnpm/Yarn 布局、Maven 非 Wrapper 路径、Windows `mvnw.cmd`、显式工具和其他语言工具。原生版本/配置探针、CLI 统一入口、报告与智能体对话反馈仍需实现；因此 14.5 和 14.6 保持未完成。
