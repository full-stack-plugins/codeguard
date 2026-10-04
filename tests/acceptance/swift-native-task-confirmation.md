# Swift 无定位恢复的原生任务确认

日期：2026-10-04。规格事实源：`introduce-rust-codeguard-cli` 的 syntax-precheck，既有 9.9/9.10/14.10/14.11/14.19。本批不更改核心验收门槛、grammar 字节或资格，父任务保持未完成。

## 实际行为

原先 Swift `func f(_ x: ) {}` 的固定 grammar 隐藏错误导致无定位恢复任务，`task verify` 只能返回缺 adapter。现在显式 `--swift-tool ABS_PATH` 通过统一 Rust runtime 调用已有 Apple Swift 6.4 frontend parse，记录同一任务的原生有界位置、工具和输入摘要、追加事件、next 及 repair_ready。参数不匹配和相对入口在租约前拒绝；原始诊断文字不进入 Hook 摘要。

列坐标为 UTF-8 字节，合法边界与当前源码逐项复核。原生读取冻结 stdin，cwd 固定 `/`，环境清空，沿用同一 deadline 和 64 KiB 输出预算；不读取项目构建参数、不类型检查、不展开项目插件或执行源码。Apple 启动器在 stderr 输出独立 driver 版本，只有精确三段数字 banner 被允许，未知输出不猜成源码违规。有先行 error 的同源合法 note 仅用于源码/caret 上下文，不能变成另一条违规；无先行错误的 note、warning 和未知路径不猜成成功。工具摘要绑定入口，完整工具链来源/组件身份仍需更广验收。

诊断推动源码修复，源码或工具变化使旧位置 stale；复检无诊断只产生 `candidate_absent_unverified_policy`，任务仍 open。失败尝试和两次无进展继续停止重复修复并给具体决策需求。此批没有新增 Swift 可信关闭 SDK、默认宿主策略、完整 lint、类型/项目覆盖或发行资格。

## 协议

新增五份封闭 schema，历史原件不改：复检 0.4.0、复检反馈 0.15.0、next 0.6.0、Hook 0.9.0/摘要 0.3.0、携带 Swift next 的 check 0.39.0。check 其余入口保留 0.38.0，Zig/Erlang 既有协议保留。task show 仍按既有可读任务投影。

## 验证与证据

首轮目标测试 1 passed/3 failed/1 ignored，分别暴露参数、Hook 和 adapter 恢复缺口。受控测试最初通过后，真实 Apple 工具测试失败，发现正常 driver banner 被错当 stderr 异常；修正精确 banner 识别和原生源码/caret 上下文解析后，随后真实缺大括号/括号样本的定位 note 暴露错误被丢弃，针对实际原生文本的红测先失败再修正。最终目标 8 passed/0 failed/0 ignored，包含显式 `/usr/bin/swiftc` 的 13 例基础原生复检（8 合法/5 非法），并验证多字节坐标。仅是已选窄范围样例，不计算产品 precision/recall。

原生 13 例基础样本、修复后输入、多字节错误、缺工具、task show、repair_ready 和聚合 next 的完整报告归档：[实际报告](evidence/swift-native-task-confirmation-2026-10-04.json)。固定二进制前后摘要一致；这个档案是 CLI 命令回放，不声称真实 Claude/Codex/Gemini 对话验收或独立精度评测。最终回归、格式、Clippy、schema 和 OpenSpec 终态追加于下。

日志：`/tmp/codeguard-swift-recheck-red.log`、`real-red.log`、`final.log`、`regression.log`、`capture.log`，后四项均同 `codeguard-swift-recheck-` 前缀。完整 WASM/358 例没有重跑，Erlang 10 FN、VB.NET 1 FP 和其余未裁定语料不改；预先 Erlang RED 草稿保持未提交，公开 npm/插件锁保持。

测试选择记录：一次将 `--include-ignored` 同时用于 library 和 Swift 集成目标，误触发未提供 Node/ESLint 工具环境的显式原生 library 测试，36 passed/1 failed；这是验收命令范围错误，没有修改 ESLint 行为或跳过其验收标准。重新分别执行正常 library（36 passed/0 failed/1 ignored）与显式 Swift 目标（8 passed/0 failed/0 ignored）。原失败日志 `/tmp/codeguard-swift-recheck-note-final.log` 保留。

## 最终验收终态

- 默认 workspace/all-targets：220 组，1237 passed/0 failed/113 ignored；最终源码已全量执行。
- 受影响 WASM 14 组：173 passed/0 failed/23 ignored；随后仅 Swift 诊断上下文与独立 help 补齐，最终 library 36 passed/0 failed/1 ignored、Swift 目标 8 passed/0 failed/0 ignored。各范围重叠，不合计。
- 默认/WASM 全目标 Clippy -D warnings、fmt、crate 分层、OpenSpec strict 通过。208 schema 元定义，50 份实际完整报告及 150 个伪造变体通过；开发协议测试 5 passed。
- Linux CI 37192444784 在安装后 Erlang 测试的旧聚合版本断言失败（0.37 vs 实际0.38）；同一本地私有包已重现。修正为精确0.38并要求 syntax_tasks 为空，保持原生覆盖不重复造任务；受控协议与真实 OTP 28 重新离线安装回放，runner 3 passed/0 failed/0 skipped（含父测试）。[实际安装报告](evidence/npm-erlang-recheck-v0.38-2026-10-04.json) 不覆盖旧档案。新提交 CI 需独立确认。
- Swift 固定程序 SHA-256 为 `30db5a53c6d766937ea8a81ae167f89fa52219d5b4373be681c2b2a2b55fbb42`，实际回放及安装后源制品前后相同；没有改变 grammar、公开 npm、插件锁或受保护 Erlang RED 草稿。

日志摘要：

- `/tmp/codeguard-swift-recheck-red.log`：SHA-256 `70477c1ec2cb31e8e7fc7aa3908c1ad16fcbb4c30ebca5da25ac82bfc0ae8ec9`。
- `/tmp/codeguard-swift-recheck-real-red.log`：SHA-256 `15bbbf2a69240cfcefb49348dbecd3207610ef30f8922a391a158a8b42f47077`。
- `/tmp/codeguard-swift-recheck-note-red.log`：SHA-256 `86e4bfce333afc3c0f975784453f83ae2dce041f0c7da8246bbd99078c33c995`。
- `/tmp/codeguard-swift-recheck-target-final.log`：SHA-256 `415ccd967b8f52c9dbe8842334adb67f2d74cfb5c810c02cb190c251c847a2ff`。
- `/tmp/codeguard-swift-recheck-library-final.log`：SHA-256 `30fa197609ea38e6e31b3da31b80e2c96307ea1114477f5b9d3b3790d6ae0d23`。
- `/tmp/codeguard-swift-recheck-regression.log`：SHA-256 `c397fabfbc3527a487699bd8245239641cdce8000c28c3c41d95925b32d8f675`。
- `/tmp/codeguard-swift-recheck-workspace-final.log`：SHA-256 `9cb1f54b275f2fdf86e01f6891f19bda4285f705736a05036311687e2b0fd010`。
- `/tmp/codeguard-swift-recheck-default-clippy-final.log`：SHA-256 `72c3f0642645ce66444421c2389a191e3b9a2c5ac26bdb0945e01b8de67a220d`。
- `/tmp/codeguard-swift-recheck-feature-clippy-final.log`：SHA-256 `d9847d0185749efaff3a791bc8847d13607455e8e9aaaaf9949f55fc9e96047d`。
- `/tmp/codeguard-swift-recheck-npm-red.log`：SHA-256 `7ef679fb4482abde34365356ab4a5911c330098dae1af66ac35e4cf9faf4ad72`。
- `/tmp/codeguard-swift-recheck-npm-final.log`：SHA-256 `efd54325d70ef4d42b945719f81cea8de6c989e2fe8785cb537d4e4c876be109`。
- `/tmp/codeguard-swift-recheck-capture-final.log`：SHA-256 `a8c330bc7dcbec932eb4192aa028e08f344c604893bdeae6cabe69057e1874e4`。
- `/tmp/codeguard-swift-recheck-all-evidence-schema.log`：SHA-256 `2b05d234a8bf50a8cad45d3cce5f1641b8d89810d7912dc7b0ac4636164eb9de`。
- `/tmp/codeguard-swift-recheck-schema-tests.log`：SHA-256 `1331eb27b2fa236183659a52b4a93ed9e2f99bebe81e963b5aad7d89ffb0cc01`。
- `/tmp/codeguard-swift-recheck-openspec-final.log`：SHA-256 `cf3ec12601576e4a3dd335699d867edc30029dd07ee9fdcd6396379524b1fa4b`。
- `/tmp/codeguard-swift-recheck-fmt-final.log`：SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。
