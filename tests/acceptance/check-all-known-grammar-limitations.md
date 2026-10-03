# `check all` 投影已知 grammar 限制的局部验收

日期：2026-10-03。对应 OpenSpec 14.7、14.17、14.19 的局部增量。

初始 RED 测试创建有效但未缩进的 VB.NET 方法和含 `SELECT FROM` 的 CFQuery 标签。统一 `check all --format=json` 虽返回候选观察，却没有给智能体提供清单中已知的 VB.NET 误报和 CFQuery SQL 覆盖缺口。随后在原生检查之后的候选阶段，只读取并校验内置固定清单元数据，把所选 grammar 的有界 `known_limitations` 附到每条观察；源码读取失败且尚无语言路由时使用空数组。真实 worker 仍自行核对选中资产字节，清单文字不能使恢复节点升级为确认违规。

聚合 `check_feedback` 从 0.33.0 升至封闭的 0.34.0；旧 Schema 独立归档。新字段要求数组最多 8 项、每项 1–1024 字符，仍要求 `grammar_qualified=false`、`delivery_decision=incomplete`。清单元数据无效时候选阶段报告 `grammar_manifest_invalid`，不继续解析或签发通过。真实双文件报告有 VB.NET、CFML、CFQuery 三条候选记录，VB.NET 和 CFQuery 都携带具体限制；无论某条 worker 观察是否完成，都不把恢复节点或零恢复当作原生判定。

目标回归先因缺字段失败，修复后 1/1 通过；六项候选路由测试 6/6、聚合契约测试 18/18（另 3 项按原标记忽略）。本机已有 `/opt/anaconda3/bin/python3` 的 Draft 2020-12 校验器接受真实 0.34.0 报告，拒绝旧 0.33.0 Schema、超长限制和伪造 `delivery_decision=allow`。Python 只用于独立协议验收，不属于 Rust 产品运行路径。

这一增量只改善报告中的可见限制。已发布 npm 0.1.3 仍嵌入旧协议；插件自动读取、对话渲染、原生对照、系统误报率与逐语言发行验收仍缺，父任务不勾选。
