# 全 32 grammar 开发回放验收

日期：2026-10-04。对应现有 OpenSpec S12.10–12.11、S14.17、S14.19。没有替换规格事实源、改变验收门槛或提升任何 grammar 资格。

## 实际回放

Rust `evaluate_grammars` 开发入口复用现有隔离 worker 与 Core 统计计算。运行前冻结全部 32 语言、186 例回归语料，包含未修复 Erlang 扩展终止符标签。实际报告固定程序前后摘要一致，186 项均启动、32 份 grammar 实际加载；退出 3，`fixture_outcome=fails_fixture_threshold`，不是语法验收通过。

原始脱敏 JSON：[grammar-regression-2026-10-04.json](evidence/grammar-regression-2026-10-04.json)，SHA-256 `11bc1989d9401f395a717fa348496e4af0a00629ad379df94491755a453a71cc`。这是实际 stdout 原字节，包含清单、源码及运行程序身份；不手工改写计数或耗时。

| 范围 | 实际观察 |
|---|---|
| 全部样本 | 186；可判定 183；未知 3 |
| 相对回归标签的计算范围 | 182；TP 48、FP 1、FN 10、TN 123 |
| Erlang | 38 例含路由控制；10 个终止符漏检，6 TP、22 TN |
| VB.NET | 2 例；未缩进合法方法相对回归标签 1 FP |
| Kotlin | 14 例；2 unknown：object / missing_parameter_type |
| Swift | 14 例；1 unknown：bad_param |
| CFQuery | 2 例；缺 SQL 选择列表的临时预期单列 pending，不计 FN |
| 其它覆盖不足 | 本轮统一语料中 20 种语言只有 1 个合法路由控制样本，不能给出可估计 precision |

已确认的原生工具差分仍沿用各自独立验收记录；本轮不运行原生 oracle，不能将上述回归混淆计数称为生产误报率/漏报率。每样本只有一个“有无语法异常”事件，不是规则实例召回。待裁定标签与未知解析不是互斥集合。

## 测试及报告校验

目标测试最初因缺 `grammar_evaluation` 模块 RED，日志 `/tmp/codeguard-grammar-evaluation-red.log`；实现后默认构建 3 passed/0 failed，WASM 特性 5 passed/0 failed/1 ignored。其中正确校验全部语言与源码身份，拒绝重复键、少语言、未知标签、重复 ID 和字节变化；隐藏错误保留未知；取消与到期仍输出全部 186 项；运行程序发生变化撤回全部分类。

真实入口输出通过新回放与语料两份封闭 schema；9 类伪造批准/原生回放/holdout/语言数量/交付通过均拒绝。逐语言 n=decidable+unknown、已评样本=TP+FP+FN+TN、32 语言全集及语料/清单 SHA 均复核一致。schema 只校验形状，跨字段等式由独立实际数据校验完成。

`cargo clippy -p codeguard-cli --all-targets --features wasm-precheck --locked --offline -- -D warnings` 通过。完整显式回放测试、受影响 Core 测试、最终静态检查与新提交远端 CI 的终态分别追加；不借上一提交测试称本轮全工作区重跑。

## 剩余工作

仍需修复已知 grammar 差异、将已有独立测试语料纳入统一回放并补 20 种语言的非法与版本/方言样本、独立原生标签、批准 holdout、热启动/内存与同覆盖性能、多平台和真实宿主验收。Rust 1.85 尚未安装，因此 14.2 的 MSRV 也不能按当前 stable 构建勾选完成。Erlang 重建工具授权仍待用户回复，没有下载 CLI。

本轮没有改 grammar 字节、npm 制品或插件锁，保留未提交的 Erlang RED 测试草稿。12.10/12.11/14.17/14.19 以及总体目标保持未完成；测试回放基础设施通过不等于 grammar 验收通过。

范围说明：Dart 的 150 例上游 corpus 和其它语言的直接加载正反例仍有独立测试与验收；本次统一入口尚未收录它们。“20 种只有一个合法样例”仅指此固定回放语料，不否定其它测试存在，也不把未并入的结果混计到本报告。

最终语料回放在 Regression 类型隔离加固后重跑；上面归档已替换为该终态实际 JSON 整件。初始开发入口报告及回放日志保留于 `/tmp/codeguard-grammar-evaluation-actual.json` 和 `/tmp/codeguard-grammar-evaluation-full-test.log`，两阶段不混计测试数量。

最终终态：Core 全目标 9 组 56 passed/0 failed/0 ignored；相关默认 CLI 四组 17 passed/0 failed/0 ignored；最终归档后的 WASM 目标 6 passed/0 failed/1 ignored，完整 186 例回放另显式 1 passed/0 failed/0 ignored。Core 9 项计算测试包含于上述全目标，不重复累加。workspace 全目标 WASM Clippy -D warnings、fmt、分层、OpenSpec strict 和 188 份 schema 元定义通过；上一源码远端 CI 已成功，本轮新提交仍须独立远端验证。完整回放、正常目标与历史阶段有重叠，不能相加声称全量测试规模。
