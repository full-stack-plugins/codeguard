# 误报白名单精确身份匹配切片

对应 OpenSpec `introduce-rust-codeguard-cli` 的 4.8 前半段。`codeguard-core::match_false_positive_identity` 只比较本次原生发现与候选裁定的完整身份：finding/checker/原生规则/类别/指纹、源码路径与文件字节摘要或依赖组件/版本/图/advisory，以及工具、适配器、规则包摘要。缺失身份、通配路径和目录越界均拒绝；相似文案不参与比较。

测试先因公共接口缺失而失败，增加纯领域实现后 5 个契约测试通过；覆盖同一源码命中、另一文件/内容失配、规则与工具版本变化、无效范围、依赖图/版本/advisory 变化。全工作区 `cargo test --workspace --offline -q`、`cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo fmt --all --check` 均退出 0。原生 P3C 等忽略测试未计入。

匹配成功**不是批准**：目前没有可信策略修订/签发来源校验、当前时钟/最长有效期核验、白名单 CLI、RunReport 1.3、门禁 `allow_with_exceptions` 或宿主对话映射。不得据此关闭 OpenSpec 4.8/2.10/12.12，也不得把 `Ok(())` 接到放行逻辑。

后续新增 `schemas/false-positive-decision.schema.json` 与 `codeguard-cli::false_positive_decision::parse_false_positive_decision_candidate`。解析器要求单条精确身份、误报类别、原因与复现引用、批准修订/引用/评审人及 UTC Unix 秒的有限期限；未知字段（含本地 `approved=true`）、通配/越界目标、错误摘要、缺批准引用和无期限候选均拒绝。`false_positive_decision_contract` 的 6 个测试先因接口/schema 缺失失败，后通过。解析结果明确命名为 Candidate，不能用批准字段文本当作可信授权；到期时钟、策略修订来源、最大期限和同一 PR 自批仍待独立边界验证。

候选集合现在可由 `classify_false_positive_candidates` 做确定性筛选：同一精确身份多条候选一律报冲突，不挑较新或未过期的一条；缺可信上下文字段、策略修订不符、创建时刻在未来、已到期或超过策略期限上限各有独立结果。目标测试新增四项，先因接口缺失失败后共 10 项通过。唯一通过筛选的状态仍叫 `ReadyForAuthorityCheck`，不能作为门禁 allow；调用方必须独立提供受保护策略身份和可信时钟，再核验批准引用及同 PR 自批边界。
