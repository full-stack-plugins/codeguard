# TypeScript 本地 ESLint 候选优先：局部验收（2026-09-29）

对应 OpenSpec 14.6。启用 `wasm-precheck` 的单文件 `lint typescript` 在缺少显式原生参数时，先只读核对源码最近的 `package.json`、普通 `node_modules/eslint/package.json` 与 `bin/eslint.js`，以及唯一的普通 `eslint.config.js/mjs/cjs`。只有适配器将其分类为 `local_candidate_requires_native_probe` 时，才尝试从当前 `PATH` 选择普通可执行 Node 文件；候选本身不能证明原生已运行。旧 `.eslintrc`、TypeScript 配置及多个 flat config 不作为可直接选取的候选。

若能解析到 Node，CLI 使用本地包清单中的具体版本、项目入口/配置和构建根调用现有受控原生探针；Node、入口、配置和源码均由探针按字节核对，版本输出与报告范围必须吻合。探针得到的发现仅是局部原生观察，仍为 `delivery_decision=not_evaluated`。若当前 `PATH` 无可用 Node，仅返回原生准备动作，不把 PATH 缺失当作项目工具未安装，也不启动 WASM 抢跑。项目只声明 ESLint、没有本地入口时仍可提供候选语法初检。

先新增真实 CLI 反例：项目本地 ESLint 10 包、入口和配置可见时，旧命令仍启动 WASM，断言因出现 `syntax_precheck` 而失败。接线后该用例通过。下一条端到端用例用受控 Node 命令与本地 ESLint 包建立真实子进程调用，先因 `local_coherent=false` 失败；自动原生调度接线后原生 `no-debugger` 发现进入报告，私有原生消息未泄漏。随后让版本输出从 `10.0.0` 变为 `10.0.1`，确认返回 `eslint_version_mismatch`，不执行 WASM 来洗白失败。另一个反例保留相同声明和配置但不安装本地包，仍允许候选初检，且 `native=not_run`、`delivery_decision=not_evaluated`。

本次定向命令：`CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-cli --features wasm-precheck --test typescript_syntax_fallback_candidate --test eslint_lint_cli --test grammar_status_cli`，17 项通过，4 项要求显式真实 Node/ESLint 的用例仍忽略。先前只读候选阶段为 16 项通过，不能把该旧数字作为本次自动原生阶段的验收。当前机器有 Node 24.18.0，但 PATH 上没有 `eslint` 命令，本轮未取得真实 ESLint 10 的原生运行证据。CLI 不安装或修改 `node_modules`，但自动原生路径会按用户的 `lint` 请求执行项目本地 ESLint 及其原配置中的 JavaScript；因此这仍是受控检查执行，不是只读发现命令。

仍缺：项目自定义脚本与参数解析、动态配置/插件闭包的完整性确认、pnpm/Yarn/多模块布局、真实 ESLint 10 运行验收、跨平台与宿主反馈。当前固定选择 `--no-config-lookup --config` 的显式原生命令，未证明它与项目脚本所有行为等价。14.5/14.6 均不勾选。
