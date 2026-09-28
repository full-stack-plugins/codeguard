# npm审计声明的发现与对话反馈（2026-09-27）

`detect` 逐package.json构建根返回node.npm.audit的明确声明、未观察到脚本、自定义调用及畸形脚本四种状态。只读解析，不执行项目脚本；缺脚本不证明CI/外部流程未配置。重读清单摘要不匹配或不可读时标记unknown及观察未完成，不沿用旧声明。

`plan cve typescript`及`plan cve all`保留各根声明、仅为明确声明生成待确认执行候选。修复此前以checker ID首段等于语言ID筛选而漏掉node检查器的问题；现node检查器与注册表的typescript（包含JavaScript）对应。质量仍为not_evaluated，命令候选为null，不声称已经运行。

`check all`的human反馈显示npm各构建根、原因及下一步，JSON沿用发现契约。init沿用共享发现链，但未声称npm专有初始化行为已完整验收。

新增binary测试先因无npm发现失败，再因计划筛选遗漏失败，修复后通过。受影响发现24项、清单变化1项、计划3项、初始化41项及新流程1项通过（70项不同用例）；Clippy及格式通过。固定脚本副作用标记未出现，detect/plan/check未初始化工作区。

尚未自动执行npm审计、核对包管理器/bun/pnpm/yarn、外部CI声明、工具锁及库时效；公开CVE命令、稳定任务和正式门禁仍未完成，7.3/7.5不勾选。
