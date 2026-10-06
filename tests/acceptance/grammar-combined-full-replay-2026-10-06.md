# 32 种 WASM 的原始解析器与组合结构规则全量测量

延续 introduce-rust-codeguard-cli 的12.11/14.17/14.19，使用现有358例、32语言、35来源组；源码、标签、来源不变，输入显式绑定当前清单。新增开发入口 `replay_corpus_with_structures`，与项目扫描相同的JavaScript/Erlang结构worker入口，其它语言沿用已有worker。每个样例只启动一次worker，原始恢复与结构事实来自相同冻结字节。

```mermaid
flowchart LR
    A[当前固定语料与程序] --> B[隔离worker]
    B --> C[原始ERROR与MISSING]
    B --> D[独立结构规则及坐标]
    C --> E[保留原始分组报告]
    C --> F[组合样例分类]
    D --> F
    F --> G[分语言与来源的混淆计数]
    E --> H[并列报告 不授予语言资格]
    G --> H
```

协议 `grammar_structure_evaluation 0.1` 包含完整 `raw_report`、逐例 `combined_cases` 和 `combined_cohorts`。逐例保留源码/grammar摘要和固定规则原坐标；分组列出TP/FP/FN/TN、unknown与pending分母，不汇总跨来源精度。结构候选不冒充原始恢复或原生诊断。原始扫描截断、取消、过期及程序变化令组合结果也为unknown；待裁定标签不进入混淆计数。程序变更撤回两层分类，不能用候选数量伪造通过。

先失败测试确认开发入口缺失；补齐入口后原回放15项和新取消/过期目标通过，原协议行为不改。完整回放显式运行：

```bash
CODEGUARD_COMBINED_REPORT_DIR=/private/tmp/cg-combined-full-2026-10-06 \
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test grammar_structure_evaluation \
  full_combined_replay_keeps_raw_erlang_gaps_and_measures_structure_recovery \
  -- --ignored --exact --nocapture
```

此为固定回归语料上的规则效果测量，原生oracle未执行、独立holdout缺失、资格0/32，不能代替语言正式精度验收、宿主修复闭环、可信关闭或发布门禁。历史原始十个Erlang漏检和全部旧报告不覆盖。

## 当前实际结果

显式全量目标1 passed / 0 failed / 0 ignored，274.07秒，全部358例实际attempted，程序前后身份稳定。程序SHA-256：`0ad9b06e41b3b2a7c57968e8b777881770e7dda30fb4a7e33ae42fc78a8e1999`。

|测量层|TP|FP|FN|TN|unknown|pending|
|---|---:|---:|---:|---:|---:|---:|
|原始解析器|73|1|10|269|3|2|
|组合候选|83|1|0|269|3|2|

表中仅汇总计数便于盘点，不提供跨来源准确率；各语言/来源的分母在原始报告中分开保存。十项Erlang原始漏检均得到固定form终止符候选，没有增加回归集中的误报。剩余六项：VB.NET合法未缩进方法疑似误报；Kotlin缺参数类型、合法object与Swift缺参数类型三项unknown；CFQuery和COBOL两项pending。COBOL组合观察为invalid也不能自行把pending标签改成已裁定。

[实际报告](evidence/grammar-combined-report-2026-10-06.json)和[输入](evidence/grammar-combined-corpus-2026-10-06.json)分别保留字节绑定。实际报告通过新schema，六种伪造资格/原生oracle/holdout/交付/规则摘要/语言身份反例拒绝；373份schema元定义通过。历史全量报告保持原样。

受影响四目标21 passed / 0 failed / 4条件忽略（完整目标已另行显式执行），新增归档校验后新目标3 passed / 0 failed / 1条件忽略，与前者重叠不相加。归档回归逐例核对来源、原始/组合分类、当前固定规则及原坐标，并独立重算每组混淆计数和unknown/pending分母。改变程序的目标验证两层分类撤回。WASM全目标严格Clippy、分层、OpenSpec strict和diff检查通过；远端CI独立核验，不能借用本地结果。

最终默认与WASM全目标Clippy -D warnings均通过；所改Rust文件格式、分层、OpenSpec strict和diff检查通过。完整358例仅运行新组合入口一次，原始分类来自该轮相同worker，不冒充另一轮独立原生验收。
