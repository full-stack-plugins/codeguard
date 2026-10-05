# 32 种 WASM 当前版本全量回放与缺陷清单

日期：2026-10-06。构建源码：`9208b438fae07c42c8e1fced1f4bb269ebb27c92`。对应现有 OpenSpec S12.11 / S14.17 / S14.19；父任务保持未完成。

当前 Rust CLI 全部 358 个固定样例实际执行，覆盖 32 种语言、35 个来源组。实际测试 1 passed / 0 failed / 0 ignored，317.87 秒；程序身份前后稳定，全部样例 attempted=true。测试通过只证明回放完成，不代表 grammar 精度通过。

逐例报告：[当前实际报告](evidence/grammar-current-manifest-2026-10-06.json)；输入：[当前字节绑定语料](evidence/grammar-current-corpus-2026-10-06.json)。历史报告保留。仅显式重绑定清单，源码、标签和来源没有重新裁定。归档测试逐样例核对身份及分来源分母。

程序 SHA-256：`d7d973067c02988e3edf8cd7f3b5bb2132f027a0d1e0bb0aa0a4e02234f962bb`。
清单 SHA-256：`9fe2fa77181aedd74755a2ac5b8fe694b2d4f2d33f03518d814476a911b2a610`。
语料 SHA-256：`166fc8c2630632f956ec74a0014c55164d68d2d139e28d0d790b6451e8cb11dd`。

## 必须继续处理的具体差异

| 语言 | 样例 | 当前观察 | 下一步 |
|---|---|---|---|
| cfquery | `cfquery-missing_select_list` | 标签待裁定；零恢复不能据此判通过 | 取得适用原生 SQL 方言及独立标签 |
| erlang | `erlang-final_semicolon` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_middle_period` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_period` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_period_after_dot_character` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_period_after_float` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_period_after_string` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_period_comment` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_period_eof` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-missing_period_unicode_crlf` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| erlang | `erlang-multi_clause_final_semicolon` | 回归标签漏检 | 修复 Erlang form 终止符识别，复跑全部合法反例 |
| kotlin | `kotlin-missing_parameter_type` | 未知；恢复扫描不完整 | 检查扫描截断、隐藏恢复与节点定位；保留 unknown |
| kotlin | `kotlin-object` | 未知；恢复扫描不完整 | 检查扫描截断、隐藏恢复与节点定位；保留 unknown |
| swift | `swift-bad_param` | 未知；恢复扫描不完整 | 检查扫描截断、隐藏恢复与节点定位；保留 unknown |
| vbnet | `vbnet-unindented_method` | 回归标签误报 | 修复未缩进合法方法识别，取得 VB.NET 原生对照 |
| cobol | `cobol-structural_negative` | 标签待裁定；零恢复不能据此判通过 | 取得适用原生 SQL 方言及独立标签 |

Erlang 十项差异已有先前 OTP 28 原生对照支持，见[原生差分记录](native-grammar-differential.md)。本轮没有重跑原生工具，不将历史原生身份冒充当前批次证据。VB.NET 回归标签仍需原生编译器复核；CFQuery pending 不计入已裁定漏报；三项未知不计入通过。

## 执行与验收边界

```bash
cargo test --locked -p codeguard-cli --features wasm-precheck --test grammar_evaluation replay_expanded_cohorts_and_archive_current_evidence -- --ignored --exact --nocapture --test-threads=1
```

已验收 grammar 数仍为 0，independent_holdout=false、native_oracle_executed=false、delivery_decision=not_evaluated。Dart 上游来源和其它回归来源分别计算，不汇总出一个全语言准确率。全部逐样例 worker 耗时相加为 317.245197 秒，仅是当前机器串行冷启动壁钟观察，不是暖启动、内存、并发或宿主编辑性能验收。

归档测试和 JSON Schema 通过不能消除上表缺陷。生产宿主、普通任务自动关闭契约调整、原生检测全矩阵及发行门禁继续开放。未改变规则、白名单、关闭权限或 grammar 字节。

## 归档与规格验证

新增逐样例身份校验后，grammar_evaluation 普通组实际 15 passed / 0 failed / 2 ignored（两项完整回放默认忽略，本轮 358 例已另行显式执行）。实际报告与输入分别通过 0.2 JSON Schema；OpenSpec strict、crate 分层、所改文件 rustfmt 和 git diff --check 通过。没有执行未提交的 Erlang 对照草稿；远端 CI 状态独立核验，不以本地通过代替。
