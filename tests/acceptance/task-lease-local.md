# 本地任务租约局部验收

`task claim/heartbeat/release` 当前只在 Unix CLI 接入，使用同工作区每任务文件锁保护状态替换。claim 生成 32 字节随机 token，只在响应中返回；持久状态记录其 SHA-256、owner、任务、工作区、generation 和 5 分钟期限。续租/释放同时核对 owner 与 token。释放重试在同一代内幂等；下一次领取换 token 并增加 generation，旧 token 不能续租或释放新租约。成功退出 0 只表示本地协作状态更新，质量决策始终 `not_evaluated`。

目标测试覆盖四进程并发领取只有一个成功、同名 owner 旧 token 失效、错误 owner/token、过期后新代领取、无效任务不创建锁、符号链接租约文件不被覆盖，以及锁已被持有时立即返回未完成。`task_lease_contract` 6 项测试通过。跨机器全局独占、未结束 attempt 的过期恢复、受控 fix 和 task verify 租约绑定、Windows 实现及可信策略门禁均未验收。
