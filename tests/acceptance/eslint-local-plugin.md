# ESLint 项目本地插件原生验收

使用本机既有 Node 24.18.0 与 ESLint 10.11.0，经公开 Rust CLI 加载项目原 flat config 及其导入的本地 guard.cjs。插件以 workspace 命名空间声明 no-debugger 规则，由真实 ESLint 遍历 DebuggerStatement 并生成原生诊断；不以协议脚本冒充该工具。

首次 lint 把完整 workspace/no-debugger ID 保留到对话与稳定任务。源码从 debugger 改为普通调用后，公开 task verify 按相同 Node/入口/主配置/cwd 复扫，并用原生 print-config 观察该插件规则仍为级别2，返回 candidate_absent_unverified_policy，保存事件；插件文件与主配置字节保持原样，status 的门禁仍未评估。测试91.56秒通过，无安装或升级。

它证明局部项目插件诊断及相同配置的原工具复检可以穿过公开CLI和任务链路，不证明完整插件/JS导入闭包已绑定或获批准，也不证明TypeScript parser/tsconfig、monorepo完整范围、依赖审计、完整required规则或正式关闭/重开。7.3/9.7保持未完成。
