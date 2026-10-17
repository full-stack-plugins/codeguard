# ESLint TypeScript 方言 tsconfig 输入绑定与必测反例验收

本记录对应 OpenSpec 7.3 的一个局部增量：TypeScript 方言（ts/tsx/mts/cts）文件在原生 ESLint 探针中按**包根 tsconfig.json** 冻结输入身份，并把任务要求的必测反例（parser 缺失、配置 invalid、版本不匹配、坏报告、超时、取消、monorepo 多包、排除文件）中此前缺失的聚合层反例补齐。全部为受控 shell/JS 夹具或真实 Node 受控入口，**不是真实 ESLint 验收**（本机已无 ESLint 10.x 与 npm 缓存，见"未运行"一节）。

## 实现内容（文件与行为）

- `crates/codeguard-cli/src/eslint_lint_command.rs`
  - 新增 `typescript_project_config(cwd, source)`：仅 ts/tsx/mts/cts 方言在包根（原生执行 cwd）查找 `tsconfig.json`；缺失返回 `None` 不阻塞；链接、目录或不可 stat 的特殊文件返回 `eslint_tsconfig_untrusted`，在任何原生执行前拒绝。
  - `observe_captured` 将普通文件 tsconfig 纳入冻结摘要集合（上限 1 MiB），与 Node/入口/配置/源码一同进入 `EslintProbeRequest.expected_sha256`；tsconfig 与已冻结输入同路径时去重。
  - `feedback()` 新增专属 next_action：`eslint_parser_or_configuration_diagnostic`（解析级致命诊断、方言 parser 覆盖指引）、`eslint_unattributed_diagnostic`（排除文件/无法归属诊断，被排除文件不能记为已检查）、`eslint_version_mismatch`（声明版本与原生输出不一致）、`eslint_report_invalid`/`eslint_report_read_failed`/`eslint_execution_evidence_failed`（报告缺失或不可解析，含配置无效导致无报告的情形）、`eslint_tsconfig_untrusted`。
- `crates/codeguard-cli/src/eslint_probe.rs`：与请求构造方镜像——TS 方言且包根存在 tsconfig（任何存在形态）即纳入探针输入核对；中途新建或换成链接都会被判为输入变化而非静默放行。
- `crates/codeguard-cli/src/eslint_preparation.rs`：`valid_reason` 接受 `eslint_tsconfig_untrusted`（可形成环境任务）；`guidance()` 增加对应修复步骤。
- 聚合 `check all`、hook 快检、`task verify` 复检与目录模式均经由同一 `observe_captured`，自动获得该绑定，无需各自修改。

## 语义边界（如实声明）

- 冻结 tsconfig 只证明**本轮观察绑定了该输入的字节身份**，不证明项目 parser（如 typescript-eslint）实际读取或适用该文件；无 tsconfig 的 TS 方言检查照常执行。
- JS 方言（js/jsx/mjs/cjs）不绑定 tsconfig；被原配置 `ignores` 排除的文件仍由既有 `eslint_unattributed_diagnostic` 路径保持未完成，本记录只补充其指引文案。
- 工作台持久观察（`eslint_workbench`）的 inputs 清单仍为原四项，未把 tsconfig 写入持久身份；跨运行复检通过重新冻结补齐，历史观察不含 tsconfig 摘要。
- monorepo 各包按最近 package.json 根分别绑定各自的 tsconfig；提升安装（hoisted、子包无 node_modules）的候选发现问题在 `eslint_native_first_candidate.rs`（非本切片文件）。

## 测试证据（先 RED 后 GREEN）

新增 `crates/codeguard-cli/tests/eslint_tsconfig_binding.rs`（7 项，受控 shell 夹具）：

| 用例 | 断言要点 | 基线结果 |
|---|---|---|
| tsconfig_mutation_after_report_invalidates_native_observation | 报告写完后 tsconfig 被追加 → `eslint_input_changed`、local_coherent=false、source_sha256=null、WASM 不被抑制、delivery incomplete | RED→GREEN |
| symlinked_tsconfig_is_untrusted_and_blocks_native_execution | 链接 tsconfig → `eslint_tsconfig_untrusted`，且原生入口零执行（marker 缺失） | RED→GREEN |
| monorepo_tsconfig_mutation_only_invalidates_its_own_package | frontend/backend 各自候选；仅 backend tsconfig 被篡改时 frontend 保持一致观察、backend 失效 | RED→GREEN |
| ts_parser_fatal_keeps_incomplete_and_names_parser_guidance | fatal 消息 → `eslint_parser_or_configuration_diagnostic`、零 findings、专属 next_action（含"解析级致命诊断/方言"） | 指引 RED→GREEN |
| invalid_config_exit_without_report_is_explicitly_incomplete | 退出 2 且无报告 → `eslint_report_read_failed`/`eslint_execution_incomplete`，非干净 | 基线即通过（回归） |
| native_version_mismatch_is_reported_and_not_laundered | 原生 `--version` 输出与包声明不符 → `eslint_version_mismatch` + 专属指引 | 指引 RED→GREEN |
| bad_report_is_rejected_without_clean_claim | 非 JSON 报告 → `eslint_report_invalid` | 基线即通过（回归） |

`crates/codeguard-cli/src/eslint_lint_command.rs` 内联单元测试 2 项：方言路由矩阵（js/mjs 不绑定；ts/tsx/mts/cts 绑定包根普通文件；链接与目录返回 `eslint_tsconfig_untrusted`；外部包根无输入不阻塞）。

`crates/codeguard-cli/tests/eslint_probe_contract.rs` 忽略测试扩展第 8 模式：真实 Node 24.18.0（`CODEGUARD_NODE_BIN`，受控 JS 入口产出报告后篡改 tsconfig）→ `eslint_input_changed` 且保留 parsed。

超时/取消与排除文件的既有验收维持不变（`interrupted_scan_preserves_cancel_and_deadline_reason_with_or_without_report`、`ignored_native_file_is_not_mistaken_for_completed_syntax_coverage`），本轮全部通过。

## 实际执行记录（2026-10-07）

- RED：`cargo test --offline --locked -p codeguard-cli --features wasm-precheck --test eslint_tsconfig_binding` → 5 failed / 2 passed（失败均为本记录目标缺口；其中 1 例为夹具自身 bug 修复后复确认）。
- GREEN：同命令 → 7 passed / 0 failed（4.19s）。
- 真实 Node：`CODEGUARD_NODE_BIN=/Users/wandl/.nvm/versions/node/v24.18.0/bin/node cargo test --offline --locked -p codeguard-cli --features wasm-precheck --test eslint_probe_contract real_node_controlled -- --ignored` → 1 passed（7.62s，8 模式含 tsconfig 篡改）。
- 回归：`--lib` 146 passed / 0 failed（12.08s）；`--test check_all_eslint`（14）、`eslint_lint_cli`（798 行目标内相关用例通过）、`eslint_probe_contract`（2 passed / 2 ignored）、`eslint_directory_cli`、`eslint_config_map_cli`、`eslint_effective_interrupt`、`eslint_effective_rule`、`eslint_finding_identity`、`eslint_discovery_contract`、`eslint_native_argv`、`typescript_syntax_fallback_candidate`（2）、`javascript_lint_candidate`（6）、`javascript_module_probe`（6）、`check_all_npm`（14）全部通过。
- `rustfmt --edition 2024 --check` 对三个改动源文件无差异。
- 目标 Clippy 本轮**未运行**（8 个并行智能体共享磁盘/编译资源，按切片约束只跑指定 lib 测试与受影响目标）。

## 未运行 / 未证明

- 真实 ESLint 10.x 原生执行**未运行**：本机 npm 缓存为空（`~/.npm/_cacache` 仅 4KB），仅存 ESLint 6.8.0/8.57.1，适配器要求 major 10；`real_eslint_observes_native_rules_suppression_parser_and_repair` 维持 ignored。
- 不证明：parser/tsconfig 实际读取闭包、本地插件闭包、完整项目源集、required 规则批准覆盖、可信宿主门禁、正式任务关闭/重开。OpenSpec 7.3 保持未完成。
