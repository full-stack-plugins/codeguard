# JavaScript 项目声明模式观察

日期2026-10-06，基础e652a4b，延续12.11/14.9/14.17/14.19，父任务未完成。新观察服务是自动启用模块规则的前置证据接口，不是新CLI入口。本批尚未改变现有lint/check/Hook行为。

物理root和无跳级相对路径校验后，读取普通UTF-8源码并绑定摘要。`.mjs`明确module，`.cjs`明确CommonJS，不读包配置；`.js`在工作区内搜索最近package.json（64目录上限、256KiB清单上限），仅唯一显式type=module/commonjs返回已声明模式。最近清单缺type、坏JSON/重复字段/非对象/非法type/链接/超预算保持unknown，不能向外层继承或越过用户选择的root。`.jsx`等loader相关模式未推断。

结果0.1保留来源、源码/包摘要、搜索目录负向依据及具体原因，native_execution=not_run、delivery_decision=not_evaluated。源码改变、近包创建/删除/内容改变使两次观察不等；在未来worker前后必须比较这些证据，不能把静态声明当有效ESLint配置或原生完成。未知模式未造源码finding，不给任务关闭。

新API缺失E0432的RED后实现；默认四组测试4通过/1条件忽略，WASM三个相关目标11通过/1条件忽略（含重叠的模式测试），增加真实外层祖先包边界后默认目标完整重跑仍4通过。显式已有Node24.18.0条件目标另1通过：5份真实文件涵盖mjs/cjs、根module包、嵌套commonjs包与缺type未知包；--check按文件路径运行，不执行源码，输入/模式证据和Node入口字节前后一致。Node接受缺type的样例不使静态unknown变成批准。

21份实际观察逐成员通过封闭schema；错误模式/后缀、缺摘要、伪造原生执行/allow、未知字段拒绝；402份schema元定义有效。默认与WASM CLI/all-targets严格Clippy、分层/OpenSpec/diff通过。见[证据](evidence/javascript-project-mode-2026-10-06.json)。

自动扫描消费者、模式证据持久化、稳定任务/复检/对话与跨平台/独立holdout仍未完成；正式语言资格0/32、不更改已有358及额外module语料指标、不发布npm/插件/市场。
