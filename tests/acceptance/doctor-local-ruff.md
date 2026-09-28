# doctor 局部原生诊断入口

对应 OpenSpec 5.2/5.6 的局部接线，完整准备流程仍未验收。

```bash
codeguard doctor . --format json
codeguard doctor . --ruff-tool ABS_PATH --timeout 2m --format json
CODEGUARD_TEST_RUFF=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --test doctor_cli --test native_version_observation --test init_command_contract --test tools_verify_cli --test crate_boundaries --offline
```

默认只读发现检查配置，未选择工具为 not_selected，不能伪称缺失。显式绝对路径 Ruff 才执行固定 --version；清空继承环境、关闭 stdin，在 0700 私有临时目录中调用统一 runtime 观察，版本必须 Ruff 0.16.8，前后核对工具字节。默认总预算 2m，单探测至多 10s 且不超过剩余预算；静态发现尚非硬预算覆盖。脚本入口缺隔离启动前 incomplete；Mach-O/ELF 入口只是格式筛选，不证明批准、完整动态闭包或网络隔离。

配置与原生版本通过同一 human/JSON 输出。失配保留具体 reason 和恢复方向，未选择/缺文件/脚本/未知入口/版本故障不同。版本成功仅 observed_untrusted，策略来源未核验，readiness=unknown、交付 not_evaluated、quality_checks=not_run、gate_effect=none，正常退出 3，取消 130。工具与临时路径、原生文本不出现在公开报告；日志仅临时私有保留，未导入工作区，persistence=not_saved。

入口缺失时三个行为用例因无 JSON 输出失败；实现后目标用例覆盖默认配置/未选工具、缺工具/脚本不执行、真实 Ruff 版本调用、参数拒绝、错误项目/未知入口和 human/JSON 一致。显式真实 Ruff 用例在本轮实际执行，不把未配置环境时的条件跳过视为真实证明。没有安装、写 codeguard/、执行项目 wrapper 或源码扫描。

doctor 6 项、版本观察 7 项、初始化 41 项、工具核验 8 项、边界 4 项共 66 项通过，all-target Clippy、格式和 OpenSpec strict/diff 通过。四份实际输出及项目摘要通过 doctor 0.1/现有 init 摘要 schema；7 项伪造批准、准备/质量/交付完成、门禁、保存或必需策略反例拒绝。

其它工具/运行时、可信锁、必需前置映射、完整预算/隔离、持久准备报告、work sync/next 与任务关闭仍缺，5.2/5.6 不勾选。此输出是 doctor_observation 0.1，不是正式 PrerequisiteReport 或门禁结果。完整计划继续推进。
