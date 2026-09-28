# task verify 本地租约绑定验收

状态：Unix 本地协作切片；不证明正式任务关闭或完整交付门禁。

`cargo test -p codeguard-cli --test task_verify_contract --offline` 的普通样本验证：未占用时 verify 自动领取并释放自有租约；已有占用时无凭据请求和错误 token 在原工具扫描前退出，正确 token 可借用并保留原领取者租约。`task_lease_contract` 还验证未结束 attempt 阻止 verify。实现会在扫描后、报告与复检事件持久化前重新核对任务、owner、token、generation、期限和未结束 attempt；失租不写复检事件。

本机 Ruff 0.16.8 的显式原生测试 `CODEGUARD_RUFF_BIN=/Library/Frameworks/Python.framework/Versions/3.13/bin/ruff cargo test -p codeguard-cli --test task_verify_contract --offline -- --ignored` 通过 6 项，其中新增真实 finding 的错误 token 拒绝/正确 token 借用场景。错误 token 响应的 `native_scan=null`，报告数量不增加；正确 token 复检得到 `still_present`、保存观察事件，租约保持 active。

剩余：长原生操作的自动续租、失租后的原生子进程取消、Windows 同等互斥、可信工具/规则/覆盖身份及 resolved/reopen 状态机尚未实现。所有局部响应仍退出 3、`local_unverified`、`not_evaluated`。
