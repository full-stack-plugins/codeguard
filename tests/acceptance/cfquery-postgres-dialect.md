# CFQuery 的 PostgreSQL 方言原生对照

日期：2026-10-06。对应S12.11、S14.4/S14.17/S14.19，父任务未完成。此记录纠正泛SQL裁定的依据，并留下可复现的真实漏检；不宣称已修复CFQuery grammar或实现项目SQL适配器。

## 同字节两类结果

PostgreSQL官方[SELECT兼容性说明](https://www.postgresql.org/docs/18/sql-select.html#SQL-SELECT-COMPATIBILITY)允许空输出列表，但使用DISTINCT时不允许。没有明确数据库方言的源码不能被一概认定为“缺select list”。

| 固定源码 | PostgreSQL18.6 PREPARE | 固定CFQuery WASM |
|---|---|---|
| `SELECT FROM users` | 接受 | 零恢复节点 |
| `SELECT DISTINCT FROM users` | 语法错误SQLSTATE42601 | 零恢复节点，真实方言漏检 |

原生使用已缓存镜像`sha256:6c538e7206ea40ff740ef27883529390a690b6ead6ba96b44c67a9f7c638e8fd`，实际版本输出为postgres(PostgreSQL)18.6。启动参数明确--pull=never、--network none、tmpfs PGDATA；不下载镜像、不开放端口、不使用项目数据库。仅执行夹具临时表DDL和PREPARE，不执行两条待检SELECT。Rust runtime管理有界Docker进程；容器以本轮返回的ID绑定并显式清理，Drop仅兜底本轮ID。

真实原生证据为`evidence/cfquery-postgres-oracle-2026-10-06.json`。两份原`grammar probe`反馈为`evidence/postgres-{empty-select-list,distinct-empty-select-list}-grammar-2026-10-06.json`，分别绑定源码SHA256。固定WASM SHA256为9d4eaaec46eec7e6400d4b8d6c85fac7dfe33867ed5b4e5c218dfa7851c32558；两份仍incomplete、grammar_qualified=false、delivery_decision=not_evaluated。

## 语料与指标边界

原`cfquery-missing_select_list`一直是pending/provisional_syntax，未进入已裁定TP/FP/FN/TN分母；保留原源码、标签、清单和历史报告，不把本次PostgreSQL结果升级为所有CFQuery/SQL项目的标签。旧限制说明将零恢复描述为select-list风险，必须结合本记录理解，不能直接据此修改用户源码。带DISTINCT的样例给出更明确的原生反例，但仍仅限当前固定方言和夹具。

这两份有目的的样例不是独立holdout、不证明误报率、召回率、CFML模板求值、其他数据库、项目schema绑定、SQL注入安全或发行资格。不根据本次对照批准白名单或自动关闭问题。

## 执行与验收

```bash
cargo test --locked -p codeguard-cli --test cfquery_postgres_dialect
CODEGUARD_DOCKER_TOOL=/absolute/docker cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test cfquery_postgres_dialect -- --include-ignored
```

默认归档契约1通过，原生用例按明确外部依赖忽略1项。显式已有Docker与固定镜像下，归档、原生SQL及当前WASM共3通过/0失败/0忽略。原生用例有版本与镜像身份检查，缺夹具失败且不隐式拉取。正例要求零恢复，反例允许未来grammar修复增加恢复节点，不把现有缺陷永久锁死。

新封闭schema0.1和一项协议回归验证真实原生记录、两份worker记录、同字节摘要和pending来源；拒绝假allow、假grammar资格、假独立holdout和未知字段。default/WASM workspace all-targets严格Clippy通过；修改文件格式、layering、OpenSpec strict及差异检查通过。此前1f54ebd的1452项全workspace通过属于前一源码基线，本批只新增隔离对照与回归/文档，没有借旧结果证明当前全workspace。

## 下一步要求

项目级CFQuery/SQL原生能力须明确datasource/数据库方言与版本，并保留动态模板和schema依赖的不确定性。VB.NET未缩进方法误报仍独立开放，重建所需tree-sitter-cli隔离安装等待用户确认；不通过改用户缩进或忽略所有MISSING冒充grammar修复。
