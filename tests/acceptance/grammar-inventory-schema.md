# Grammar 库存 1.1.0 输出 schema 补齐

关联 OpenSpec S14.7。公开 0.1.4 的 `grammar status --format=json` 原有实际输出未改变；此项补齐源码协议 artifact 和开发验收，不声称程序重新发布。

先用真实 release 程序运行新增验收，因输出 schema 缺失而 RED；补齐 Draft 2020-12 封闭 schema 后，4 项验收通过。原报告退出 3、32 来源 grammar、30 随仓资产、32 未验收候选、0 已发布，来源库存不加载源码、不认证 parser 或交付。schema 要求语言集合完整且每种唯一、provider 数量一致、candidate_count 与实际候选行一致；已知限制有界，候选不得删除限制文本，未集成行保持空候选身份。未知 major、字段、authority、allow、released 和计数伪造均拒绝。

复现：使用本机已有 `jsonschema` 的开发 Python 执行 `python3 tests/grammar_inventory_schema.py`，可用 `CODEGUARD_INVENTORY_BIN` 指向需核验的程序。这是开发 JSON Schema 验证，不是产品 Python 内核；Rust 原报告及检查逻辑不变。库存来源与批准仍需独立验证，schema 不能给报告签发可信权威。S14.7 的全消费者与范围协议验收尚未完成。
