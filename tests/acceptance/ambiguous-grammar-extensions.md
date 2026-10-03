# 共享后缀的语法候选路由局部验收

日期：2026-10-03。对应 OpenSpec 14.4、14.6、14.19 的低误报局部修复，父任务仍未完成。

`.m` 可表示 Objective-C 或 MATLAB，`.sc` 可表示 Scala 脚本或 SuperCollider。先写路由反例并确认失败：普通 MATLAB `plot.m` 被送入 Objective-C 候选；带 `@interface` 字符串与 `% #import` 注释的 MATLAB 文件也被宽松的字节匹配误识别。现只在 `.m` 源码行首出现 Objective-C 专属声明或 `#import` 时自动路由，`.mm` 与 `.scala` 保持明确路由；`.sc` 没有项目级证据时不猜测。显式 `grammar probe objc FILE` 仍可由用户指定语种运行。

`grammar_route` 的共享后缀、32 份资产可路由集合及 CFQuery 边界回归 5/5 通过；真实 `check all` 混合项目测试确认 MATLAB/SuperCollider 文件未产生错误候选观察，`unrouted_count=2` 且报告仍是未完成。四个有界项目的 32 份 worker 全量回归 1/1 通过，用时 54.74 秒，候选语种并集仍与固定清单一致。

这只减少自动候选误路由；静态发现仍可能按旧语言注册表把 `.m/.sc` 归入 Objective-C/Scala，自动报告也仅给未路由总数。项目级语言证据、未路由文件逐项诊断、`.m` 中不带标记的合法 Objective-C 范围及原生优先/发行验收仍缺，不能宣布这两个方言已完整支持。
