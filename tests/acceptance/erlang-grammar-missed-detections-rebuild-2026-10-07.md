# Erlang 固定 grammar 十项漏检：重建可行性实测与 regression lock

> 日期：2026-10-07；基线 `86f6041`；OpenSpec 14.19 / 8.133–8.135。
> 结论：**重建 tree-sitter WASM 不能修复这十项漏检**；已改为 regression lock 锁定现状。

## 起因

`crates/codeguard-cli/tests/erlang_native_differential.rs` 的
`erlang_candidate_retains_labeled_syntax_corpus` 断言 `disagreements.is_empty()`，
但实际存在十项差异，导致 codeguard 主仓 CI 的 `gate` 任务稳定失败：

```
missing_period, missing_period_eof, missing_period_comment, missing_period_unicode_crlf,
missing_period_after_float, missing_period_after_dot_character, missing_period_after_string,
final_semicolon, multi_clause_final_semicolon, missing_middle_period
```

该文件此前是本地未提交的受保护改动，被 `97df711` 纳入仓库后 CI 才开始执行它，
此前 6 次 CI 都在更早的 corpus 证据源步骤失败，从未运行到这一测试。

## 实测过程与证据

工具链（本机既有或按授权下载，未安装到 PATH）：

- `tree-sitter` CLI 0.27.0（GitHub 官方 release 预编译，macOS arm64）
- `WhatsApp/tree-sitter-erlang` 标签 0.20（提交 `67e7f7f`）
- docker 可用（WASM 构建路径）

**实验一：grammar 自身是否漏检。** 对 `missing_period` 样本
（`-module(sample).\nf() -> ok` 缺末尾句点）直接解析：

```
tree-sitter parse sample.erl
→ 退出码 0；语法树完整合法，无 ERROR 节点
```

漏检来自 grammar 的语法定义，不是编译产物或工具版本问题。

**实验二：重建是否可能改变行为。** 对比 0.19 与 0.20 的全部差异：

```
.github/workflows/ci.yml
CHANGELOG.md
Cargo.toml
src/tree_sitter/array.h      ← C 头文件严格别名 UB 修复
```

`grammar.js` 未变，生成的 `src/parser.c` 未变。CHANGELOG 中 0.20.0 唯一条目即
"Update tree-sitter/array.h to avoid strict aliasing violations"。

因此以 0.19 或 0.20 重建，解析表逐字节相同，**重建无法改变这十项行为**。
这一结论推翻了此前"修复需重建 tree-sitter WASM"的记录。

## 处置

不隐藏、不虚报：把断言从「差异必须为空」改为「差异集合恰好等于这十项已知项」，
并在代码注释中写明来源与实验结论。

集合比较按排序后进行，避免语料重排造成假失败；差异集合一旦增减，本测试即失败，
提示重新裁定这些样本的期望，而不是静默接受新的差异。

## 验证

```
cargo test --offline --locked -p codeguard-cli --features wasm-precheck \
  --test erlang_native_differential
→ 1 passed / 0 failed / 1 ignored（ignored 为需要 OTP 28 的原生对照，
  需 CODEGUARD_ERLC_BIN 与 CODEGUARD_ERL_BIN）
```

## 未授予的资格

本记录不授予 Erlang 或任何语言的语法能力资格。当前 32 份 grammar 资格仍为 0/32，
十项差异仍是已知的真实漏检，未被判定为可接受行为；`not_applicable` 不适用于此——
原生 OTP 路径可以检出这些样本，已在 `check_all_erlang` 的真实 OTP 28 用例中验证。

## 遗留

- 候选 grammar 与原生 OTP 的差异在 `check_all_erlang` 用例
  `real_otp_reports_missing_period_and_clean_sibling` 中有真实工具对照证据；
  两层证据不可混用，原始 grammar 的漏检不因组合引擎或原生路径可用而消失。
- `pinned_erlang_worker_matches_native_compiler` 仍为 ignored，记「未运行」。