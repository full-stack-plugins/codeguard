# 失败写入与预配置检查器参数验收

日期：2026-10-06。对应现有 OpenSpec hook-protocol、9.3、11.17；不关闭全语言、真实宿主或发行父任务。

## 问题与实现

Hook 的 FastFileCheck 与 NoCheck 共用了六种已接线工具的参数白名单。因此失败写入携带 `--go-tool`、`--cargo-tool` 或 `--maven-tool` 时，在进入 NoCheck 前退出2，错误提示为“任务复检参数仅用于repair_ready”。新公开反例确实得到该退出码，测试RED：2通过、1失败。

NoCheck现在忽略所有已登记检查器配置参数，返回原有0.7协议的 `not_run/write_failed`，交付仍 `not_evaluated`。语法解析仍先拒绝未知、重复、空、相对路径和16KiB超预算参数；`--owner`与`--lease-token`继续仅允许任务复检。FastFileCheck参数边界没有扩大，尚未接线的Go工具在确认编辑时仍明确拒绝。

## 观察与验证

`crates/codeguard-cli/tests/hook_failed_write_options.rs` 的可执行夹具若启动任何配置工具就写标记；全部登记参数共同传入失败写入后无标记、无工作台、无局部反馈。独立反例覆盖租约、所有权、相对路径、未知/重复参数、空值、超预算和确认编辑不得静默忽略Go工具。

默认及WASM受影响五目标（新增失败写入、通用Hook、Claude、Ruby、Shell）：各46通过、0失败、3忽略。最后补充空值/超预算后，新增目标在两种构建各重跑3通过、0失败、0忽略；不要将重复运行加进独立测试数。

日志：`/private/tmp/codeguard-failed-write-options-{red,green,wasm,final-default,final-wasm}.log`。默认及WASM workspace/all-targets Clippy `-D warnings`通过；分层、OpenSpec strict、拥有文件fmt及diff检查通过。没有重跑完整workspace测试，之前1452通过属于前一源码基线。

实际WASM CLI以不存在的绝对Go/Cargo/Maven工具配置返回退出3，无stderr，无工作台；[原始反馈](evidence/hook-failed-native-options-2026-10-06.json)通过未修改的 `schemas/hook-execution-feedback.schema.json`。这是CLI规范化事件证据，未冒充已安装宿主实测。

## 剩余范围

Go与Rust确认编辑的原生优先接线仍未完成。Go `vet` 是包/模块检查，不能在每次编辑时直接全项目运行或伪装单文件覆盖；需要限定范围、构建条件与输入身份的设计及实际验收。32grammar精度、可信任务关闭/复发、实际宿主、跨平台和发行继续开放。用户Erlang草稿不修改、不执行、不纳入提交，散列保持 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`。
