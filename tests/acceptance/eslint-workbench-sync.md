# ESLint 局部工作台连接验收

公开单文件 lint typescript 增加显式 --workspace ABS_PATH，要求已有 init 工作区，不隐式初始化。Rust 将摘要绑定的脱敏观察保存到 codeguard/reports，并通过既有同步协议生成 finding 与 Markdown 任务，再将 next 或 next_error 回传对话。反馈协议升级至0.2.0；未指定工作区仍只输出局部观察。

报告严格限制字段、版本、工作区、原cwd、Node/ESLint入口/配置/源码路径与摘要；导入前复核当前字节和稳定任务投影。报告不能自称allow或覆盖完整。已消费报告按不可变字节收据复用，避免源码修复后把历史报告误判为新坏报告。私有原生临时输出仍在退出时清理，持久记录只有脱敏结构化观察，不能声称保存完整原生证据。

同一规则及锚点重复扫描只建一个稳定任务，保留用户备注及勾选文字；七项任务内容包含问题证据、规则依据、允许范围、修复步骤、复检、历史尝试和关闭条件。next核对最新已消费观察与首次工具/原配置连续性；源码或配置变化、原生抑制、未完成和局部零诊断要求复检，不自动关闭任务。status/task show展示同一开放任务，不能签发质量通过。

新回归明确复现配置变化后的指引缺陷：旧next fallback选择Python，断言typescript先失败，修正为ESLint上下文待核验参数后通过。复检指引不能猜原工具身份或要求修改无关源码。

33项普通相关回归通过：身份4、CLI4、next6、status/show4、同步15。CLI工作台契约还覆盖带作用域插件规则ID、重复扫描、用户备注、配置变化和伪造allow报告拒绝；插件ID只是协议夹具，不能证明实际TS插件执行。补充status/task show联通用例通过。当前公开反馈、actionable简报及失效配置简报均通过Draft202012 schema验证。目标Clippy及fmt以最终检查结果为准。

真实原生复扫单独使用本机既有Node24.18.0和ESLint10.11.0，经公开CLI在原配置下先发现no-debugger、修复源码后零诊断；测试必须显式运行ignored用例，不能用受控Node夹具代替。最终执行结果记录在OpenSpec verification，局部反馈仍coverage_proven=false、delivery_decision=not_evaluated，任务保持开放。

Node环境阻塞任务、完整原生证据保留、正式task verify关闭/重开、全项目与多文件覆盖、TS插件/tsconfig/配置导入闭包、依赖/CVE、安全策略及真实宿主门禁仍未完成。OpenSpec7.3/9.7继续不勾选。
