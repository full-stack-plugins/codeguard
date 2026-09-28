# 2026-09-28 迁移与补录验证

本轮仅调整文档与 OpenSpec，不修改 Rust 或插件运行时代码。开始前已有的源码改动与运行期间其他工作树变更均未回滚。

- 完整迁移 29 个原文件：11 份 Rust 设计文档及 change 文件；含迁移前未提交 verification 更新。原路径摘要见 migration-manifest.json。
- 迁入前的 108 条 requirement 及其场景全文保留；MODIFIED 分组改为新仓 ADDED，不更改条款内容。新增 WASM 12 条、npm 分发 1 条。当前共 121 条 requirement、467 个 scenario。
- 新增 24 项已验证的有界子任务、18 项 WASM 待办；父任务完成范围不扩大。
- `openspec validate --all --strict`：1 个 change 通过，0 失败。规划产物齐备不代表功能已实现；未 sync/archive。
- 全部 requirement 标题均可在 implementation-coverage.md 找到映射；迁移及主文档的本地文件链接无缺失。新增 Mermaid 仅检查源码结构，没有执行图形渲染验收。
- `git diff --check` 通过。既有回归按 implementation-baseline.md 的 28 目标命令执行，退出 0：266 通过、0 失败、46 忽略。忽略项不计原生实测；本轮没有执行全部工作区测试、真实宿主接入或 WASM 运行验收。
- npm 发布为既有验收证据，本轮未重新发布。所有迁移仍为本地未提交变更，尚未推送。

## 定向回归结果

```text
     Running tests/capability_selection.rs (target/debug/deps/capability_selection-8af7841c2955a8c1)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
     Running tests/cargo_audit_cli.rs (target/debug/deps/cargo_audit_cli-4bf3575422ef9733)
test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 7.87s
     Running tests/check_all_partial_contract.rs (target/debug/deps/check_all_partial_contract-088192c952b35449)
test result: ok. 17 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 3.82s
     Running tests/check_all_rust_native.rs (target/debug/deps/check_all_rust_native-42f418bc7deb5a58)
test result: ok. 12 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 2.33s
     Running tests/config_inspection_contract.rs (target/debug/deps/config_inspection_contract-36ffa72d2050b946)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/detect_cli.rs (target/debug/deps/detect_cli-bc2decbfb8689c89)
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/doctor_work_sync.rs (target/debug/deps/doctor_work_sync-ea7fd86680e00503)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
     Running tests/eslint_directory_cli.rs (target/debug/deps/eslint_directory_cli-cdd5810653452549)
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.02s
     Running tests/go_work_sync.rs (target/debug/deps/go_work_sync-b2f8e7850c8a8e3c)
test result: ok. 4 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1.22s
     Running tests/init_command_contract.rs (target/debug/deps/init_command_contract-a5cb341fc70780ef)
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.25s
     Running tests/java_checkstyle_workbench.rs (target/debug/deps/java_checkstyle_workbench-2b2fc167ca94ee06)
test result: ok. 7 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 0.63s
     Running tests/java_javadoc_cli.rs (target/debug/deps/java_javadoc_cli-2c4c0e4611c25027)
test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.67s
     Running tests/java_p3c_cli.rs (target/debug/deps/java_p3c_cli-70d78a0ca466f343)
test result: ok. 6 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.63s
     Running tests/lint_python_cli.rs (target/debug/deps/lint_python_cli-7b209810596190ec)
test result: ok. 12 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 1.32s
     Running tests/next_command_contract.rs (target/debug/deps/next_command_contract-13bfffab3c1a35cd)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
     Running tests/npm_audit_cli.rs (target/debug/deps/npm_audit_cli-8d8cba5a768e6574)
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.05s
     Running tests/plan_preview_cli.rs (target/debug/deps/plan_preview_cli-448cff9e48769010)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/python_cve_cli.rs (target/debug/deps/python_cve_cli-00dcaad213b45ac1)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.63s
     Running tests/rules_list_cli.rs (target/debug/deps/rules_list_cli-61d0deec4e1cee7f)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/run_report_contract.rs (target/debug/deps/run_report_contract-34c6e90c304c8cd5)
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/rust_build_cli.rs (target/debug/deps/rust_build_cli-d5bf9ca095ee7eb6)
test result: ok. 10 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 1.24s
     Running tests/rust_comments_cli.rs (target/debug/deps/rust_comments_cli-3d551a49badd151b)
test result: ok. 14 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.55s
     Running tests/task_lease_contract.rs (target/debug/deps/task_lease_contract-278e2427e9a17c1d)
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s
     Running tests/task_verify_contract.rs (target/debug/deps/task_verify_contract-8ae5c5c8173fe35d)
test result: ok. 10 passed; 0 failed; 10 ignored; 0 measured; 0 filtered out; finished in 0.64s
     Running tests/version_cli.rs (target/debug/deps/version_cli-4e728c64aebf25cf)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/whitelist_command_contract.rs (target/debug/deps/whitelist_command_contract-34308b8a9a634906)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/whitelist_propose_contract.rs (target/debug/deps/whitelist_propose_contract-a7d8b0997095723f)
test result: ok. 5 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.23s
     Running tests/work_sync_cross_category.rs (target/debug/deps/work_sync_cross_category-5f710795ff319e20)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
```
