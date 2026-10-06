# 57语言注册表身份与迁移字段核对

## 实際范围

固定插件 dec5f9d1361eefff493e120b4278939a243814a9 的 languages.json，核对 Rust 固定快照全部57项语言字段。54 stable和3 planned身份一致；只有C++、R、TypeScript的四项字段差异。保留Rust已修正的扩展名覆盖；旧gate模板保留为兼容元数据，不变成原生命令计划或质量allow。

完整结果：[逐字段报告](evidence/legacy-registry-audit-2026-10-06.json)。源码入口：[Rust审计工具](../../crates/codeguard-cli/examples/audit_legacy_registry.rs)。该报告只绑定固定快照，不自动代表未来插件HEAD。

## 实际RED与GREEN

新增身份反例在修复前实际1通过、5失败：规范ID替换、stable/planned归属互换、追加未知状态重复行、重复忽略元数据键及未知版本均被错误接受；双快照合法对照通过。修复后适配器库9通过，新身份目标6通过，均无失败/忽略。重复键反例同时覆盖根version和嵌套append_files。

CLI能力库存、能力选择、planned缺口、项目detect、旧协议及审计示例六目标46通过、0失败、0忽略。全部57行均归于保留/纠偏/legacy兼容，四项变化附规格及扩展名测试引用。`--check`重新计算所有字段、拒绝重复JSON键与过期报告；CI在完整WASM检查前执行该契约。

修复前提交5813d1a的完整默认workspace/all-targets为1509通过、0失败、135条件忽略，仅作为前置基线，不声称是本次修改后的完整回归。新身份检查不执行旧工具、联网或用户Erlang差分草稿。

## 未完成范围

OpenSpec1.1仍需旧非语言源码行为的逐项纠偏/兼容对照，以及真实原生迁移验收；不能凭字段核对勾选父任务。语言stable登记不代表新适配器已实现；本批不改变32 WASM资格、真实宿主、可信关闭权威或发行制品。

日志：`/private/tmp/codeguard-legacy-identity-red.log`、`/private/tmp/codeguard-legacy-identity-green.log`、`/private/tmp/codeguard-legacy-identity-cli-regression.log`。归档报告及可重算命令为长期证据，临时日志不是发行依据。

最终受影响WASM六目标：53通过、0失败、0忽略。默认/WASM受影响两crate全目标严格Clippy均已通过。OpenSpec strict、分层和diff检查通过。实际归档check通过，篡改计数及重复键各退出3。完整默认suite的新源码回归仍待执行，当前不宣称全部测试验收。
