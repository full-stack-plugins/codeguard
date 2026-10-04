# Erlang 原生首次发现进入工作台验收

日期：2026-10-04。对应唯一 change `introduce-rust-codeguard-cli` 的 syntax-precheck 原生首次证据场景及 8.134 / 9.4 / 9.10 / 14.10–14.11。此记录证明所列流程，不证明完整语言、可信关闭或发行完成。

## 原问题与可观察修复

此前 `check all` 即使已有 OTP 28 原生诊断，仍报告 `native_findings_not_integrated`，首次任务必须依赖 WASM 候选。初始流程测试实际失败，任务引用不存在。现在已初始化工作区可直接用当前原生证据创建稳定任务，不要求先有 WASM 错误。

`check all` 与 `lint erlang FILE` 复用 workspace / 文件 / 语言身份；重复扫描和历史 WASM 来源不重复建任务。单文件入口定位最近已有 `.codeguard/`，不初始化未绑定目录、不越过损坏工作台。批次只同步一次工作台。原生首次证据 `observations: []`，不伪造 grammar 身份；其复检原报告引用的 `grammar_sha256: null` 通过新协议明确表达。

`next` 从已消费的扫描和复检报告选择最新观察；扫描不是 `task_verify` 事件。当前诊断指导源码修复，缺工具/不支持版本/预处理指导恢复环境；源码或工具变化撤回旧位置。保存失败保留诊断，任务 ID 为 null 且原因可见。首次零诊断不创建任务，已有任务的零诊断记录为 `candidate_absent_unverified_policy`，仍 open。`repair_ready` 使用同一原工具、尝试/租约与追加式复检证据。

## RED → GREEN 与边界

- 初始任务引用缺失 RED：`/tmp/codeguard-erlang-native-workbench-red.log`。此前写错 JSON 根字段的一次测试草稿不作为产品失败证据。
- 缺工具未进入任务 RED：`/tmp/codeguard-erlang-native-workbench-boundaries-red2.log`，5 passed / 1 failed。第一次边界草稿的部分失败来自错误的预期枚举，修正为既有协议后才得到此产品 RED；不把草稿断言错误冒充缺陷。
- 单文件入口未绑定任务 RED：`/tmp/codeguard-erlang-native-lint-workbench-red.log`。
- 模板审查实际复现原生任务正文错误宣称固定 grammar 和未接入 adapter，`/tmp/codeguard-erlang-native-task-document-red.log`；原生首次任务现按 7 个必需字段给出真实证据来源、修改范围及 --erl-tool 复检，WASM 模板也不再一律声称 adapter 未接入。已有人工任务正文不自动覆盖，next 提供当前权威边界。
- 模板修正前的工作台特性目标 11 passed / 0 failed / 1 ignored，覆盖首次/重复、源码修复/再出现、环境/宏、保存失败、工具/收据变化、单文件跨目录、最近损坏工作台、Hook、WASM/native 同任务。
- 严格导入测试逐项拒绝 10 个变体：未知版本、交付 allow、越界位置、源码/工具摘要不符、预处理伪装完成、不支持 OTP、错语言、伪造 WASM 与错规则；每个变体在移除原任务和收据后确认无新任务。
- 默认四组相关目标 35 passed / 0 failed / 4 ignored；七组特性目标 69 passed / 0 failed / 6 ignored。随后工作台目标补最近工作台、跨来源身份和任务正文用例，当前完整结果另记。阶段有重叠，不合计为独立测试数量；忽略项不算通过。

## 真实原生执行与协议

显式 `/opt/homebrew/bin/erl`（OTP 28，规范启动器为 `/opt/homebrew/Cellar/erlang/28.5/lib/erlang/bin/erl`）实际运行首次坏函数发现、原任务复检和修复后的零诊断，显式目标 1 passed / 0 failed / 0 ignored。这不是编译器全项目测试或全语言精度测量；工具摘要只绑定启动器，不绑定整套 VM/stdlib 供应链。

真实 CLI 输出保存于 `/tmp/codeguard-erlang-native-workbench-observations/`：聚合检查、内嵌 forms、单文件 lint、首次 next、首次原生报告、坏源码复检、内层复检、repair_ready、陈旧 next、修复后复检及 next。197 个 schema 元定义、11 份真实输出和 16 个伪造协议变体已校验。双语专题的 4 个完整 JSON 示例与实际输出一致，两份技术方案的 forms 示例也更新为真实任务引用；不是手填成功报告。

新 schema：observation 0.2、forms 0.2、check 0.37、lint 0.3、首次 next 0.5、native-first recheck 0.3 / 外层 0.14。复检后的 next 仍为 0.4、Hook 仍为 0.8；历史 WASM 来源 recheck 0.2 / 外层 0.13 不改。全部 190 份历史 schema 与本批 HEAD 字节一致。

workspace 全目标 WASM Clippy `-D warnings`、fmt、分层与 OpenSpec strict 已通过。最终全工作区与新 SHA 远端结果另记，不以局部目标代替。模板修正前全工作区 212 组、1171 passed / 0 failed / 109 ignored 已完成，随后修正模板并增加其回归；最终源码全量另行复跑，阶段结果不当作最终结果。

## 仍未完成

完整 Erlang 项目 lint、宏/include/条件编译、注释规范、可信关闭及复发重开、实际安装宿主、MSRV、多平台、独立语言精度与完整门禁仍缺；父任务保持开放。原 WASM 的 10 个终止符漏检保留，37 例 RED 草稿未提交，也未重建 grammar。公开 npm 0.1.4 与插件 lock/版本没有改变。

实现说明与实际完整报告：[中文](../../docs/Codeguard-Native-Repair-Workflow.zh_CN.md)、[English](../../docs/Codeguard-Native-Repair-Workflow.md)。

## 日志身份

- `/tmp/codeguard-erlang-native-workbench-red.log`：SHA-256 `a18d26820fb97f4e81828af465e634deb9e99361fe89c6927bcd88698f93c72e`。
- `/tmp/codeguard-erlang-native-workbench-boundaries-red2.log`：SHA-256 `3991062d6bb73efa7b57d88b94e980b557266a0cc976bb00e4b3369956c26813`。
- `/tmp/codeguard-erlang-native-lint-workbench-red.log`：SHA-256 `d735e7b5a4155e2d2065a97d48556db930043c4167c9a57c2fa1646f0925be2c`。
- `/tmp/codeguard-erlang-native-task-document-red.log`：SHA-256 `c161257d810d9c4aba2904872431311572e3697868ee1248270d6c4a18e073d0`。
- `/tmp/codeguard-erlang-native-workbench-default-final.log`：SHA-256 `9790edaf861f090329c1384394e19a02878914208bedb7f1e828cc0dfeab643f`。
- `/tmp/codeguard-erlang-native-workbench-feature-final.log`：SHA-256 `0cb4101cdef588adefdcd63a60043a2dc851971cbce1060eac4e476df1d88735`。
- `/tmp/codeguard-erlang-native-workbench-identity-final.log`：SHA-256 `3ec43a2064630c85716bd63fa4ea84e4d1961aa441c9a834a4574ab5fe03a6ac`。
- `/tmp/codeguard-erlang-native-workbench-task-text-green.log`：SHA-256 `e5e3515240054c1aeb7c3211468b534a54d422f51c1015697e7b344145033ade`。
- `/tmp/codeguard-erlang-native-workbench-real.log`：SHA-256 `f04468758b0409c4773fb53598926df3f8e21b1d68118e88714c5c6d62d3ae2c`。
- `/tmp/codeguard-erlang-native-workbench-schema-final.log`：SHA-256 `3f638554cd200f5f88d6c9db492a270b56793fba7b0fd1ec744281c5e26cc96f`。

原提交 `874b13b0800fdb420cf0354ba4eb2f7621d3a854` 的 [Linux CI](https://github.com/full-stack-plugins/codeguard/actions/runs/37168752261) 已 completed/success；本批新源码不能借用该结果。

## 最终默认全工作区终态

最终源码 `cargo test --workspace --all-targets --locked --offline` 212 组、1172 passed / 0 failed / 109 ignored，退出 0。日志 `/tmp/codeguard-erlang-native-workbench-workspace-final.log` 的 SHA-256 为 `b7d7854b09fcb3c39c61a4d1ac0dc3a3ba692a0c04d920321b64aa48dc55a0c5`。模板修正前的 1171 项阶段结果保留，不合计或覆盖。已知 Erlang grammar RED 草稿仅在 WASM 特性启用时运行，此默认全量不证明 grammar 修复；没有把 109 个忽略项当作通过。

模板修正后的最终六组特性目标（工作台/Erlang/WASM任务/Hook/限定关闭服务/crate边界）51 passed / 0 failed / 4 ignored，退出0；其中工作台12 passed / 1 ignored。日志 `/tmp/codeguard-erlang-native-workbench-feature-current.log` SHA-256 `ed72e2aa2e23a5cd8dbc711656d13403b446eaa2b69320b42c4bef196323c808`。阶段有重叠，不与前述69或11合计。

最终源码的两个显式真实 OTP 28 目标均各 1 passed / 0 failed / 0 ignored，分别覆盖原生首次任务和历史 WASM 来源任务复检。日志 `/tmp/codeguard-erlang-native-workbench-real-current.log` SHA-256 `c081c6f39d628d116ebc7606d2966ed3dcf65840188da442ba691c8d08d5a749`。随后 workspace/all-targets/WASM Clippy `-D warnings` 退出0；日志 `/tmp/codeguard-erlang-native-workbench-clippy-current.log` SHA-256 `8d00fc8faf528f3c0c05fc7d04fe815122d22d7bfdcfcc324ea66d97d37bd3f5`。

原生准备边界核对：当前 grammar 未验收，Erlang 还存在已知终止符漏检，因此零恢复节点仍是 incomplete，不满足 SP03 的“完整正常初检”条件；缺工具恢复任务不能被伪造 clean 降为推荐。未来经批准、非空、范围完整的 clean 才可仅推荐；已有必需原生义务始终保留。此核对不授予 grammar 资格，也不把首次原生 forms 零诊断当作完整项目通过。
