# Erlang 任务复检自动发现原生工具

日期：2026-10-04。规格事实源仍为 `introduce-rust-codeguard-cli`，对应现有 9.9、9.10、14.10、14.11、14.19；父任务保持开放。

## 缺口与最终行为

lint/check 已经从调用方绝对 PATH 选择 Erlang，task verify 却仅接受显式工具。因此已安装 OTP 28 时，已有语法任务仍返回 explicit_erl_tool_not_provided，无法从恢复指引进入原生确认。本批复检复用现有 ErlangToolSelection；只在任务语言为 Erlang 时发现工具，不从可编辑历史报告执行旧路径。显式入口优先，首个可执行入口固定，版本、运行和工具字节检查使用现有探针。

没有显式参数仍可确认候选来源与原生首次来源任务，repair-ready Hook 复用 task verify。next 依据本轮有效原生证据生成显式原工具复检 argv；PATH 后续改变不能替换这个明确入口。真实缺工具返回 erlang_tool_not_found_on_path，并指向恢复环境而非修改源码。相对/空 PATH、不可执行文件不作为自动工具；显式坏工具、所选版本或执行失败不改用后续同名工具，不自动安装。

本批没有改报告形状或历史 schema：候选来源 syntax_task_recheck 0.2 / task_verification_preview 0.13，原生首次来源 0.3 / 0.14 保持；Hook 返回脱敏摘要与真实报告引用，不直接包含完整原生报告。局部零诊断继续为 candidate_absent_unverified_policy、任务保持开放，不授予完整 Erlang lint、构建、预处理或交付许可。

## 测试与实际证据

三个新增选择反例先实际执行，0 passed/3 failed，分别暴露已安装工具未调用、首个失败工具未被记录和非显式工具未固定。实现后前三反例通过。新增范围还包括原生首次来源、repair-ready、坏显式入口、首个版本/运行失败不换工具、相对/空/不可执行来源、PATH 变化时显式复用原工具，以及已安装 OTP 28 的坏源码和修复后复检。

第一次扩展回归的 Hook 断言错误读取脱敏摘要的 native_scan.tool_path，按现有协议改为检查摘要状态和引用的实际报告，没有添加路径泄露字段。上一批远端 run 37190658048 失败于 check_all_eslint 的旧“必须安装”文案断言；本地同目标重现 0 passed/1 failed，现检查“必须准备或修复适用的原生”及“不要仅凭候选结果修改源码”，保留要求原生确认与脱敏的行为断言。

[固定二进制和实际 OTP 28 复放](evidence/erlang-recheck-discovery-2026-10-04.json)记录候选及原生首次两种来源、无参数坏/修复后复检、repair-ready、缺 PATH、坏显式入口、next 与开放事实。这是实际 CLI/Hook 命令复放，不是实际宿主验收。

## 验证终态

- 默认 workspace/all-targets：219 组、1235 passed/0 failed/113 ignored。此后仅调整缺工具历史指引措辞，最终源码的默认 Erlang 工作台目标 12 passed/0 failed/1 ignored。
- 最终源码受影响 WASM library 与 15 个集成目标：16 组、199 passed/0 failed/25 ignored。包含远端失败的 check_all_eslint、候选与原生首次任务、Hook、工作台、租约、复检、next 及限定关闭服务。各组与先前目标重叠，不合计为独立覆盖，ignored 不算原生验收。
- 三个初次目标先有 0 passed/3 failed；修复后的三目标 37 passed/0 failed/3 ignored。本机已安装 OTP 28 新增显式原生目标 1 passed/0 failed/0 ignored，最终源码另有固定二进制实际复放，两种来源都验证诊断→同工具复检→零诊断候选及缺工具、坏显式入口，任务仍开放。
- 默认和 WASM workspace/all-targets Clippy -D warnings、fmt、分层与 OpenSpec strict 通过。历史 schema 均未修改；本批新增实际证据不覆盖其他批次。
- 最终二进制前后 SHA-256 均为 `e2f15ca12af6ec6c1260f1d9073cb9715d66b59697fa552c187e6f1581870998`。实际报告仍不批准 grammar、完整项目检查或真实宿主。
- 前批 ce9d7ac 的远端 run 37188782374 已完成 success；4afa8bc 的 run 37190658048 完成 failure，具体旧文案断言已本地重现并修正。本批远端结果独立核验，不借用前批绿灯。

## 剩余验收

Erlang 10 个 grammar 漏检未修复，其他语言原生适配器、默认宿主受保护策略、通用正式关闭与复发、版本/方言、预处理和项目完整能力仍缺。本批未改 grammar 字节、npm 0.1.4 或插件锁；预先存在的 Erlang RED 草稿保持原样。没有用局部回归代替完整 WASM suite 或 358 例语料验收。


固定日志摘要（本批实际运行）：

- `/tmp/codeguard-erlang-recheck-discovery-red.log`：SHA-256 `c93d76fc6850282d971718eae396d7f86a396ceb4ea040bdb41cfd63d6c96256`。
- `/tmp/codeguard-erlang-recheck-discovery-targets.log`：SHA-256 `be2e018101c3c420680d8eacde8fe8e269099bf6ea1ddb87e6a34ccd7cbdb861`。
- `/tmp/codeguard-dialogue-wording-red.log`：SHA-256 `b276bddcac1a7ab9f3aab6da3a36f44e8fc67d26734b40d62b0f5758abeaf64d`。
- `/tmp/codeguard-erlang-recheck-discovery-targets-final.log`：SHA-256 `bdf72e0843b1436e6d0225eb44f8b86dbf8d02d4d23bd03bb39643b498da209e`。
- `/tmp/codeguard-erlang-recheck-discovery-real.log`：SHA-256 `9cf17938581553d4fc31532ac37fd7762bac96f9130cc5edcb2d437c498fabcc`。
- `/tmp/codeguard-erlang-recheck-discovery-workspace.log`：SHA-256 `f2fe63252ea602b72c34890798c7b66a69ea499c2d362dc3ca01fcd40126346b`。
- `/tmp/codeguard-erlang-recheck-discovery-default-final.log`：SHA-256 `15c70c5b64280d36c725101a73c4b81a9f9176860d20b44d0889445b1a1e1c0c`。
- `/tmp/codeguard-erlang-recheck-discovery-feature-final.log`：SHA-256 `358bbf4fedea246566e5c9f7444bdeda24ea20c60db7fa5a98433d6f225e9ac7`。
- `/tmp/codeguard-erlang-recheck-discovery-capture-final.log`：SHA-256 `10db729600d1abb21f3f23aa64a73caf2dd1a8971a6d1b3aabde207bcdaad65a`。
- `/tmp/codeguard-erlang-recheck-discovery-default-clippy.log`：SHA-256 `e60076eff0d1e720bfcc90f8eb3717139a82a3684c7e0825bf5430ace1674fc5`。
- `/tmp/codeguard-erlang-recheck-discovery-feature-clippy.log`：SHA-256 `e60076eff0d1e720bfcc90f8eb3717139a82a3684c7e0825bf5430ace1674fc5`。
- `/tmp/codeguard-erlang-recheck-discovery-fmt.log`：SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。
- `/tmp/codeguard-erlang-recheck-discovery-openspec.log`：SHA-256 `cf3ec12601576e4a3dd335699d867edc30029dd07ee9fdcd6396379524b1fa4b`。
- `/tmp/codeguard-erlang-recheck-discovery-layering.log`：SHA-256 `280c626d05d03e0944fca548fce5a1e9039200ab2cdd1f3d68f64c12e236de36`。
- `/tmp/codeguard-ci-37190658048-failed.log`：SHA-256 `4ce8f65147f6df55340680241879bf7e93bcb1990c66e280daec888b15ec88bb`。
- 实际报告归档：SHA-256 `5963b10310ea304e8e013500a654c9ba7af19715f0a7e658936bac8c227ea0d8`。

最终协议及文档复核：203 schema 元定义、16 份完整实际报告、48 个伪造变体和 979 条修改文档本地链接通过；历史 schema 原字节不改。日志 `/tmp/codeguard-erlang-recheck-discovery-validation-docs-final.log`：SHA-256 `b678b5042194c92d003f087edb607f939741fe6a6e0f1b91ebcd9f76de274231`。
