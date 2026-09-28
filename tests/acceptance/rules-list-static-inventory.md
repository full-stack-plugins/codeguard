# rules list 静态规则目录

规范事实源：OpenSpec introduce-rust-codeguard-cli / rulepack-governance 的版本化规则包要求，任务4.7，命令C08。

Rust公开入口为 `codeguard rules list <language|all> [path] [--format human|json]`。服务复用项目静态发现和编译时固定的规则包，不启动原工具、不运行动态配置、不读取项目文件作规则包批准，不修改源码/配置/任务。Node检查器使用旧登记明确的TypeScript生态映射。

目前只有Ruff F401/E501的已知候选映射；来源、许可、版本、兼容版本及原字节摘要均来自已有严格解析器。项目配置声明configured不意味着这两条启用：具体规则固定enabled=unverified、execution=not_run、approval=unverified；不虚构原生严重度或已批准例外。所有所选语言的六类目录缺口均保留，空目录不表示无检查义务。协议rules-inventory:0.1.0为只读静态视图，固定incomplete/退出3、gate_effect=none、delivery_decision=not_evaluated。未知语言或非法参数为2，内置登记/映射损坏为4。

TDD：原入口只接受rules whitelist，新增5项测试初次为1通过/4失败（合法请求被usage拒绝）。实现后5项通过。增加Node配置归属断言时再现一项失败：原生checker_id使用node前缀，不能按typescript前缀过滤；已改为明确生态映射。随后增加human/help与机器协议验收。

验收入口：`cargo test -p codeguard-cli --test rules_list_cli --test config_inspection_contract --test whitelist_command_contract`。本条覆盖只读配置声明、候选不伪装为有效规则、同名项目映射和本地自批不授权、动态ESLint配置无副作用、all保留57种语言与目录缺口、非法参数/格式、缺项目范围及human/help。

实际二进制Python/TypeScript/Java/all的四份JSON输出已通过Draft202012 schema验证；将authority、delivery_decision、exit_code、tool_lock篡改为批准/通过的四种变体被拒。用于独立协议验证的Python不参与Rust产品执行路径。

当前完整有效规则目录、可信工具锁/政策、原生生效规则/配置闭包、suppression差异、批准例外及跨平台/宿主仍缺；不把该入口阶段验收提升为整个4.7或全计划完成。最终测试和Clippy结果以OpenSpec verification.md的本轮终态为准。

当前终态：规则目录6项、配置解释6项、白名单查询8项共20项通过/0失败/0忽略；fmt及新源码全工作区all-targets Clippy -D warnings均退出0。新增前880项工作区通过不能冒充本入口新增后的全工作区运行结果。
