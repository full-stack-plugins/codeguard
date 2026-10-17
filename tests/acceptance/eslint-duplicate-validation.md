# ESLint 重复诊断的全量验证

对应 OpenSpec：`introduce-rust-codeguard-cli` 的 remediation-workflow，稳定 ESLint 原生任务投影要求。此记录验证投影边界，不代表 ESLint 项目全量验收或真实宿主验收。

## 问题与修复

投影原先先按规则、行、列和严重性去重，再验证剩余诊断。同键的异源路径或超预算消息若排序在合法诊断之后，会被去重丢弃，导致无效完整输入生成部分任务。新增反例在原实现实际失败。

现在先验证全部诊断的归属、规则、严重性、消息预算及源码定位，再稳定排序、去重、生成任务。保留原 fingerprint 协议、合法重复诊断消息选择、UTF-16 列、行终止符及脱敏消息摘要。此修复不改变原工具诊断与源码违规判定；原生 JSON 解析器已有的输入验证继续保留。

## 实际验证

- RED：`duplicate_diagnostics_cannot_hide_invalid_ownership_or_message` 实际失败。
- GREEN：同一反例覆盖异源路径及 4097 字节消息，各自两种输入顺序；全部拒绝任务投影。
- 六个目标：`eslint_finding_identity`、`eslint_lint_cli`、`eslint_directory_cli`、`check_all_eslint`、`work_sync_contract`、`work_sync_cross_category`；40 passed / 0 failed / 8 ignored。忽略项含外部原生工具条件，不计通过。
- `cargo clippy --locked -p codeguard-adapters --all-targets -- -D warnings` 通过。
- OpenSpec strict、crate layering、所改 Rust 文件格式化及 `git diff --check` 通过。

没有执行或修改未提交的 Erlang 原生差分草稿。未重跑全工作区、WASM 全量语料或真实 ESLint 外部条件测试，未将父任务标为完成。
