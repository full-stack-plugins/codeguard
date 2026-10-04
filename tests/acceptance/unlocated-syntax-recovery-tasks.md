# 无定位语法观察进入稳定检查恢复任务

日期：2026-10-04。对应现有 OpenSpec 9.3、9.9、14.7、14.10、14.11、14.19；规格事实源仍为 introduce-rust-codeguard-cli。父任务未完成。

## 实际缺口与行为

已有运行时正确将 Swift/Kotlin 隐藏缺失 token 标为未完成，但通用任务同步跳过 recovery_count=0；Hook 只返回未完成提示，没有可执行任务。聚合 check 的通用 WASM 观察原本也没有工作台同步。现在两入口共享原同步服务，绑定当前源码、固定 grammar 和稳定工作区/文件/语言身份。

有位置的候选异常进入原生确认；没有位置但 syntax_recovery_incomplete 的观察进入检查能力恢复任务；完整零恢复不新建任务。每张恢复任务包含证据、规则依据、允许范围、步骤、复检、历史和关闭条件，原生确认前不得修改源码。不制造行列、已确认 finding 或通过。

check_feedback 0.38.0 明确 syntax_tasks、失败范围和可用 next；新本地 syntax_confirmation_observation 0.3.0 允许有未完成原因的零恢复，旧 0.1/0.2 schema 原件保留。原有稳定指纹不变，新版无定位首次任务可消费后续可定位报告；task verify 对缺失原生 adapter 记录 incomplete，WASM 完整零恢复不关闭已有任务。Claude命令上下文区分“疑似恢复节点0项”和“恢复扫描未完成1项”，同时给出真实 task show/verify；此处不是实际宿主安装验收。

## TDD与复核

初始四目标0 passed/4 failed，直接复现遗漏任务、误用一般建议和保存失败无反馈；修复后第一次运行暴露测试对既有 task show/verify 字段及保存原因的错误假设，按实际公开契约纠正，未修改这些协议。增加聚合接线反例后5 passed/1 failed：check all 尚未同步通用观察。接线后整个 hook_syntax_tasks 12 passed/0 failed/0 ignored，涵盖原有6项及新增6项。

新增范围：三个真实WASM无定位样例、重复扫描身份、完整零恢复、后续可定位观察、缺adapter复检、七个无原因/错版本/错身份伪造报告拒绝、写入失败保留证据、聚合与编辑共享任务、对话上下文。Swift缺类型与Kotlin缺类型/对象样例的既有未知分类没有改变；没有为了通过测试删除样本或忽略缺失token。

[固定二进制实际运行报告](evidence/unlocated-syntax-recovery-2026-10-04.json)归档检查、重复检查、Hook、任务、next、复检、上下文及失败保存；后续完整零恢复保持原任务开放。原生Swift对照单独记录，不提升WASM资格。

## 剩余验收

本批不修复grammar字节，不改变32种候选/零资格、Erlang10个漏检、VB.NET1个误报、3个unknown及2个pending。所有语言原生确认adapter、通用正式关闭/重开、真实宿主、多平台、完整独立精度及发布仍缺；已有Erlang RED草稿未修改或提交。没有执行完整WASM suite或重跑358例语料。

## 最终验证

- 默认全工作区/all-targets：219 组、1234 passed / 0 failed / 113 ignored。
- 受影响 WASM library 与 15 个集成目标：16 组、210 passed / 0 failed / 30 ignored；其中 hook_syntax_tasks 为 12 passed / 0 failed / 0 ignored。各组可能重叠，不合计为独立覆盖，ignored 不计验收。
- 默认与 WASM 全工作区/all-targets Clippy -D warnings、fmt、crate 分层、OpenSpec strict 与 git diff --check 均退出 0。
- 203 份 schema 元定义、16 份实际完整报告、48 个伪造权威/交付/未知版本反例通过。实际归档校验曾发现新聚合 schema 的 next 只接受旧 0.1 简报，现仅修正新的 0.38 schema，使其复用完整且严格的 0.1–0.5 简报协议；旧 schema 原件未变。新增[开发协议回归](../unlocated_syntax_recovery_schema.py) 4 项通过，独立覆盖真实 next、嵌套伪造、零恢复条件及旧消费者拒绝未知版本。
- 实际二进制前后 SHA-256 均为 `834da6d1dddbb63671bcac0e75d03314feb9c7f49a85924ff65c1973b0580213`。捕获两类场景和 7 份原始确认报告；真实 Apple Swift 6.4 对缺类型源文件退出 1、补 Int 后退出 0，只是两个单文件 parse 对照，不提升 grammar 资格。
- 初次 WASM 回归命令误写不存在的 target 而退出 101，未执行任何测试；按真实 target 更正后取得上面的终态。初次开发 schema 校验因 next 协议分支遗漏失败，日志保留；修正后重新通过。产品不调用 Python，此开发校验使用本机既有 jsonschema 环境，没有安装工具。


固定证据摘要（本批实际运行，不借用前批结果）：

- `/tmp/codeguard-unlocated-task-red.log`：SHA-256 `6e9f89ebe2925e66098b7a6b38beeb605209be41e6627255558864844f8138c4`。
- `/tmp/codeguard-unlocated-check-red.log`：SHA-256 `80bf4585060436e84f8eac479fbbe1e40a6e5b9609ff95bd1971276314047add`。
- `/tmp/codeguard-unlocated-task-final.log`：SHA-256 `c79fcd3c78693158291afbe92b3f85032690e749357f3ab8c432b3b1f725a242`。
- `/tmp/codeguard-unlocated-workspace.log`：SHA-256 `f99939ae61cdf0c07ba0ea918abb73acc7925a4b4f0db064ee1a43eeb9532406`。
- `/tmp/codeguard-unlocated-feature-tests-final.log`：SHA-256 `f77a831b3834c22e4240beb4128b0bfb88acb7b54e3af6aaeb0ea9d9733e5706`。
- `/tmp/codeguard-unlocated-capture.log`：SHA-256 `cf59cc273c12c59869cfbf2f7f1907bf6d81cd306e4ac7b9fb487d2d5e952e0c`。
- `/tmp/codeguard-unlocated-validation.log`：SHA-256 `218df0bf8c67068d395fbf01c007dc98b4fbed524d6221919a02046e5396e8b5`。
- `/tmp/codeguard-unlocated-validation-final.log`：SHA-256 `242843e5404f85a9be68de8c4674e438061b2e9f91f0a1482131d5cc752968a8`。
- `/tmp/codeguard-unlocated-schema-tests.log`：SHA-256 `3ba64c1014c84ed5a2a3ab9034ee196f8eb4cfe9067c3bb44f08baeabca4ecab`。
- `/tmp/codeguard-unlocated-default-clippy.log`：SHA-256 `0e9800ac1b0e1b18686c8389eda10732715bd9a567323bad76c2513a5303b59c`。
- `/tmp/codeguard-unlocated-feature-clippy.log`：SHA-256 `0c31604735c12a390492d0bb98906e47002acdbbf323b5c5b29dfc60ff018223`。
- `/tmp/codeguard-unlocated-fmt.log`：SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。
- `/tmp/codeguard-unlocated-layering-final.log`：SHA-256 `280c626d05d03e0944fca548fce5a1e9039200ab2cdd1f3d68f64c12e236de36`。
- `/tmp/codeguard-unlocated-openspec-final.log`：SHA-256 `cf3ec12601576e4a3dd335699d867edc30029dd07ee9fdcd6396379524b1fa4b`。
- 实际归档 JSON：SHA-256 `a73f27a19ba6b1d25d73a2636df6236f545a08698678bb385af568d6a42e2eea`。

最终文档复核：930 条修改文档本地链接通过；双语架构文件命名符合现有规范。文档更新后的协议/链接校验日志 `/tmp/codeguard-unlocated-validation-docs-final.log`：SHA-256 `c1a286f6f7cd8f221dc880cc804a19ca689f56ff0a19fc5902cf6c248443a38c`。
