# JavaScript 函数外 return 的有界 AST 事实

日期2026-10-06，基础40cc624。延续OpenSpec12.11/14.17/14.19，父任务保持未完成。先更新模块模式规则边界规格，新的扫描测试因API缺失（E0432/E0425）失败，再实现线性游标遍历。runtime仅返回AST位置事实，不负责推断module/CommonJS或签发违规。

遍历覆盖顶层return以及if/while/try等控制流中的return；整个函数声明/表达式、生成器、箭头函数和方法子树跳过，字符串/注释文本不形成事实。记录预算1至128，节点访问预算1至200000，源码上限1MiB，耗尽保留已得事实并标truncated。避免递归栈和反复祖先查找；位置使用原始UTF-8字节与CRLF行列。

四个相关runtime目标16通过/0失败/2条件忽略。显式真实Node24.18.0条件测试另1通过：11份源码、module/CommonJS共22次语法检查，module下函数外return被拒绝，CommonJS同样源码合法；函数内部return在两种模式下合法。Node入口路径和字节前后稳定，环境清空，仅--check、不执行源码。此为自建局部反例，不是独立holdout。

WASM runtime/all-targets严格Clippy通过。公开worker、probe、项目模块模式观察和差分入口尚未接线；本次没有改变既有module_return漏检统计，没有新增报告协议或语言资格。下一步须以显式模式参数/身份将事实接入候选，拒绝未知模式自动启用，并分别保留raw/combined指标。见[证据](evidence/javascript-module-return-ast-2026-10-06.json)。

前批40cc624远端CI37431391905终态failure：MSRV成功，gate仍失败于Check out corpus evidence source，后续门禁未运行；不得以本地回归替代。
