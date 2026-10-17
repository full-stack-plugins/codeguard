# CodeGuard 生产验收真实状态账本（只读审计）

日期：2026-10-07。审计性质：**只读审计**，未修改任何 Rust 源码。审计基线：本仓库 `codeguard` 分支 `feat/rust-hook-prompt-guidance`，HEAD `2b68317`（工作树干净）；相邻插件仓 `codeguard-plugin` 本地 HEAD `dec5f9d`。受保护文件 `crates/codeguard-cli/tests/erlang_native_differential.rs` SHA-256 实测仍为 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`，与基线一致，未被触碰。

本审计不授予任何生产资格；结论只描述"账本与事实是否一致"。

## 0. 审计方法与实际执行的检查

- 用 python3 解析 `openspec/changes/introduce-rust-codeguard-cli/tasks.md`（2162 行）、`rulepacks/production_acceptance_plan_v1.json`（9111 行）、`grammars/manifest.json`、`grammars/codegraph-coverage.json`，统计全部来自脚本重算，不是转录。
- 逐项核对 66 个已勾选任务引用的证据文件是否存在、内容是否支持声称。
- 远端核验（`gh` / `npm view`，2026-10-07 实际执行）：codeguard 仓库 CI run `37243591502`、`37414921955`、`37423582241`；`full-stack-plugins/codeguard-plugin` 的 tag/release/合并提交；npm 注册表 `@partme.ai/codeguard`。
- 本地实测一条验证命令（允许范围内、单 crate、offline/locked）：

```bash
CARGO_PROFILE_TEST_DEBUG=0 cargo test --offline --locked -p codeguard-cli --features wasm-precheck --test production_acceptance_plan
# 结果：6 passed / 0 failed / 0 ignored，退出 0
```

其中 `all_registered_languages_keep_four_blocked_core_obligations` 通过，即**当前代码侧强制**全部注册语言四核心保持 blocked——与计划 JSON 一致，代码与账本没有漂移。
未运行：全工作区测试/clippy（受保护 Erlang 测试约束）、358 例全量回放、任何真实原生工具条件测试。上述均不在本审计执行范围内，本文不引用它们为通过。

## 1. 任务账本对账

### 1.1 总量核对

`tasks.md` 复选框实测：`[x]` 66、`[ ]` 288，共 354。**"66 已完成 / 288 未完成" 属实**。

### 1.2 已勾选 66 项的分类

按任务书给定的回填特征（任务文字含"仅验收基线所列测试契约；剩余：…"）：

**A 类：仅契约切片 / 回填记账 —— 23 项**（全部位于"2026-09-28 已实现切片补录"一节，tasks.md 行 1004–1026）：

2.1.1、2.2.1、4.7.1、4.7.2、4.9.1、5.7.1、5.8.1、5.2.1、6.2.1、6.3.1、6.3.2、7.1.1、7.1.2、7.2.1、7.3.1、8.2.1、9.1.1、9.4.1、2.3.1、9.8.1、9.10.1、9.7.1、2.7.1

这 23 项的证据集中在 `openspec/changes/introduce-rust-codeguard-cli/implementation-baseline.md`：2026-09-28 轮 28 个 CLI 集成测试目标 266 通过 / 46 忽略，**大多使用受控工具输出（fixture），真实环境测试未重新执行**。该文件自己声明"不能推导整项语言支持、正式交付或真实宿主验收通过"。每一项均逐条列出对应测试文件与"仍未完成"列，回填记账边界诚实。

**B 类：真实完成（自身验收标准完整满足、独立交付）—— 18 项**：

- 主线编号任务 17 项：1.1、1.2、1.3、1.4、1.5、2.2、2.6、3.1、5.7、5.8、8.1、8.16、8.145、8.146、8.147、9.5.1、9.7.2
- 14.2（Rust WASM 加载器）：有完成核验、远端 CI 成功记录（见 §3）

注意这 18 项中多数完成的是**基线/契约/档案类**任务（工程基线、schema、协议映射、静态候选档案、planned-gap 防伪），其任务文本本身就声明"不代表原生实现完成"。它们是真实完成的，但**不构成任何语言生产能力的完成**。

**C 类：有真实证据的局部实现切片（父任务保持未完成）—— 25 项**。不属于 A 类的"仅契约"，也不是父任务量级的完成：

- npm/插件发布子任务 5 项：13.4.1、13.4.2、13.4.2a、13.4.3、13.4.4（发布事实可远端核验，见 §3；每项均声明"不代表完整 S13"）
- 2026-10-06 日期戳切片 20 项（Java comments/Javadoc 工作台/复检、Maven Javadoc、Java 差分分类、CLI 别名、默认回归兼容；tasks.md 行 1844–1908），证据如 `java-comments-unified-entry.md`、`default-workspace-comments-dispatch.md`

**审计结论**：66 个勾选里，真正"完整量级交付"的只有 B 类 18 项（且多为基线性质）；23 项是明确的契约切片记账；25 项是局部切片。没有任何一个勾选声称"某语言×四核心生产能力完成"——与能力矩阵（§2）零资格一致，账本没有虚报完成度。

## 2. 能力矩阵现状（rulepacks/production_acceptance_plan_v1.json）

JSON 头部自我声明：`qualification: not_granted`、`authority: repository_plan_only`、`platform_scope: candidate_targets_not_advertised_support`；平台目标 5 个（macos_arm64、macos_x86_64、linux_x86_64、linux_aarch64、windows_x86_64）。

### 2.1 57 语言 × 四核心

- 四核心 = `syntax / documentation / conventions / vulnerabilities`，57 语言各有全部四项（228 个能力单元）。
- **228 个单元 qualification 全部为 `blocked`，0 个合格。**
- 该状态不只是文档：`production_acceptance_plan.rs` 的 `all_registered_languages_keep_four_blocked_core_obligations` 测试（本次实测通过）在代码侧锁定它。
- `version_scope.qualification` 57 语言全部 `unqualified`；`legacy_status` 为 stable 的 54 语言每条都带"不按旧 stable 推断支持"类 blocker（见 §5 风险 1）。

### 2.2 364 条构建生态路径

四核心合计 364 条 build_paths（77 种 ecosystem）。`implementation_status` 实测分布：

| 状态 | 数量 | 占比 |
|---|---:|---:|
| partial | 46 | 12.6% |
| wasm_candidate_only | 26 | 7.1% |
| not_integrated | 292 | 80.2% |
| （完整集成/合格） | **0** | 0% |

不存在任何"integrated/qualified"取值——**没有任何一条路径具备生产资格**。语言分布：

- 有 ≥1 条 partial 的语言 13 个：rust（4/4 全 partial，唯一无 not_integrated 的语言）、python（13 partial/3 not）、java（7/1）、typescript（3/5）、shell（4/4 partial + 4 not）、c、cpp、go、erlang、kotlin、ruby、swift、zig（各 1–2 partial）
- 仅 wasm_candidate_only 的语言 16 个：arkts、cfml、cobol、csharp、dart、lua、luau、nix、objc、pascal、php、r、scala、solidity、terraform、vbnet
- 其余 28 个语言全部路径 not_integrated

### 2.3 grammar 资格 0/32：属实

- 计划 JSON 中 `grammar_candidates` 共 **32 条、覆盖 28 种语言**（typescript 含 typescript/javascript/tsx 三份，cfml 含 cfml/cfquery/cfscript 三份）；32 条全部挂在 `syntax` 下，qualification 全部 `blocked`。
- `grammars/manifest.json`：32 份资产 `release_status` 全部 `candidate_unvalidated`；`codeguard_runtime_validation` 全部仅 `rust_loader_smoke_passed`；32 份的 `language_versions` **全部为空**。
- `tests/acceptance/grammar-current-full-replay-2026-10-07.md`：358 例全量回放（TP 73 / FP 1 / FN 10 / TN 269 / unknown 3 / pending 2），`grammar_qualified_count=0`；`codegraph-grammar-coverage.md`：`candidate_count=32、released_count=0`。
- 结论：**"grammar 资格 0/32" 属实**，且四处独立来源（计划 JSON、manifest、回放证据、grammar status 输出）一致。

## 3. 声称 vs 证据核对（抽查 23 项，含远端核验）

抽查覆盖 A/B/C 三类。结论先行：**未发现虚报完成**；发现 1 处记录损坏（哈希抄写错误）、1 处计划账本滞后于 HEAD、若干证据缺少日期。逐项如下。

### 3.1 核对通过（声称与证据/远端一致）

| 任务 | 声称 | 核对结果 |
|---|---|---|
| 1.1 | 源码审计 + 57 语言审计证据 | 两文件均存在；`legacy-source-migration-audit.md` 记 21 个变化文件、38 项旧 Python 回归、证据 JSON 路径，与任务文字一致 |
| 1.2/1.3/1.4/1.5 | engineering-baseline / crate_boundaries.rs / corpus-baseline / capability-inventory | 四个证据文件与两个测试文件均在；1.5 的"54 stable/3 planned"与计划 JSON `legacy_status` 分布（54/3）吻合 |
| 2.2 | run-report schema + 契约测试 | `schemas/run-report.schema.json`（现 1.4）、`tests/run_report_contract.rs` 存在；任务文字描述的是 1.2 时期完成态，1.3/1.4 属未勾选 2.10 的后续增量，历史分层无矛盾 |
| 2.6 | 164 项旧插件回归 + 7 项 MCP | `legacy-v1-protocol-map.md`（2026-09-28）记载与任务文字一致，且诚实记录了旧运行时 Python 3.9 兼容缺口 |
| 3.1 | process_contract.rs + ruff_probe_contract.rs | 两测试文件存在 |
| 5.7/5.8 | discovery/capabilities 证据 | `discovery-baseline.md`、`capability-inventory.md`、schema 与三个测试文件均存在 |
| 8.1/8.16 | go/ruby 六类别静态档案 | `go_static_candidate_v1.json` 实测六类别全部 `applicable/gap`，与 `go-candidate-baseline.md` 表格逐项一致；ruby 侧文件亦在 |
| 8.145–8.147 | planned gap 防伪测试 | `planned_language_gaps.rs` + `planned-language-gaps.md` 存在 |
| 9.5.1/9.7.2 | 投影恢复 / 独立源码任务 | `task-projection-recovery.md`、`next-independent-source-work.md`（均 2026-10-05，Ruff 0.16.8）与任务文字样本一致 |
| 14.2 | WASM 加载器完成；提交 `8ca3bb1`；CI 37243591502 MSRV 成功 | `8ca3bb1` 是 HEAD 祖先（git 实测）；`gh run view 37243591502` → **success, headSha 8ca3bb1c…**，与证据完全一致 |
| 13.4.2a | npm 0.1.2 发布、注册表完整性 | `npm view @partme.ai/codeguard versions` 含 0.1.0–0.1.4（0.1.2 在列）；远端事实成立 |
| 13.4.3 | plugin 0.17.0，合并提交 `de0936a…`，tag/GitHub Release | 插件仓本地 `de0936a` 存在（"Add locked Rust runtime candidate (#85)"）；`gh release list` 见 v0.17.0（2026-09-29）；tag `v0.17.0` 指向该提交 ✓ |
| 日期戳切片 | Java comments 入口等 | `java-comments-unified-entry.md`（真实 JDK21 用例）、`default-workspace-comments-dispatch.md`（290 组 1565/0/142，与 tasks.md 行 1906 数字一致）；后者记录的 CI `37423582241` 实测 **failure**（gate 阶段），与"gate 失败、不称 CI 通过"的记载一致 |
| workspace 回归 | 54e905c；CI 37414921955 终态 failure | `54e905c` 是 HEAD 祖先；`gh run view 37414921955` → **failure** ✓（证据如实记失败） |

远端核验总评：抽查的 3 个 codeguard CI run、2 个插件 tag/release、npm 五个版本全部与账本一致；**两处 CI failure 在证据里都如实记为 failure**，无粉饰。

### 3.2 发现的问题条目

1. **13.4.4 记录的合并提交哈希是损坏的 42 字符串**（tasks.md 行 1031）：`461f1529f92135c51c2bf569a864f7f78e459e439c` 不是合法 SHA-1（实测 `git cat-file -t` 报"Not a valid object name"；第 30 位起多抄了两个字符）。真实对象为 `461f1529f92135c51c2bf569a864f78e459e439c`（插件仓 v0.18.0 tag 实测指向它，GitHub Release v0.18.0 存在，PR #86 合并记录在 `git log --all`）。**发布事实为真，账本哈希抄写损坏**；后续凡按此哈希做完整性核对都会失败。属于 tasks.md 文本，归 openspec 所有者修正。
2. **计划 JSON `source_hashes` 滞后 HEAD 一个提交**：312 条文件哈希中 309 条与当前 HEAD 一步不差，但 `grammar_native_differential.rs`、`next_command.rs`、`work_sync.rs` 三份与 HEAD `2b68317` 的实际 SHA-256 不符——三者都在 `2b68317`（当前 HEAD）被改，而计划 JSON 最后更新于 `97df711`。按计划的"来源锁定"语义，这 3 条现在是过期引用。`rulepacks/` 不在本审计可写范围，仅上报。
3. **部分证据文件无日期**：`discovery-baseline.md`、`go-candidate-baseline.md`、`planned-language-gaps.md` 全文无日期字样（其余多数有精确日期+提交）。按本项目自己的验收记录规范（handoff §11 要求基线 commit/日期），这属证据规范缺口，不影响其内容有效性（内容与当前 JSON/测试仍一致）。
4. **最旧采样证据**：`capability-inventory.md`（2026-09-24，任务 1.5/5.8）。其"54 stable/3 planned、1710 单元全 gap"的声称与当前计划 JSON 仍一致（本次实测），未发现漂移；但若 capabilities 注册表再演进，此证据不会自动更新。
5. **无法核实项**（如实标注）：14.2 的"CI gate 仍在运行"后续、13.4.2 的 CI run `36512426737` 成败、npm 0.1.2 当年 integrity 值与今天注册表值的一致性（latest 已是 0.1.4）——本次未逐一远端拉取原始 run 日志核对，标注为"未核验"。

## 4. P0-A：32 份 grammar 各缺什么

### 4.1 全体共同缺口（32/32 全缺，来自 manifest + 计划 + 回放三源交叉）

| 缺口类型 | 现状 |
|---|---|
| 发行资格 | `release_status=candidate_unvalidated`、`released_count=0`；无一份进入发行清单 |
| 版本/方言范围 | `language_versions` 32 份全为空；version_scope 全部 unqualified |
| 运行时验证深度 | 仅 `rust_loader_smoke_passed`（冒烟），无全量语料级验证 |
| 独立 holdout | 无（358 例回放与九语言差分均自记"不是独立 holdout，不能汇总准确率"） |
| 五平台/宿主反馈/可信关闭/复发重开 | 计划 JSON 32 条 syntax 路径 blocker 全部挂着此项 |
| 语料精度验收 | 358 例含 FP1/FN10/unknown3/pending2，`grammar_qualified_count=0` |

### 4.2 按子集归类（manifest known_limitations + 差分证据交叉）

- **已有窄样本原生对照（≈13 语种，非资格）**：c（Apple clang21，13 例）、cpp（C++17 回放）、go（gofmt 1.23.4，20 例，**raw 仍 2 FN**）、java（javac21，8+5 例，另有 Java21 新语法 8 例不进 holdout）、javascript（Node 24.18，18 例，**2 FN：module_return、duplicate_binding**，且已发现差分入口用错 worker 入口未评重复绑定）、ruby（2.6.10，18 例 0FP0FN）、rust（rustfmt 1.9.0，13 例）、zig（0.16.0，11 例）、kotlin（kotlinc 2.4.10，**2 例 hidden recovery 仍 unknown**）、swift（6.4，**1 例 unknown**）、erlang（erlc OTP28 对照，**已知 10 项原始漏检**）、python（Ruff 差分）、dart（固定解析器+Zig 重建外部扫描器）。
- **仅有加载冒烟 + 窄 fixture、无原生对照（≈19 份）**：csharp、lua、luau、tsx、typescript、objc、solidity、arkts、nix、terraform、pascal、cfml、cfscript、scala、r、php、vbnet、cfquery、cobol。
- **资产来源/ABI 特例**：objc、solidity 来自依赖包 `tree-sitter-wasms@0.1.13`（grammar status 显示 `dependency_bytes_not_pinned`）；dart 为重建资产（source_wasm 与原始不同）；zig 需 `__main_argc_argv` 导入适配；csharp 加载符号是 `c_sharp`；cobol 符号大写 `COBOL` 且 **16,355,286 字节超过加载器 8 MiB 限额（当前根本加载不了）**。许可证/SHA/ABI 字段在 manifest 里 32 份都有记录（来源/许可治理基本在案）。
- **待裁定项**：cfquery（合法 `SELECT DISTINCT FROM users` 零恢复、PostgreSQL 方言拒绝）、cobol（pending 标签 + 超限）、vbnet（未缩进方法疑似误报 1 例，需固定 .NET 编译器裁定，安装需用户授权）。

### 4.3 可并行切分建议（互不阻塞）

1. 隐藏 ERROR/MISSING 原因贯通（Kotlin/Swift unknown → 精确 reason 版本化字段）——handoff 首切片，RED 已复现。
2. Erlang 10 FN：组合结构规则（`rulepacks/erlang/form_terminator.json`）独立修复+回放，**不得触碰受保护差分测试**。
3. JavaScript 差分入口对齐（`run_syntax_worker_candidate` → binding 候选入口）+ 2 FN 裁定，与 1/2 完全独立。
4. vbnet 疑似 FP 裁定：隔离 .NET 安装方案需授权，等待期间不阻塞其余 31 份（handoff 明示）。
5. cfquery/cobol 人工+原生 oracle 裁定（cobol 另需决定加载限额策略：提限额/瘦身/豁免）。
6. 19 份"仅冒烟"语种的原生对照扩展：每语种一个独立切片，按语言并行。
7. `language_versions`/方言声明填充：manifest 字段已存在（全空），纯声明+验证工作，可全量并行。
8. 发行实装链路（released_count 0→N 的清单/许可/ABI 门禁）：依赖 1–7 的产出，最后串行收口。

## 5. 风险与陷阱（给后续开发者）

1. **`legacy_status: stable` 不可信**：54 语言带 stable 标记，但 version_scope 全部 unqualified，且代码测试强制四核心 blocked。任何"旧清单说 stable 所以可用"的推断都会被 `production_acceptance_plan.rs` 测试与计划 blocker 否定。
2. **回填契约 ≠ 实现**：23 项 A 类勾选只是 fixture 级契约测试（266 通过/46 忽略那一轮），不要把 tasks.md 的勾选数（66/354≈19%）当作能力完成度——真实能力资格是 0。
3. **受控 fixture ≠ 原生实测**：大量集成测试用受控工具输出；真实环境条件测试默认 ignore（本轮 46、workspace 轮 138/142 项忽略）。**被 ignore 的工具条件测试必须记"未运行"，绝不记 PASS**——handoff 与各证据文件反复强调，是本项目第一军规。
4. **局部零诊断 ≠ 通过**：例如 `java-comments-unified-entry.md` 里"补齐注释后零诊断"只是局部观察；证据文件自身注明"局部零诊断不代表项目交付"。 §2 的 0/228/0/32 才是门禁真相。
5. **受保护 Erlang 测试**：`erlang_native_differential.rs` 禁改禁跑（SHA 已验证未动）；因此**禁止 `cargo test --workspace`、`cargo clippy --workspace --all-targets`**，一律单 crate 定向。
6. **冷启动抖动**：2026-10-06 回归首轮 6 项失败（超时/未启动）、同源码重跑 76/0 通过；证据明文"不授予冷启动可靠性"。勿把一次通过当稳定，也勿把首轮失败当回归。
7. **CI gate 常态性失败于固定插件源**：固定提交 `dec5f9d` 在远端不可达，导致 54e905c/4445f06 的 gate 阶段失败（实测两个 run 均 failure）。看到 CI failure 先看是不是这个已知原因，不要盲目"修"。
8. **计划 JSON 的 source_hashes 会滞后**：HEAD 每动一次共享文件而 `rulepacks/production_acceptance_plan_v1.json` 未同步，就出现 §3.2-2 那种哈希漂移。引用 source_hashes 做完整性判断前先重算。
9. **tasks.md 里的哈希可能有抄写错误**：13.4.4 的 42 字符串即例。核对提交先 `git cat-file -t` 验证合法性。
10. **同日多份回归报告数字不同**（1535/138 vs 1565/142）：对应不同源码提交与轮次，引用时必须连提交号一起引，不可跨报告混用。
11. **公开 npm 包能力 ≠ 仓库能力**：0.1.2 公开包未启用 `wasm-precheck` 特性（其证据自记）；公开制品落后于仓库内部状态。
12. **"首次失败保留"原则**：历史证据中的失败记录是被有意保留的（不覆盖、不放宽断言换绿）。重跑通过不能删除/改写旧失败证据。
13. **"相邻 codeguard-cli/…"旧路径表述**：早期任务文字里的 `codeguard-cli/tests/...` 指本仓库自身（历史名），现仓库根为 `codeguard`；按文字路径找会扑空，应映射到本仓库 `tests/`、`crates/codeguard-cli/tests/`。

## 6. 总账

| 维度 | 数 |
|---|---:|
| 任务 | 354 总 / 66 勾选（18 真实完成 + 23 契约切片 + 25 局部切片）/ 288 未完成 |
| 语言×四核心能力单元 | 228 / 228 blocked，0 合格 |
| 构建生态路径 | 364：partial 46、wasm_candidate_only 26、not_integrated 292、合格 0 |
| grammar | 候选 32（28 语言）/ 已验收发行 0 / 资格 0 |
| 已发布制品 | npm @partme.ai/codeguard 0.1.0–0.1.4、plugin v0.17.0–v0.18.3（均候选性质） |
| 审计发现的记录缺陷 | 哈希抄写损坏 1 处（13.4.4）、计划哈希滞后 3 文件、无日期证据 3 份 |

一句话结论：**账本诚实——勾选、能力矩阵、grammar 资格三处数字互相咬合且与代码/远端实测一致；项目处于"工程基线完成、生产能力零资格"阶段，P0-A 的 32 份 grammar 是当前最大的可并行缺口池。**
