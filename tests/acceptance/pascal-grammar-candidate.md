# Pascal grammar 候选局部验收

日期：2026-09-29。对应 OpenSpec 14.1、14.4、14.19 的局部进展，任务保持未完成。

RED：新增候选契约测试时，`bundled_grammar_candidate("pascal")` 返回“不支持的 grammar 语种”。GREEN：CodeGraph 固定提交中的 716886 字节 WASM 与本目录资产逐字节相同；原始 `package.json` 把上游锁定为 `Isopod/tree-sitter-pascal@042119eca2e18a60e56317fb06ee3ba5c32cb447`，许可证与该提交原文件字节相同。来源细节见 [grammar 说明](../../grammars/pascal/README.md)。

Rust 离线加载实测 ABI 14。`program Hello; begin writeln('Hi'); end.` 无恢复错误，缺少 `end.` 的反例有恢复错误；隔离 worker 的合法样例仍返回 `incomplete` 和 `grammar_qualified=false`。`grammar status` 更新为 32 种来源、25 份候选、0 项已发行。

尚缺逐版本 Delphi/FreePascal 正反语料、原生工具对照、误报漏报评估、公开原生优先路由、任务和宿主对话反馈、资源预算及发行包实装。该局部验收不证明 Pascal lint 可用，也不允许以初检零恢复批准交付。
