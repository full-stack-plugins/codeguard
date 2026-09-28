# Java 单文件 WASM 候选初检局部验收（2026-09-29）

本切片只适用于启用 `codeguard-cli/wasm-precheck` 的源码构建。`lint java FILE` 在完全没有显式 Maven/P3C 上下文时，若目标是普通 `.java` 文件，复用固定 Java grammar 的私有 Rust worker 给出疑似语法恢复位置。原生状态保持 `not_run`；grammar 版本/方言尚未验收，因此有无恢复节点均为 `incomplete`、退出 3、交付 `not_evaluated`。报告不制造已确认 finding 或任务 ID，不写入工作台；下一步要求适用的 Java 原生语法能力复核，P3C 风格规则本身不能单独确认语法。

TDD 先运行新增测试：疑似恢复、无恢复与 human 摘要三项因仍返回既有 P3C 报告而失败，部分上下文/链接反例先通过。实现后六项目标测试通过，覆盖疑似定位、无恢复不假通过、超大输入不当源码违规、部分原生参数/Javadoc/链接/非 Java 文件不选候选、human 摘要及封闭 schema 字段。既有 P3C 六项普通测试与 TypeScript 五项候选测试通过；两项需要真实 Maven/JDK/P3C 闭包的 P3C 用例仍为 ignored，不将其计入本次原生验收。

默认 `cargo test --workspace --all-targets --locked --quiet` 完整退出码 0；默认构建不启用此候选。目标 `cargo clippy -p codeguard-cli --all-targets --features wasm-precheck --locked -- -D warnings`、目标文件 rustfmt、分层检查、OpenSpec 严格校验和 `git diff --check` 均通过。仓库级 `cargo fmt --all -- --check` 在本次修改前已有其它模块格式漂移，未据此把整个仓库称为格式通过，也未批量格式化无关文件。

独立 Draft 2020-12 校验使用本机已有 `/opt/anaconda3/bin/python3` 的 `jsonschema 4.25.1`，对实际二进制输出的坏/好 Java 文件均通过 [0.1.0 schema](../../schemas/java-syntax-precheck-feedback-v0.1.schema.json)；把任一输出篡改为 `allow`、`coverage_proven=true` 或 `status=clean` 均被拒绝。此 Python 仅是独立验收工具，不是产品运行路径。

```text
cargo test -p codeguard-cli --features wasm-precheck --test java_syntax_fallback_candidate --test java_p3c_cli --test typescript_syntax_fallback_candidate --locked
cargo clippy -p codeguard-cli --all-targets --features wasm-precheck --locked -- -D warnings
rustfmt --check --edition 2024 crates/codeguard-cli/src/java_syntax_precheck.rs crates/codeguard-cli/src/java_lint_dispatch.rs crates/codeguard-cli/tests/java_syntax_fallback_candidate.rs
openspec validate introduce-rust-codeguard-cli --strict
```

范围边界：这不是 `lint java .` 项目级调度，也未证明原生工具缺失；只证明调用者未提供显式原生执行上下文。项目本地 Maven Wrapper/JDK 自动选择、原生执行失败后的混合保留、grammar 语言版本验收、持久任务/宿主对话、macOS worker 内存硬隔离和正式交付门禁仍未完成。公开 npm `0.1.0` 未包含该可选构建特性。
