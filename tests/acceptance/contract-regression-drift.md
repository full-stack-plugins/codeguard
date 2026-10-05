# 契约漂移回归验收

日期：2026-10-05。规格事实源为 introduce-rust-codeguard-cli；对应签名期限约束、Swift 工作台接线及 S12/S13 回归门禁，父任务仍开放。

对提交 15dcbbd 运行完整默认 workspace/all-targets，在第172组遇到签名处置两个失败，之前累计962 passed / 2 failed / 97 ignored，测试在此中止，不是全套验收。旧正例要求候选到期300裁剪成签名到期200后接受，与已批准的越界拒绝约束相矛盾。现用200及180验证合法边界，另以300明确断言 approval_candidate_lifetime_outside_signature。身份漂移、撤销与过期反例改用合法候选，确保它们实际到达各自拒绝条件，而不是因候选期限先失败。生产绑定器没有放宽。

Linux CI 37217854535 的 Hook 测试期待初始化 Swift 报告0.43及not_connected，但实际是0.44和已连接工作台。本机定向复现同一失败；断言现要求0.44/synced_partial，并增加缺编译器时不伪造原生task_id的检查。WASM任务及next/Hook仍复用同一稳定ID，已有零位置及原生确认指引断言保留。

两个修正目标最终19 passed / 0 failed / 0 ignored（Hook15、签名4）。完整默认套件修正后的重跑另行验收，不能据此声称全部回归或新CI通过。新增缺失投影恢复场景只是既有可重建要求的具体化，尚未实现；没有发布或改变grammar资格。

## 可重跑命令

```bash
cargo test -p codeguard-cli --features wasm-precheck --test hook_syntax_tasks --test signed_disposition_binding_contract
cargo test --workspace --all-targets
```

## 原始日志摘要

- `/tmp/codeguard-15dcbbd-workspace-default.log`：`19efc1641af940a1f3f0685993784a74f73eb766484247310c54b3cd98b42e4d`。
- `/tmp/codeguard-15dcbbd-hook-protocol-red.log`：`fcb4d5a33c20987ec6b2efcd196df57337ec6a495c5b2849e5f0f2ba74f4ab98`。
- `/tmp/codeguard-contract-drift-green.log`：`909f5506ea8a6c39790623b3d56eb5025f3b3557a6a1d353be1a070902a0ef88`。

## 修正后的完整默认终态

0a3e6a3 对应修正后默认 workspace/all-targets 的实际重跑已退出0：230组、1278 passed / 0 failed / 113 ignored。日志 `/tmp/codeguard-contract-drift-workspace-default.log`。此结果属于投影恢复实现之前，不用于证明后续源码改动已通过全套。忽略项仍未验收，完整 WASM suite、语法资格、真实宿主、可信关闭及公开发行不因此完成。

2026-10-05 实时核验：[CI 37218637535](https://github.com/full-stack-plugins/codeguard/actions/runs/37218637535) 的 headSha 为 `0a3e6a3e3c90f213e84ad43f4189d20cf94cb1bc`，gate 与 msrv 均 completed/success。Linux WASM 资源边界、32 grammar 回放、离线 npm 包验收和完整测试步骤均成功；Rust 1.85 默认及 WASM targets 实编成功。该结果不覆盖当前未提交的任务投影恢复改动，也不升级 grammar 资格或真实宿主验收。
