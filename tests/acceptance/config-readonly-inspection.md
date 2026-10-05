# C09/C10 本地配置只读观察切片

命令：`codeguard config validate|explain [path] [--policy-candidate FILE] --format json`。

本记录描述历史旧配置切片；当前 0.3 原生配置扩展另见[验收记录](config-native-observation.md)。历史切片只读取项目根的旧 `codeguard.json` 和 `codeguard.lock.json`。旧字段 `exclude`、`gate_scope=delta`、`java.commands` 仅计数并产生待迁移诊断，不会被解释成批准的扫描排除、工具选择或白名单。未知/损坏旧字段、损坏工具锁、符号链接及超大文件均为未完成。`codeguard/decisions/` 的本地文件不能自授批准；当前命令不把它们作为有效策略输入。

即使两个文件结构正确，可信质量策略和白名单批准来源仍未接入，`effective_policy=null`、`authority=unverified`、`gate_effect=none`、`quality_decision=not_evaluated`、退出 3。此切片不执行项目脚本、不写项目文件，也不宣称完成 C09/C10 的规则引用、原生配置差异或批准来源验证。

显式 `--policy-candidate` 可只读检查 1.0 候选协议：必需检查不能空，语言与检测族必须登记，规则/源码集合/阻断级别不能空或重复，工具锁引用按原始字节摘要比对。源码排除只容许精确相对文件路径、该文件 SHA-256、结构化原因和有限到期字段；通配、路径穿越、自写 `approved=true`、重复义务或排除均拒绝。工具锁引用匹配仍只代表两个**本地字节**相符，不代表受保护批准、扫描覆盖或候选已生效。

验收：`cargo test -p codeguard-cli --test config_inspection_contract --test quality_policy_candidate_contract --offline`。反例涵盖旧排除/自定义命令、未知字段、坏工具锁、伪造本地批准、符号链接、空必需集合、重复义务、通配排除和工具锁字节失配。正式策略绑定、精确白名单生效与门禁验收仍在 OpenSpec 4.1、4.2、4.5、4.7、4.9、12.12。
