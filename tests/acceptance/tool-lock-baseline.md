# 工具锁协议阶段验收（2026-09-24）

`schemas/tool-lock.schema.json` 定义 1.x 工具锁文档的工具、平台、适配器、规则来源、安装来源及可选运行时身份。`codeguard-cli::tool_lock::parse_tool_lock_document` 对未知 major、非法枚举、缺失/非小写 SHA-256、重复 `(tool_id, platform)` 和未声明权威字段拒绝解析；兼容 minor 只能通过 `x-` 前缀扩展携带非权威元数据。

`cargo test --offline -p codeguard-cli --test tool_lock_contract` 的 5 项测试通过，覆盖正反例及发布 schema 的基本结构。解析成功只证明文档符合协议，不证明锁文件经过批准、二进制真实存在、来源可信、运行时可启动，或项目安全边界已隔离。工具解析、真实身份校验、doctor、下载与来源批准仍属 OpenSpec 5.2/5.3；请求、计划、报告 schema 未齐前，2.2 继续未完成。
