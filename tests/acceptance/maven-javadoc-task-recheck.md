# Maven Javadoc 原任务复检局部验收

延续 introduce-rust-codeguard-cli 的6.x/9.x。原任务 `task verify` 已接通首次报告摘要、工作区、构建根、原POM、Maven/JDK及离线仓库身份，重新发现所属Java源码并执行既有原生多文件探针。缺前置或身份不匹配不改用JDK单文件检查；配置或源集变化返回覆盖复核，范围外源码阻塞保持。报告0.1、公开复检0.28、Maven包装0.6、内部简报0.5；历史schema不改。每次局部复检记录原任务开放事件，兼容租约、尝试与next历史失效。

初始目标测试因task_checker_unsupported失败；接线后因重构Result类型歧义编译失败，明确返回类型后通过。尝试历史接线因遗漏Sha256引入失败，改用已有digest辅助函数后恢复。目标六项全部通过，覆盖源码/POM/JDK运行中变化、缺工具准备、重复扫描稳定身份、原任务仍存在/缺工具/工具身份变化/局部消失/POM变化/新增源码、范围外阻塞、伪造覆盖和指纹拒绝、租约下两次无进展后needs_decision。失败记录保留，不计为通过。

八个受影响回归目标110通过、0失败、22条件忽略。没有执行条件原生验收，没有执行或更改用户Erlang草稿。实际Maven进程为受控夹具，不是本轮真实Maven插件验收；不执行安装、下载或发行。

```bash
cargo test --offline --locked -p codeguard-cli --test check_all_java_p3c --test java_comments_cli --test maven_javadoc_input_stability --test status_show_contract --test task_lease_contract --test task_verify_contract --test work_sync_contract --test work_sync_cross_category
cargo clippy --offline --locked -p codeguard-cli --all-targets -- -D warnings
```

五份新schema元定义、17份实际输出（诊断/准备包装与简报、六类公开复检及内嵌容器、next）、六个伪造coverage变体通过预期校验。聚合0.57只有元定义验证，未取得嵌入新Maven简报的实际输出证明。真实check java捕获的0.38聚合优先选择P3C配置准备任务；校验失败，旧schema不接受该简报，作为独立缺陷待修复，不算本批协议通过。CLI严格Clippy、分层与OpenSpec strict通过。

证据与日志摘要见 evidence/maven-javadoc-task-recheck-2026-10-06.json。未完成：真实Maven新工作台/原任务复检验收、复杂生效模型、完整范围和可信关闭/复发、真实宿主与发行；聚合P3C简报协议缺口。父任务仍开放。
