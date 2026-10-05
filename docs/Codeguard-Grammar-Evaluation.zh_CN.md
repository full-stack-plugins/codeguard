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
  --base-manifest tests/fixtures/grammar_manifests/manifest_2026_10_04.json \
  > /tmp/codeguard-grammar-current-corpus.json
```

导入器保留 CRLF、空行和最后一棵预期树，来源绑定上游文件摘要和样例名称；拒绝缺分隔符、空节点、损坏预期树及不支持的 corpus 指令。上游预期树只生成 grammar 回归标签，不提升为独立裁定。历史 [0.1 语料](../tests/fixtures/grammar_regression.json)、schema 和 186 例实际报告仍保留，历史报告读取保留原协议；当前回放须另行生成绑定当前清单的输入，不能直接执行旧身份。

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

Python 开发差分 0.3 将原始 grammar 与“解析恢复 + 独立结构规则”候选分开测量：原有 `comparison` 和 TP/FP/FN/TN 保留，新增规则身份与原始坐标、`combined_candidate_comparison` 及独立分母。截断、取消和程序变化不产生清洁候选，原生身份变化撤回两层比较；候选仍要求原生确认，历史 0.1/0.2 报告不改写，资格和交付权威不扩大。见[分层验收](../tests/acceptance/native-structure-differential.md)。

JavaScript 开发差分新增固定 Node 24.18.0 的隔离 `--check --input-type=module` 观察：清空环境、冻结 stdin、共享截止时间，调用前后核对制品及请求入口。0.4 报告明确 module 目标，只保留核对源码的行号；未知输出或工具变化保持未完成，不推断项目 CommonJS/ESM，也不代替 ESLint。Node 独立 128 MiB 制品预算不扩大其它工具的 64 MiB 上限。见[JavaScript 原生验收](../tests/acceptance/javascript-isolated-native-differential.md)。 实际18样本保留5TP/0FP/2FN/11TN：模块顶层return及重复绑定仍为漏检，资格保持0/32。


### 空语句块事实扫描（接线未完成）

Rust runtime新增 `scan_wasm_empty_blocks`，全树观察无非comment命名语句的block，保留父节点类型、字节位置和预算耗尽。它与原始ERROR/MISSING分开，不签发语言违规；合法Rust空函数也会产生事实。固定Python required_suite候选规则现已接入私有worker、显式探针及单文件lint/work sync稳定确认任务，原始恢复和结构观察使用独立版本化字段；聚合check现通过0.48反馈与通用0.7确认报告保留同一结构证据和任务身份；可信原生关闭仍缺，Python原始grammar两项漏检仍保留。详见[结构任务接线](../tests/acceptance/python-structure-lint-task.md)。见[结构事实验收](../tests/acceptance/wasm-empty-block-facts.md)。

聚合结构证据接线见[验收记录](../tests/acceptance/check-python-structure.md)，不能据此声明grammar质量认证或完整交付门禁完成。

Python隔离原生探针现于版本探测后、检查前核验请求入口与冻结字节，入口变化不启动第二次调用；检查后继续核验。实际三输入与替换/重定向反例见[原生入口连续性验收](../tests/acceptance/python-native-tool-continuity.md)。Python可信关闭仍缺稳定任务身份、首次报告与批准目标版本接线，不把开发py312探针视为项目通过。


### 历史清单与当前回放

历史语料/报告保留原清单与语料摘要，使用保存的原清单字节只读核对。当前回放仍要求当前固定清单；复用相同样本须显式生成新绑定输入并记录新语料摘要，不覆盖历史报告。只读清单核对不提供执行或发行许可，详见[边界验收](../tests/acceptance/grammar-manifest-history-binding.md)。


### 内置资产复用与源码结果缓存的边界

同一进程对固定内置grammar的成功字节/许可证核验现按语种复用，元数据也只解析一次；最多缓存当前清单列出的固定资产，未知语种不占缓存项。并发选择只完成一次核验，返回的元数据是独立副本。显式传入的WASM、许可证和资产身份仍每次核对，源码、配置、原生工具、任务历史及门禁都不在此缓存中。进程重启或更新二进制会重建缓存；这不实现跨命令源码结果缓存，也不把候选零恢复升级为clean。见[资产复用验收](../tests/acceptance/selected-grammar-asset-reuse.md)。


原生差分回放也遵循历史与当前身份分离：先按保存的旧清单核对原语料，再显式生成只变更清单摘要的当前输入，完整保留源码、标签和来源。旧输入直接执行仍拒绝；原生工具需要显式提供。Ruff 控制替身精确核对当前隔离参数，真实对照另存当前报告，不覆盖旧报告。见[验收记录](../tests/acceptance/native-corpus-current-binding.md)。


开发导入器默认严格核对当前清单；历史输入必须显式提供 `--base-manifest` 且摘要与原输入匹配，不自动寻找历史清单。输出只写 stdout，生成新的当前绑定语料，不覆盖历史文件；现有全部样本、源码与来源通过逐项回归核对，不能用原历史文件做字节 `cmp`，因为清单身份已改变。缺失或错误清单在读取追加输入前拒绝，最终输出仍接受当前生产校验。见[全工作区验收与导入器修复](../tests/acceptance/workspace-regression-importer-binding.md)。

原生开发差分现把同一取消令牌传入Zig、Erlang、Swift、Kotlin、Python、JavaScript观察器的版本和源码阶段；执行中取消保留样本与unknown，不改变grammar资格。同步摘要I/O仍无硬中断保障；见[取消验收](../tests/acceptance/native-differential-shared-cancellation.md)。

Zig、Erlang、Swift原生观察现于版本返回后、源码调用前及调用后复核原请求别名、规范入口和制品摘要；变化时停止后继调用，保持未完成。六语言别名/制品替换反例见[入口复核验收](../tests/acceptance/native-version-entry-binding.md)。核验与spawn间的完整TOCTOU和工具链闭包仍未解决。

Kotlin版本阶段结束后，原生观察器在编译前后核对私有源码副本与冻结输入；Zig版本探测异常stderr会停止AST调用。两者均保持工具/输入未完成，不生成源码违规，见[阶段输入验收](../tests/acceptance/native-version-input-continuity.md)。

## Ruby 原生对照接入统一差分入口

开发验收新增固定 Ruby 2.6.10p210 的受控 `-c -` 观察：冻结 UTF-8 stdin、清空环境、禁用 gems、共享截止时间和取消，并核验入口与制品连续性。完整一致输出才可分类；错路径、普通警告、坏位置、未知输出或运行故障保留 unknown。0.5 报告支持七种显式观察器，保留旧协议及所有未选语言。18 例实际 Ruby 对照为 5TP/0FP/0FN/13TN；BEGIN/END/require/shebang 样本验证检查不执行源码。这是开发回归证据，公开 Ruby lint、独立 holdout、可信任务关闭和发行资格仍未完成。见[验收](../tests/acceptance/ruby-isolated-native-differential.md)。
