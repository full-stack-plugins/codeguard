# Rust grammar 与原生 rustfmt 的局部语法差分

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的局部增量，不是 Rust lint 验收。

固定本机 `rustfmt 1.9.0-stable (48a229ceae 2026-09-01)`，实际程序位于 Rustup stable toolchain，SHA-256 `98e8da71078a8b5710f1818c98da25d92de162a122bd9a6db222ca929e545468`。用 `rustfmt --emit stdout --edition 2021 FILE` 只读解析每份样例；CodeGuard 对同一普通源码调用公开 `grammar probe rust FILE --format=json`，并要求真实 worker 返回候选观察、零授权和退出码 3。

共 13 例：8 份合法样例覆盖普通函数、泛型、async、macro_rules、raw string、生命周期、match guard 与 const generic；5 份破损样例覆盖缺失大括号、圆括号、未闭合字符串、缺失表达式与破损 match。原生与候选的合法/破损分类全部一致。普通固定语料测试 1/1 通过；显式设置 `CODEGUARD_RUSTFMT_BIN=/opt/homebrew/opt/rustup/bin/rustfmt` 运行忽略的真实原生差分测试 1/1 通过。测试还核对 rustfmt 不修改源文件。

初次原生差分曾错误地只接受 rustfmt 退出 0/1；未闭合字符串实际返回 101 并给出 `error[E0765]`。测试现仅在非零状态伴随错误诊断、且没有 panic 文本时把它认作源码拒绝，防止工具崩溃被计为语法命中。

此对照只覆盖本机 rustfmt 版本及 Rust 2021 的 13 份小样例。`rustfmt` 的解析接受与完整 `rustc` 类型/构建检查、Clippy 规则和其它 edition 不等价；当前 grammar 仍 `grammar_qualified=false`，零恢复节点仍不表示原生 lint 或交付通过。尚无独立 holdout、系统误报/漏报率、跨平台版本、完整原生优先路由和宿主反馈，父任务不勾选。
