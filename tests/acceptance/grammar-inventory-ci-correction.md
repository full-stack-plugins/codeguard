# grammar 库存断言与 CI 失败纠正

日期：2026-10-06。对应现有 OpenSpec S12.1 / S14.1 / S14.9；父任务保持未完成。

远端运行37381194325（e0e0f8a）和37379671502（e9f1620）均在默认全工作区的grammar_status_cli失败：测试仍要求CFQuery限制包含“does not validate full SQL semantics”。当前固定清单已根据PostgreSQL18.6对照区分SELECT DISTINCT FROM users被拒绝与SELECT FROM users被接受，库存输出正确投影了具体方言限制。该故障是旧断言与当前证据不一致，不能据此改回泛SQL裁定或消除真实限制。

当前源码默认目标也实际复现同一RED。修改后保留库存32份候选、0份资格、execution=not_run、delivery_decision=not_evaluated与只读权威断言；新增全部32行known_limitations逐字节等于固定清单的核对，并明确断言PostgreSQL版本、两份SQL的方言差别、原生确认及其它方言仍未验收。此修改没有调整grammar、来源清单、报告协议、规则或资格。

CI新增前置步骤：保留无WASM二进制后立即运行默认/WASM两种库存检查，再进入完整解析器和npm检查。原完整suite继续保留，没有删除失败目标或改变退出要求。

实际局部验证：默认库存2 passed；WASM库存与CFQuery结构/方言三个目标8 passed / 1 ignored。忽略项需要显式Docker原生对照，本轮没有重新取得该原生结果。当前源码默认全工作区及静态检查终态追加于下。

## 当前源码完整默认回归

`cargo test --locked --workspace --all-targets` 实际结束，271个目标组共1500 passed / 0 failed / 132 ignored。忽略项保留原外部工具/显式回放条件，不计通过；WASM功能未启用，未执行或改动未提交的Erlang草稿。该结果不是完整WASM suite、原生工具独立标签、真实宿主或发行验收。

实际当前debug二进制的库存Schema四项正反例通过，包括伪造资格、交付通过、缺语言与移除限制的拒绝。Workflow YAML及前置default/WASM步骤解析通过，OpenSpec strict、crate分层和差异检查通过。远端验证须由修复提交的新运行单独确认；旧失败记录不改写。

本轮日志身份：

- `/private/tmp/codeguard-grammar-status-ci-red.log`：SHA-256 `b672561f38bfd15bf3d0045e9cdf97764192f7770ba904c2deee424ec5e86ee5`。
- `/private/tmp/codeguard-grammar-status-ci-default-green.log`：SHA-256 `a1c977d758d4c6d90d829b962df3757d3e2ce8f594e5bef797dea3966c7d289f`。
- `/private/tmp/codeguard-grammar-status-ci-wasm-green.log`：SHA-256 `8a20e3d6288a4818e235c2baac6faa8ca7748d1b5d85f945971c47e85fce69c5`。
- `/private/tmp/codeguard-default-workspace-grammar-status-green.log`：SHA-256 `96825ee62e6dc3937d66aa9d442cfda45ea0214bc9f08835e4c934e9a5620a68`。
- `/private/tmp/codeguard-grammar-status-ci-schema.log`：SHA-256 `ae49447e17d26125783856ec2e3736c59b5384793aaf0501e58078b372be5617`。

CLI的WASM all-targets严格Clippy与所改Rust文件格式检查通过。Clippy日志SHA-256：`cbca290f89940a729d05a349d0a1a3bf34a2ff5f3678cc190260e921b8c1a8f9`。
