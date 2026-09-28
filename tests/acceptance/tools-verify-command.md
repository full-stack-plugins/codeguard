# tools verify 只读入口验收

对应 OpenSpec S05 5.2/5.6 的局部接线，未完成正式工具准备验收。

```bash
codeguard tools verify . --format json
codeguard tools verify . --tool-lock-candidate FILE --managed-cache ABS_PATH --runtime jdk=ABS_PATH --format json
```

默认读取项目 codeguard.lock.json 候选。使用运行时有界普通文件读取（256 KiB），拒绝链接锁，严格解析协议并记录候选字节摘要。当前平台工具按稳定顺序复用 verify_locked_artifacts，分别核对入口、运行时及目录包；反馈结构化 issue、版本声明状态与具体恢复方向。工具路径、锁原文和原生诊断不进入报告。human 从同一 JSON 投影并对字段转义。

锁来源尚未核验，固定 inspection_status=incomplete、authority=unverified、readiness=unknown、gate_effect=none，退出 3。匹配仅 matched_untrusted，execution=not_run、version_status=lock_declared_only，不证明可启动、完整动态闭包、规则执行或安装版本。其它平台工具不生成本机制品结论。缺/坏/不可读锁没有源码 finding。

8 项 CLI 用例覆盖 wrapper 副作用未执行、缺文件/失配/不可执行、缺/坏/链接锁、独立运行时与 bundle 缺口、受管缓存/显式运行时、跨平台条目、安装/重复/相对参数拒绝及 human 标签转义。此前命令缺失，4 个最初用例因无 JSON 输出失败；实现后通过。跨平台新增用例首次用了非协议平台名，纠正为正式平台清单中的另一个平台后通过，未放宽解析。原静态身份 8 项、配置 6 项和边界 4 项回归通过。mock 入口不当作真实 doctor 运行证明。

tool-artifact-inspection.schema.json 0.1 限定无批准/执行/门禁效果。两份实际 CLI 缺锁/摘要匹配输出通过校验，伪造 authority/readiness/gate_effect/inspection_status/execution/version_status 的 6 项反例拒绝。workspace all-target Clippy、格式、OpenSpec strict 与 diff 检查通过。

不执行工具、不联网、不安装、不写 codeguard/ 或修复任务。完整 tools list/install、可信锁来源、原生版本及兼容诊断、doctor、准备报告持久化与幂等同步仍缺；5.2/5.6 不勾选。完整计划继续推进。
