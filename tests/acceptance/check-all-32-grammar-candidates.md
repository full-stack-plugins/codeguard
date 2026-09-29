# `check all` 的 32 份 grammar 候选路由局部验收

日期：2026-09-29。对应 OpenSpec S14.6、14.7、14.17、14.19 的局部进展，父任务均未完成。

源码以 `--features wasm-precheck` 构建后，`check all` 先运行现有原生节点，再对发现的普通源码执行有界候选阶段。新增路由按文件后缀选择 30 份直接 grammar，并对 `.cfs` 选 CFScript、完整 `<cfquery>...</cfquery>` 标签体选 CFQuery；`.tsx` 使用 TSX、`.js` 使用 JavaScript。`.h` 与普通 `.sql` 不凭后缀猜测。单次真实 CLI 集成样例创建 31 个源码文件，报告中出现 32 个 `candidate_observed`，其语言集合与固定资产清单完全相同。测试保持退出码 3、`delivery_decision=incomplete`、`grammar_qualified=false`、`skipped_count=0`。

第一次 45 秒候选预算的全量样例只完成 26 种，Swift 在共同截止时间超时，后续 5 个文件被跳过；这项失败证据促使候选阶段上限改为 90 秒。第二次同机样例 54.09 秒完成 32 种。该时间包括现有 `check all` 原生节点、每份资产的隔离工作进程及高成本 COBOL；不是冷暖启动统计、性能承诺或所有平台的内存验收。候选阶段还限制 64 个文件、64 个片段，每个 worker 限 1 MiB 源码和 64 KiB 输出；未观察的范围与原生阻塞保持未完成。

反馈协议由 `check_feedback` 0.31.0 升为 0.32.0，旧 Schema 独立保存。新增 `syntax_candidates` 只可表达未执行或部分候选观察；恢复位置被映射到原文件字节与行列，最多展示 8 个恢复节点，原生结果不会被覆盖。真实 `.tsx` 报告通过 Draft 2020-12 封闭 Schema 验证；原有聚合契约、单文件 probe 与路由测试通过。源码在候选 worker 执行前后复读，变化时放弃该片段观察。

源码可选构建的完整 CLI 测试集退出 0（需外部原生工具的用例仍按原标记忽略）；默认无 WASM 构建的受影响聚合/CVE 用例、全目标 Clippy `-D warnings`、格式、分层检查和 OpenSpec strict 验证通过。取消后的定向回归确认候选阶段不启动，保留退出码 130；无 WASM 真实报告仍可通过 0.32.0 Schema，伪造 `allow` 被拒绝。完整 CLI 回归早于最后一处取消路径加固，之后已单独重跑该路径的两种构建及 Clippy。

另有 1 MiB 超限源码反例：本轮未启动对应 worker，不生成 grammar 摘要或疑似源码违规，聚合记录 `attempted_incomplete/candidate_unavailable`。该反例及 Clippy 在最后一次协议补充后已单独重跑。

仍缺逐语言版本/方言语料、原生语法能力的差分对照、误报/漏报量化、为 32 种语言分别选择适用原生工具与配置、候选任务同步、宿主对话反馈、无 WASM 构建与各发行平台实装。已知 CFQuery 漏检和 VB.NET 误报依旧存在，因此本次路由不能升级为已验收 lint 或交付许可；S14.6、14.7、14.17、14.19 不勾选。
