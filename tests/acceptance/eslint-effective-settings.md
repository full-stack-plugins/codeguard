# ESLint 原规则有效配置复检

任务复检在同轮局部扫描后，使用相同显式 Node、ESLint 入口、原配置、原工作目录及共同截止时间调用原生 `--print-config FILE`。查询前后核对源码、主配置、Node 和入口文件字节；反馈只保存原规则级别、查询原因、输入一致性与输出摘要，不向对话投影原始配置。它仍是未批准的局部观察。

新 ESLint task recheck 协议为 0.2.0，增加 effective_rule；历史 0.1.0 继续只读识别，不升级为批准。规则级别 0 或未出现时，零诊断返回 rule_coverage_requires_review；查询损坏或执行未完成不能构造消失候选。原稳定问题仍检出时保留 still_present；原生抑制另行核查，任务均保持 open。next 与 human task verify 显示原因和下一步。

原生查询参考本机既有 ESLint 10.11.0 的 cli.js/options.js。实际 Node 24.18.0 与 ESLint 经统一 Rust runtime 查询启用、off、缺规则三种配置，38.50 秒通过；原生忽略目标输出 undefined，保留 target_not_selected。公开 CLI 的真实反例只修改主配置导入的 mode.cjs，将规则从 error 变为 off，源码与主配置字节均不变；108.38 秒通过，原上下文摘要仍一致但复检转规则覆盖核查，next 提醒未启用原规则，事件保存且源码未改。没有安装、升级或批准白名单。

44 项普通相关回归与分类单元用例通过，最终规则解析和 ESLint CLI 9 项普通测试、目标 Clippy -D warnings、fmt 通过。配置解析拒绝重复键、非法严重度与坏结构，固定 print-config 参数拒绝多文件范围。统一 task verify 与原生复检 schema 已用受控 CLI 产物核对；简报实测暴露历史 lint-only run_id 正则，已允许 eslint-task 前缀，后续校验结果见对应 OpenSpec verification。

四个显式输入一致不证明完整 JS 导入/插件闭包、全部环境、TS parser/项目源集或受批准 required rule 集合。规则恢复后零诊断仍仅为局部候选，不能正式关闭或签发门禁；7.3/9.7 不勾选。
