# TypeScript 统一 lint 入口的候选语法初检（2026-09-29）

启用 `codeguard-cli/wasm-precheck` 特性后，`codeguard lint typescript <显式单文件>` 在完全没有显式 Node/ESLint/config/cwd/version 上下文时，会尝试私有 Rust worker 的固定 TypeScript grammar，输出 `eslint_local_feedback` 0.3.0。原生状态固定 `not_run/explicit_context_missing`，不能据此声称工具未安装。语法观察分类为 `suspected`，位置转换为一起始 Unicode 标量列；`findings` 仍为空，整体 `incomplete`、退出 3、交付 `not_evaluated`、实际任务 ID 为 null。无恢复节点仍因候选 grammar 未验收而不返回 clean；worker 故障、源码读取或位置映射失败只形成未完成原因，不生成源码违规。

目标测试先看到 0.2.0 旧报告且缺少 `syntax_precheck` 字段而失败。实现后，疑似异常、无恢复节点、JS 文件不误选 TypeScript grammar、部分原生上下文、符号链接及旧原生结果路径均通过目标测试。该局部特性不代表项目本地工具准备探测已完成；仅在完全缺少显式执行上下文时补充观察，不覆盖已运行的原生 ESLint。老 0.2.0 schema 保留，新局部报告使用[独立 0.3.0 schema](../../schemas/eslint-local-feedback-v0.3.schema.json)。

示例（启用候选特性构建后）：

```bash
codeguard lint typescript src/bad.ts --format json
```

关键反馈形状：

```json
{
  "schema_version": "0.3.0",
  "report_type": "eslint_local_feedback",
  "status": "incomplete",
  "native": {"status": "not_run", "reason": "explicit_context_missing"},
  "syntax_precheck": {
    "backend": "bundled_tree_sitter_wasm_candidate",
    "status": "incomplete",
    "grammar_qualified": false,
    "observations": [{"classification": "suspected", "kind": "ERROR", "start_line": 1, "start_column": 19}]
  },
  "delivery_decision": "not_evaluated"
}
```

示例为字段摘录，不是可按 schema 直接消费的完整 JSON；完整字段以 CLI 输出和 schema 为准。当前公开 npm 0.1.0 不含这一可选构建，`check all`、任务同步、缓存及三宿主自动对话尚未接线。不能勾选 OpenSpec 14.6/14.7/14.9。

验收命令：

```text
cargo test -p codeguard-cli --features wasm-precheck --test typescript_syntax_fallback_candidate --test eslint_lint_cli
cargo clippy -p codeguard-cli --features wasm-precheck --all-targets -- -D warnings
cargo test --workspace --features codeguard-cli/wasm-precheck --locked --quiet
openspec validate introduce-rust-codeguard-cli --strict
```

实际结果：TypeScript 候选路径 5 项测试通过；既有 ESLint CLI 测试 6 项通过、4 项需显式真实 Node/ESLint 环境而 ignored；Clippy 无警告；启用候选特性的完整 Rust workspace 回归退出码 0；OpenSpec 严格校验与 `git diff --check` 通过。真实 ESLint 自动选择、正式任务同步和三宿主对话仍未验收。

后续增量：[TSX 候选纠错](tsx-grammar-candidate.md)将 `.tsx` 从普通 TypeScript grammar 改为固定 TSX grammar，并使用独立的 0.4.0 报告；本页 0.3.0 示例仍适用于 `.ts/.mts/.cts`。上述 5 项与完整 workspace 测试结果是当时的历史验收记录。
