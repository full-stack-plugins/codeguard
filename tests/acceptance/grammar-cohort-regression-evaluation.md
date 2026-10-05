# 32 grammar 分来源开发回放验收（0.2）

日期：2026-10-04。对应现有 OpenSpec S12.11、S14.17、S14.19 及 syntax-precheck 的上游 corpus 导入场景。规格门槛、原生优先义务及语言资格没有改变；父任务保持未完成。

## 实际回放与来源

本轮 Rust 导入器保留历史 186 例、增加 22 例结构反例、导入 Dart 上游 150 例，生成 358 例固定语料。全部 32 grammar、358 worker 均实际运行，程序前后摘要一致；完整测试退出 0 只证明回放和分组不变量，原始报告仍为 incomplete、fails_fixture_threshold、交付未评估、资格零。运行耗时 395.12 秒，不是热启动或产品性能验收。

原始 stdout JSON：[grammar-cohorts-2026-10-04.json](evidence/grammar-cohorts-2026-10-04.json)。只从完整回放日志提取 JSON 行并保留换行，没有重写字段、计数或耗时。

- 原始报告 SHA-256：`5e6e414f5c41d6fde89ee4575ce6b56c321d834c57f218ae864486fac06fff7e`。
- 语料 SHA-256：`264819368321981e548f2467369c90469b3406a4df1b7032de7e68311db7a079`。
- 回放程序 SHA-256：`8eee891a54f2e7470be4add068cda34ead4e47b77bced3dc9aefcff6aae95634`。

| 来源组 | 样本数 | 解释 |
|---|---:|---|
| repository_regression | 206 | 已有窄范围回归和新增结构反例，本轮不执行原生 oracle |
| upstream_grammar_regression | 150 | Dart grammar 的原预期树，4 例 ERROR/MISSING；不是独立编译器裁定 |
| provisional_syntax | 2 | CFQuery 缺 SQL 选择列表、COBOL 缺程序名仍 pending，不计混淆数 |

全部语言都有已选合法/非法预期，但 COBOL 非法预期待裁定。32 语言对应 35 个语言×来源组；已选合法 271、非法 87。每个组有独立 precision/recall；Dart、CFQuery、COBOL 多来源汇总只加计数，precision/recall=null，不能把不同标签权威混成一个高置信度区间。

## 实际差异与分母

| 项目 | 实际结果 |
|---|---|
| 全范围 | 358；可判定 355、未知 3 |
| 有回归标签且可判定 | 353；分组混淆计数合计 TP 73、FP 1、FN 10、TN 269；不据此计算全体混合精度 |
| Dart 上游组 | 150；TP 4、TN 146，FP/FN/unknown 均 0，仅相对 grammar 自带标签 |
| Dart 仓库组 | 2；TP 1、TN 1，独立于上游组 |
| Erlang | 原有 10 个终止符漏检仍未修复 |
| VB.NET | 原有未缩进合法方法误报 1 例仍未修复 |
| Kotlin / Swift | 2 / 1 unknown，隐藏缺失 token 未伪造可定位证据 |
| CFQuery / COBOL | 各 1 pending；不把预期未裁定的结果当作确认漏报 |

旧 186 例 JSON、0.1 语料和两份 0.1 schema 原件保持逐字节不变，新证据不覆盖旧结果。独立原生对照记录仍分别保留，不与本轮 grammar 自带标签混计。

## 实施与验证

Rust corpus 解析保留 CRLF、空行、最后一棵预期树，绑定上游文件摘要与样例名称。空节点和缺分隔符吞入下一样例的反例先 RED 再 GREEN；被引号包住的 ERROR/MISSING 文本及更长的节点名不会伪造负例。版本化严格输入拒绝现代语料缺 cohort、旧语料带 cohort、自批 holdout 及来源组版本冲突。

测试/校验终态追加于下；普通测试不默认重跑完整语料，显式 358 例实际回放已独立执行，CI 则顺序运行当前分来源回放。测试基础设施通过不等于 grammar 质量通过。语料生成的全字节复现测试包含所有源字节、来源摘要和标签。

## 剩余验收

需修复已知 Erlang/VB.NET 差异、取得 pending 标签裁定，补语言版本/方言与独立原生标签、批准 holdout、暖启动/内存/并发性能、MSRV、跨平台、真实宿主和完整修复门禁。大多数语言只有很少的正反例，Dart 150 例不能为其它语言补足证据。当前不改 grammar 字节、插件锁或 npm 发行；保留未提交的 Erlang 37 例 RED 草稿。总体 OpenSpec 目标保持 active。

## 最终验证结果

- 默认 CLI 协议/归档测试：7 passed；Rust 语料全字节复现 example：1 passed；adapter corpus：4 passed。各组均 0 failed/0 ignored。
- WASM 普通协议目标：10 passed/0 failed/2 ignored；358 例完整回放另显式 1 passed/0 failed/0 ignored。旧 186 例完整回放本轮不重复执行；重叠测试不合计。
- 190 份 schema 元定义、0.2 语料/实际报告、跨字段分母和摘要通过；16 类伪造/矛盾变体拒绝，包括混合精度、自批原生/资格/holdout、丢语言、重复来源组及程序变化却保留有效分类。
- workspace 全目标 WASM Clippy -D warnings、fmt、crate 分层、OpenSpec strict、修改文档链接通过。旧 0.1 语料/schema/报告及清单五份文件逐字节未变。
- 上一提交 c2b8e27 的 Linux CI 已成功；本轮新源码的远端结果独立记录，没有宣称本轮本地完整 workspace 重跑。

日志身份（本轮具体执行，不是历史全工作区）：

- `/tmp/codeguard-grammar-corpus-malformed-red.log`：SHA-256 `5a973acf19cd9151fa15be81ba3b6634dd3dd768c8ac8de2d0e192286f818e41`。
- `/tmp/codeguard-grammar-corpus-malformed-green.log`：SHA-256 `ebf643c4425d5a77118fcd4f0f501b85056dc4ae990cefef41946bc9a8e15ef1`。
- `/tmp/codeguard-grammar-cohorts-counts-red.log`：SHA-256 `fa333ded86bf77ce4cccdb1f88b7c2790cd46fecaaef9179059d58a3e32a493f`。
- `/tmp/codeguard-grammar-cohorts-default-final.log`：SHA-256 `48c7b7ad30fb3c3d6a8b5acb87dc892590fb12104d9d39a5596a11673085ed58`。
- `/tmp/codeguard-grammar-cohorts-adapters-final.log`：SHA-256 `96a63f0a2c853910a7cbf6b7e92d3bef972d94b05bb8219f1c0ff0fc759a1085`。
- `/tmp/codeguard-grammar-cohorts-feature-final.log`：SHA-256 `30940831212bb856232c9da1ac712c216e02a7ae8a22be21222600e08e44995c`。
- `/tmp/codeguard-grammar-cohorts-full.log`：SHA-256 `bb1ee91a5b50ee01e859c5302b0568d9c6af9652def67df15e65cacf5eb906ba`。
- `/tmp/codeguard-grammar-cohorts-clippy-final.log`：SHA-256 `32cd1cc2307521e54e48ac6514227b6cbc5e64e71091ebc4feebbcd91c628f04`。
- `/tmp/codeguard-grammar-cohorts-schema-final.log`：SHA-256 `6076751be24d233d39bf1177716734b14beca1d80a29c73d90d18b720145e49b`。

补充依赖方向验收：`crate_boundaries` 当前 7 passed/0 failed/0 ignored；日志 `/tmp/codeguard-grammar-cohorts-boundaries-final.log` SHA-256 `078f67a27d10f7e9b5d729940bebd7c897fcb94a8bde6b7bf28c8c3a5f5c351d`。新增 corpus 解析/样本对象仅在 adapters，未引入 runtime 依赖。
