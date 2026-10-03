# 共享后缀的语法候选路由局部验收

日期：2026-10-03。对应 OpenSpec 14.4、14.6、14.19 的低误报局部修复，父任务仍未完成。

`.m` 可表示 Objective-C 或 MATLAB，`.sc` 可表示 Scala 脚本或 SuperCollider。先写路由反例并确认失败：普通 MATLAB `plot.m` 被送入 Objective-C 候选；带 `@interface` 字符串与 `% #import` 注释的 MATLAB 文件也被宽松的字节匹配误识别。现只在 `.m` 源码行首出现 Objective-C 专属声明或 `#import` 时自动路由，`.mm` 与 `.scala` 保持明确路由；`.sc` 没有项目级证据时不猜测。显式 `grammar probe objc FILE` 仍可由用户指定语种运行。

`grammar_route` 的共享后缀、32 份资产可路由集合及 CFQuery 边界回归 5/5 通过；真实 `check all` 混合项目测试确认 MATLAB/SuperCollider 文件未产生错误候选观察，`unrouted_count=2` 且报告仍是未完成。四个有界项目的 32 份 worker 全量回归 1/1 通过，用时 54.74 秒，候选语种并集仍与固定清单一致。

后续补齐默认构建的静态发现：只读 `detect` 对 `.m` 最多读取 1 MiB 普通源码，与路由共用行首标记判别；无法读取时记录阻塞，不推断语言。无标记 `.m` 与 `.sc` 不进入错误语言清单，`unknown_conditions` 保留具体相对路径。`check all` 仍将这些路径计入候选源范围，报告 `unrouted_count=2`，且总交付未完成。真实 `detect` 回归、混合项目 `check all` 回归及仅含两份歧义源码的项目回归分别通过；后者证实 `source_file_count=2`、`status=not_run`、`reason=unrouted_source` 和 `delivery_decision=incomplete`，不会把空候选结果当成 clean。

这只减少错误语言归类和自动候选误路由；项目级 `.sc` 方言证据、无专属标记的合法 Objective-C `.m`、更强的词法判别、逐文件结构化歧义协议及原生优先/发行验收仍缺，不能宣布这两个方言已完整支持。

2026-10-03 增量：MATLAB `%{ ... %}` 块注释内可以出现行首 `#import`、`@interface` 示例。先将真实块注释样例加入共享后缀路由测试，旧判别错误把它交给 Objective-C，目标测试 RED；现在只在块注释外寻找行首 Objective-C 专属标记，块注释未闭合时也不猜测。块注释外真实 `#import` 仍可路由。`grammar_route`、只读 `detect` 和真实 `check all` 三条目标回归分别通过；混合项目继续对该 MATLAB 文件保留未解析范围，不产生 Objective-C 候选观察。这是对同一只读判别器的局部修复，尚不覆盖 MATLAB 完整词法、块注释边界与跨项目方言证据，14.4/14.6/14.19 不勾选。
