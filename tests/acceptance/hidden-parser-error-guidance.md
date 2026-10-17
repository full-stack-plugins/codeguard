# 隐藏解析错误的原生确认指引

日期：2026-10-06，对应 S14.5 / S14.11 / S14.17 / S14.19。状态：反馈缺陷修复；grammar 精度缺陷未修复。

## 本轮证据与根因

本机已安装 CodeGraph 的 web-tree-sitter 0.25.10 与 Rust tree-sitter 0.25.10 加载相同 SHA-256 的 Kotlin/Swift grammar，三个固定样例都 has_error=true，公开遍历却无 ERROR/MISSING 节点。Web 输出的 S-expression 分别记录隐藏 `_alpha_identifier`、`_automatic_semicolon` 和 `simple_identifier_token1` 缺失。核对固定 Tree-sitter 0.25.10 的 [node.c](https://github.com/tree-sitter/tree-sitter/blob/v0.25.10/lib/src/node.c) / [subtree.c](https://github.com/tree-sitter/tree-sitter/blob/v0.25.10/lib/src/subtree.c)：公开子节点 API 过滤不可见节点，字符串打印包含 missing 子树。因此本批证据不支持“换成 web-tree-sitter 即消除问题”。

[实际 Web 参考输出](evidence/hidden-error-web-reference-2026-10-06.json)记录已安装参考版本、JS/WASM 与 grammar 摘要；不增加产品 Node 解析运行时，不安装依赖。Rust 工作进程用同字节样本复核相同 grammar、零公开恢复与不可定位状态。这不是完整树等价、源码合法性或新一轮编译器差分证明。

固定 [Kotlin grammar](https://github.com/fwcd/tree-sitter-kotlin/blob/e1a2d5ad1f61f5740677183cd4125bb071cd2f30/grammar.js) 的类成员语句要求分隔符；是否如何修复仍需生成与对照验收，不能将放宽所有分隔符作为既定方案。

Kotlin 合法 `object C { val value = 1 }` 也出现隐藏自动分号恢复；不得仅因 has_error 就判源码违规。缺参数类型的 Kotlin/Swift 样例保留原生确认义务。原生标签及精度差异的范围仍以现有差分记录为准。

## 修复行为

旧 grammar probe 对零公开恢复一律给出常规 lint 建议，即使本轮恢复未完整取得。新增运行时不可定位标记，并在私有协议1.4与公共 probe0.5中明确传递：

```json
{
  "schema_version": "0.5.0",
  "parser_error_location_unavailable": true,
  "next_action": "compare_original_source_with_native_tool_then_review_grammar"
}
```

上述仅为字段摘录；[三份完整实际报告](evidence/hidden-error-probe-2026-10-06.json)保留当前程序摘要。原字节不修改，恢复数组不制造位置；保持 incomplete、grammar_qualified=false 和 delivery_decision=not_evaluated。本轮须确认原生工具可用并对原字节检查；原生确认合法则转 grammar 兼容性调查，原生确认语法问题才按原生诊断修复。

预算耗尽与已访问的不可定位错误分开记录；既有 truncated 聚合保持兼容，以免旧消费者误认为完整。只有观察到不可定位错误才发送新字段；旧私有协议携带该字段、新协议缺失/为false或声称未截断，父进程均拒绝。无此错误的公开0.1/0.2/0.3/0.4路径保持原件。全项目/Hook继续沿用已有“恢复不完整”协议，不把本批宣称为所有公共入口都输出新字段。

## 验证与限制

公开行为测试实际 RED：旧0.1、常规lint动作与目标要求不符；修复后三个实际样例通过。runtime恢复扫描六项通过，包括正常兄弟预算耗尽不冒充隐藏错误；新公开schema的真实报告及假通过反例两项通过。七个CLI相关目标75通过/0失败/4条件忽略；默认/WASM两个受影响crate全目标严格Clippy、OpenSpec strict、分层、定向格式及diff检查通过。完整WASM workspace未执行，以保护用户Erlang草稿；完整358例回放未重跑，历史缺陷保留。

本批不替换任何 grammar 字节、不使用字符串解析伪造缺失token位置、不消除三例未知或 Erlang/VB.NET差异，不授予32语言资格。grammar重建、独立原生holdout、真实宿主和发行仍开放。未执行或修改用户Erlang差分草稿。
