# Javadoc 项目工作台局部验收

对应 introduce-rust-codeguard-cli C15及9.x。已初始化项目JDK模式原生诊断→报告→稳定任务→next指引接线完成；Maven多文件、文件入口工作台、task verify与可信关闭仍未完成，不勾选父任务。

TDD 新增两项行为测试初始均失败，入口没有workbench。接线后两项通过；新增保存失败恢复及伪造指纹拒绝。五个受影响回归目标合计46通过/0失败/17条件忽略，覆盖Java注释、旧Checkstyle、work_sync、跨类别和status/show。

真实已有JDK21项目验收单独1通过/0失败/0忽略（是上述忽略项之一，不重复计数）：缺注释产生开放任务；给类及公共构造函数补齐文档后局部零诊断，所有旧任务仍open。未安装、下载或发布。该项不证明可信关闭或宿主反馈。

3份新schema元定义通过；配置完整和缺配置两轮真实CLI包装/next报告通过对应schema，保存的工作台观察通过schema，伪造覆盖被拒绝。跨文件引用登记使用本地schema registry；早期旧RefResolver解析URN下片段失败，改用referencing Registry后校验通过，没有降低schema约束。

默认CLI全目标Clippy -D warnings、分层、OpenSpec strict通过。仅同步当前源码和配置身份有效的局部报告；身份伪造/变化拒绝导入，不把未完成原生检查转成源码问题。缺配置记录为准备建议，不自动产生交付义务。稳定任务按规则/相对文件/源码锚点/序号归并；行号只定位证据。

尚未验收：全WASM、真实宿主、跨平台，以及Javadoc完整项目规则归因、工具可信关闭与复发。本增量没有执行用户Erlang草稿。
