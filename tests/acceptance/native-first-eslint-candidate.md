# TypeScript 本地 ESLint 候选优先：局部验收（2026-09-29）

对应 OpenSpec 14.6。启用 `wasm-precheck` 的单文件 `lint typescript` 在缺少显式原生参数时，先只读核对源码最近的 `package.json`、普通 `node_modules/eslint/package.json` 与 `bin/eslint.js`，以及唯一的普通 `eslint.config.js/mjs/cjs`。只有适配器将其分类为 `local_candidate_requires_native_probe` 时，命令不启动 WASM worker，返回未完成的原生准备反馈，并提示先核对 Node 路径、版本与原配置。旧 `.eslintrc`、TypeScript 配置及多个 flat config 不作为可直接选取的候选。候选的存在不等于原生可运行；本增量不自动执行本地入口，也不授予交付通过。

先新增真实 CLI 反例：项目本地 ESLint 10 包、入口和配置可见时，旧命令仍启动 WASM，断言因出现 `syntax_precheck` 而失败。接线后该用例通过。另一个反例保留相同声明和配置但不安装本地包，仍允许候选初检，且 `native=not_run`、`delivery_decision=not_evaluated`。`eslint_lint_cli` 现有显式原生路径与 `grammar_status_cli` 一并回归。

本次定向测试：`CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-cli --features wasm-precheck --test typescript_syntax_fallback_candidate --test eslint_lint_cli --test grammar_status_cli`，共 16 通过、4 项需要显式真实 Node/ESLint 的用例忽略。候选探测不执行项目 JavaScript、不修改 `node_modules`，不把“未给显式参数”当作“原生工具不存在”。

仍缺：可信 Node 路径与原调用参数解析、受控版本探针、动态配置有效性确认、项目本地原生自动执行、多模块原生优先调度及真实宿主反馈。因此 14.5/14.6 均不勾选。
