# Erlang grammar 候选局部验收

日期：2026-09-29。对应 OpenSpec 14.1、14.4、14.19 的局部进展，任务保持未完成。

RED：新增候选契约测试时，`bundled_grammar_candidate("erlang")` 返回“不支持的 grammar 语种”。GREEN：复制固定 CodeGraph 提交中的 421639 字节 WASM，固定 WhatsApp 上游 `0.19` 标签的完整提交及许可证；清单校验、Rust 离线加载和隔离 worker 测试通过。CodeGraph 提交中原文件的 SHA-256 与入库资产相同；上游许可证的 SHA-256 与标签提交中的原文件相同。来源细节见 [grammar 说明](../../grammars/erlang/README.md)。

合法 `-module(hello). hello() -> ok.` 无恢复错误，损坏的 `hello( -> ok.` 有恢复错误；实测 ABI 14。worker 的合法样例仍返回 `incomplete` 和 `grammar_qualified=false`。`grammar status` 更新为 32 种来源、24 份候选、0 项已发行。上游 Zig 重新构建的 WASM 导入 `__main_argc_argv`，当前 Rust 加载器拒绝；本候选直接使用已能加载且逐字节固定的 CodeGraph WASM，尚无上游源码重建字节一致性证明。

未完成独立版本/方言语料、原生 Erlang 检查器对照、误报漏报评估、公开原生优先路由、任务和宿主对话反馈、资源预算及发行包实装。该局部验收不能证明 Erlang lint 可用，也不能把初检零恢复当作交付通过。
