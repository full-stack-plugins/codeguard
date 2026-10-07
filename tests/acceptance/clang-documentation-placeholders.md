# C-family 明确占位说明观察

对应 introduce-rust-codeguard-cli 的详细文档规范要求。新增独立适配器 parse_clang_documentation_placeholders 和封闭协议 clang-documentation-placeholder-v0.1；旧结构协议 nonempty 的含义保持不变。

仅整个受支持说明经空白与句末标点归一化后等于 TODO、TBD、FIXME、待补充或待完善时，报告 codeguard.documentation.placeholder_description。用途、命名参数及适用返回分别定位；输出不含注释原文。含有标记的正常解释、void 返回和结构解析器不支持的上下文不推断违规。此规则属于 Codeguard 自有策略，不冒充 Clang 原生警告，不证明剩余文本的语义准确性。

测试先因缺失能力失败。实现后合成正反例与真实 Clang C11 AST 测试均通过：2 项通过、0 忽略。实际原生测试首次把函数行号误写为5而失败，纠正为源码第4行后通过。实际输出通过 Draft202012 schema 校验；3 个伪造 qualification、coverage 和 authority 的变异被拒绝。完整命令如下：

```sh
CARGO_PROFILE_TEST_DEBUG=0 CODEGUARD_CLANG_BIN=/usr/bin/clang \
  cargo test --offline --locked -p codeguard-adapters \
  --test clang_documentation_placeholder_contract -- --include-ignored
```

可用 CODEGUARD_PLACEHOLDER_EVIDENCE 指定绝对路径保存实际观察供 schema 校验。该输出是局部观察，不是绑定发布制品的生产资格收据。

未完成：公开 comments 命令、稳定任务与修复指引、原任务 verify、尝试输入规则身份、历史消费兼容、白名单与可信关闭、跨语言/平台独立精度验收。新文件不会因存在或测试通过而使语言生产义务升级，相关父任务继续未勾选。
