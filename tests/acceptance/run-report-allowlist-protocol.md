# RunReport 1.3 误报处置消费协议切片

对应 OpenSpec `introduce-rust-codeguard-cli` 的 2.10 前半段。`run-report.schema.json` 当前标识为 1.3：1.0–1.2 仍可读取，1.3 要求 `dispositions`、原始阻断、已处置误报及活跃阻断集合。结构解析器核对每条处置只指向本次**完整**原生结果中的一个阻断 finding，批准修订/策略摘要与报告声明一致，重复、缺期限和含控制字符的批准引用均拒绝。它按活跃阻断解释退出码：唯一阻断已处置且全部义务完整时可表示 `allow_with_exceptions`；真实阻断仍 `deny`，独立义务缺口仍 `incomplete`。

对话摘要协议升至 0.2，保留原始 finding 和决策 ID、批准引用、修订、到期时间；human 显示“带批准例外”，同时明确批准来源仍待独立核验。原生自由文本不进入反馈。`run_report_allowlist_contract` 的 7 项目标测试先因新字段缺失失败，随后通过；旧 `run_report_contract` 的 14 项回归通过。`cargo test --workspace --offline -q`、`cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo fmt --all --check` 均退出 0。需外部 Maven/P3C 等的忽略测试未计入。

此处**只证明结构和消费语义**。未实现原生报告生产、可信策略/时钟/批准核验、正式 SARIF/MCP/Hook 接线及真实 `check all` 门禁；可解析的 `allow_with_exceptions` 声明不能由宿主当作可信交付凭据。OpenSpec 2.10 保持未完成。

## RunReport 1.4 完整身份绑定增量（2026-09-28）

当前 `run-report.schema.json` 标识为 1.4，原 1.3 schema 独立保留。1.4 的每条 finding 必须包含本轮原生身份；每条白名单处置必须包含与对应 finding **逐字段相等**的决策身份。解析器还核对原生规则、工具摘要、检查器类别与状态、主源码路径或依赖组件和版本。缺身份、使用旧源码摘要或指纹、改工具/适配器/rulepack、改依赖图或 advisory、以及未知未来版本，均不能成为有效 1.4 报告。1.3 及更早报告只可作为未核验历史声明读取，不能借新字段升级权限。

新增反例先因解析器不认识 `native_identity` 失败；实现后 RunReport 白名单 9 项、旧协议 14 项、SARIF 4 项通过。独立 Draft202012 对实际 1.4 报告给出正例；删除 finding 身份或处置身份的变体被拒。SARIF 仍保留误报结果，只标记未经来源核验，不创建 suppression，也不公开私有令牌。

1.4 的两个身份字段仍是**报告自称**。真实源码字节、原生工具输出、可信审批及其撤销、终局时钟须由独立宿主冻结和核验；当前解析器不能据此签发白名单或交付放行。正式报告生产、MCP/Hook 消费与完整门禁未完成，OpenSpec 2.10 保持未勾选。

## 源码覆盖与当前字节核对增量（2026-09-28）

新增反例先证明：把 finding 主定位、原生身份和决策身份一起改为未覆盖的 `src/other.py`，旧 1.4 解析器仍接受带例外报告。现解析器要求源码目标同时在对应义务的 `coverage.expected` 和 `coverage.observed` 中；该反例从失败转为拒绝。

新增宿主调用的只读 `compare_claimed_source_hashes(report, trusted_root)`，从宿主独立指定目录安全读取报告中所有源码目标的普通文件字节并比对 SHA-256。测试先因缺接口编译失败，接入后证实真实内容一致可取得对比结果；旧报告、同文件冲突摘要、文件修改、删除及 Unix 符号链接均失败。白名单目标测试组 12 项、既有报告 14 项、SARIF 4 项通过。它不会从报告所写的工作区路径推断可信根，也不核验原生工具或批准；终局判定仍需再次复核，本增量不构成白名单放行验收。
# 原始 JSON 重复键边界

检查请求与 RunReport 的公共入口现使用递归无重复键解析器，并限制原始输入为 16 MiB。重复 `delivery_gate`、内层 `decision` 或内嵌请求的 `jobs` 直接拒绝整份报告；不得先覆盖成普通 JSON 对象再做白名单或门禁结构校验。契约测试见 `check_request_contract::duplicate_nested_options_cannot_hide_an_earlier_value` 与 `run_report_contract::duplicate_gate_or_embedded_request_fields_cannot_be_consumed`。结构有效仍不证明原生来源或独立批准。
