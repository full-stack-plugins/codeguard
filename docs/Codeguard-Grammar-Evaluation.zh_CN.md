# 32 份 grammar 的统一开发评测

[English](Codeguard-Grammar-Evaluation.md)

此文对应现有 OpenSpec 的 S12.10–12.11、S14.17、S14.19。Rust 开发入口复用 `run_syntax_worker_candidate` 和 Core `evaluate_quality`，不新增第二套检查器或任务账本。它把分散的回归标签集中到固定语料，实际运行全部 32 份随仓 grammar，并形成可定位到样本的逐语言报告。

## 运行方式

在仓库根目录执行，先构建运行程序，再顺序回放，避免其它 Cargo 构建同时改写该程序：

```bash
cargo build --locked -p codeguard-cli --features wasm-precheck
cargo run --locked -p codeguard-cli --features wasm-precheck \
  --example evaluate_grammars -- \
  "$PWD/target/debug/codeguard" tests/fixtures/grammar_regression_v0_2.json 1200 \
  > /tmp/codeguard-grammar-regression.json
```

开发回放报告固定为 `incomplete`，退出 3；错参或无效输入退出 2。它是开发验收入口，不是 `codeguard lint` 的替代命令，不修改工作区任务、grammar 资格或白名单。CI 独立顺序执行并保留 JSON 步骤输出。

## 数据与执行路径

```mermaid
flowchart LR
    A[固定 358 个样本与清单摘要] --> B[启动前校验语言全集及源码摘要]
    B --> C[共享 deadline 顺序调用 Rust 隔离 worker]
    C --> D{恢复扫描完整?}
    D -->|是| E[样本分类 valid / invalid]
    D -->|否或失败| F[unknown 保留在原分母]
    E --> G[按语言、cohort 与标签分别统计]
    F --> G
    G --> H[32 语言、35 来源组与逐样本证据]
    H --> I[修复差异并收集独立原生 holdout]
```

当前固定语料位于 [grammar_regression_v0_2.json](../tests/fixtures/grammar_regression_v0_2.json)，共 358 例、32 语言、35 个语言×来源组。它完整保留历史 186 例，追加 22 例结构反例，并导入 Dart 上游 150 例。每种语言都有已选合法与非法预期；COBOL 的新增非法预期仍待裁定，不能声称全部反例均已由原生工具确认。少一语言、重复键、未知字段、错误清单/源码摘要和自行声明的批准标签在运行前拒绝。

| `cohort` | 样本数 | 标签权威 |
|---|---:|---|
| `repository_regression` | 206 | 已有仓库回归预期；本轮不执行原生 oracle |
| `upstream_grammar_regression` | 150 | Dart grammar 自带 corpus，其中 4 例预期 ERROR/MISSING；不是独立原生标签 |
| `provisional_syntax` | 2 | CFQuery、COBOL 待裁定预期，`label=pending`，不计 TP/FP/FN/TN |

Rust [build_grammar_corpus](../crates/codeguard-cli/examples/build_grammar_corpus.rs) 在开发期构建新语料：

```bash
cargo run --locked -p codeguard-cli --example build_grammar_corpus -- \
  tests/fixtures/grammar_regression.json \
  tests/fixtures/grammar_additional_regressions.json \
  > /tmp/codeguard-grammar-corpus.json
cmp /tmp/codeguard-grammar-corpus.json tests/fixtures/grammar_regression_v0_2.json
```

导入器保留 CRLF、空行和最后一棵预期树，来源绑定上游文件摘要和样例名称；拒绝缺分隔符、空节点、损坏预期树及不支持的 corpus 指令。上游预期树只生成 grammar 回归标签，不提升为独立裁定。历史 [0.1 语料](../tests/fixtures/grammar_regression.json)、schema 和 186 例实际报告仍保留，旧输入继续按旧形状输出。

## 报告含义

当前报告协议为 [grammar_regression_evaluation 0.2](../schemas/grammar-regression-evaluation-v0.2.schema.json)，语料协议为 [grammar_regression 0.2](../schemas/grammar-regression-corpus-v0.2.schema.json)。结果逐语言及其 `cohorts` 列出：

| 字段 | 含义 |
|---|---|
| `selected_valid_count` / `selected_invalid_count` | 已选合法/非法预期数，含 pending，不代表独立确认 |
| `sample_count` | 原语料分母，失败与未知样本仍在其中 |
| `decidable_count` / `unknown_count` | 恢复扫描可判定与不可判定数量，不受临时标签影响 |
| `pending_label_count` | 未裁定标签数量，不与未知解析数量相加当作互斥分组 |
| `evaluated_count` | 有回归标签且可判定的样本数量 |
| `tp` / `fp` / `fn` / `tn` | 样本级“是否存在语法异常”的混淆计数，不是精确规则实例召回率 |
| `precision` / `recall` | 来源组内相对于回归标签计算；混合来源的语言汇总固定 null，零分母也为 null |
| `metric_aggregation` | 单组为 `single_cohort`；多组为 `not_pooled`，计数可加但不混算精度 |
| `fixture_false_clean_count` | 非法回归样本被完整解析为无恢复节点；不是项目门禁已经错误放行的次数 |
| `completion_rate` | 可判定解析数 / 全部样本数，不代表检查覆盖或源码正确率 |
| `performance` | 顺序冷 worker 的墙钟时间 p50/p95；不证明热启动、内存或同覆盖性能达标 |

每条样本保留 ID、来源、源码摘要、标签、实际分类、恢复数、未知原因和耗时；不在报告回显源码。程序启动前后摘要不一致时，全部分类撤回为 unknown。取消和预算耗尽不重置 deadline，尚未运行样本保留 unknown；预算控制进程执行与循环准入，不声称文件哈希 I/O 已有硬截止保障。

根报告固定 `authority=repository_regression_only`、`native_oracle_executed=false`、`independent_holdout=false`、`grammar_qualified_count=0`、`delivery_decision=not_evaluated`。统计门槛保持 Wilson 下界 0.98、零漏报；当前最低预测数量 200 仅用于保守开发计算，`approval=not_verified`。即使回归分类一致，也没有原生标签、版本/方言、真实宿主、热启动/内存和发布批准的自动升级。

本轮实际回放与剩余差异见 [验收记录](../tests/acceptance/grammar-regression-evaluation.md)。独立 holdout、真实原生工具回放和漏洞库评测必须继续单独实现；这些父任务保持未完成。

## 分来源报告片段

以下为协议形状说明，省略其它必需字段，不是完整运行证据：

```json
{
  "schema_version": "0.2.0",
  "cohort_policy": "separate_sources_no_pooled_precision",
  "language_count": 32,
  "cohort_count": 35,
  "languages": [{
    "language": "dart",
    "sample_count": 152,
    "metric_aggregation": "not_pooled",
    "precision": null,
    "recall": null,
    "cohorts": [
      {"cohort": "repository_regression", "sample_count": 2},
      {"cohort": "upstream_grammar_regression", "sample_count": 150}
    ]
  }]
}
```

Dart 的 150 例不能盖过其它语言的证据缺口，也不能与仓库两例合并出一个看似充分的 Wilson 区间。所有来源组均未取得独立 holdout 批准，资格仍为零。358 例实际回放和逐来源差异另见 [0.2 验收记录](../tests/acceptance/grammar-cohort-regression-evaluation.md)。

## 显式原生差分开发回放

新增 Rust 示例 `evaluate_native_grammars`：从同一32语言固定语料选择已显式提供 Zig0.16.0、OTP28、Apple Swift6.4 与 kotlinc-jvm2.4.10 的样本，逐例复用原生适配器和既有WASM worker。库存仍保留32语言；未选工具、无适配器、原生未完成及WASM隐藏恢复不被补成通过。只在双方语法可判定时统计TP/FP/FN/TN；Kotlin混合上下文阻塞保留已定位语法正向证据，但执行仍未完成。工具/入口或程序变化撤回对应分类。回放固定 incomplete/资格0，不创建任务或批准白名单。原生适配器复用、回归语料和入口制品摘要不证明独立holdout或完整工具链身份。使用方式和实际证据见[原生差分验收](../tests/acceptance/native-grammar-differential.md)。


### Python 隔离原生语法对照

开发差分新增显式 Ruff0.16.8 / Python3.12 观察，固定 stdin、`--isolated --select E9 --ignore-noqa --no-cache`，不读取项目配置或执行用户源码。只消费合法定位的 `invalid-syntax`；普通 F401、坏路径/位置、版本变化或矛盾报告不判源码语法失败。新增0.2报告，0.1历史字节保持不变；扩展开发工具选择不扩大可信任务关闭权限。18例实际对照发现缺少函数体、错误缩进2个WASM漏检，仍为未验收候选；见[Python原生差分验收](../tests/acceptance/python-native-grammar-differential.md)。


### 空语句块事实扫描（接线未完成）

Rust runtime新增 `scan_wasm_empty_blocks`，全树观察无非comment命名语句的block，保留父节点类型、字节位置和预算耗尽。它与原始ERROR/MISSING分开，不签发语言违规；合法Rust空函数也会产生事实。固定Python required_suite候选规则现已接入私有worker、显式探针及单文件lint/work sync稳定确认任务，原始恢复和结构观察使用独立版本化字段；聚合check现通过0.48反馈与通用0.7确认报告保留同一结构证据和任务身份；可信原生关闭仍缺，Python原始grammar两项漏检仍保留。详见[结构任务接线](../tests/acceptance/python-structure-lint-task.md)。见[结构事实验收](../tests/acceptance/wasm-empty-block-facts.md)。

聚合结构证据接线见[验收记录](../tests/acceptance/check-python-structure.md)，不能据此声明grammar质量认证或完整交付门禁完成。

Python隔离原生探针现于版本探测后、检查前核验请求入口与冻结字节，入口变化不启动第二次调用；检查后继续核验。实际三输入与替换/重定向反例见[原生入口连续性验收](../tests/acceptance/python-native-tool-continuity.md)。Python可信关闭仍缺稳定任务身份、首次报告与批准目标版本接线，不把开发py312探针视为项目通过。


### 历史清单与当前回放

历史语料/报告保留原清单与语料摘要，使用保存的原清单字节只读核对。当前回放仍要求当前固定清单；复用相同样本须显式生成新绑定输入并记录新语料摘要，不覆盖历史报告。只读清单核对不提供执行或发行许可，详见[边界验收](../tests/acceptance/grammar-manifest-history-binding.md)。
