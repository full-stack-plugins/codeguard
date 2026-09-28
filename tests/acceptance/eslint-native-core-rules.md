# ESLint 原生核心规则与复检验收

使用本机既有 npm 缓存中的 ESLint 10.11.0 和显式 Node 24.18.0，通过 Rust 统一 runtime 执行原生入口，不安装或升级工具。测试原 flat config 为项目自有 CJS，开启 no-unused-vars warning 与 no-debugger error；不导入外部插件。每轮绑定 Node/入口/配置/源码摘要、原生版本、本轮独立私有报告槽位及60秒共享预算。

六轮场景：干净源码、真实未用变量warning、真实debugger error、语法解析失败、原生eslint-disable抑制、移除问题与抑制后的相同原配置复检。warning/error保留规则与位置；解析失败要求调查；抑制不得签发干净范围或白名单批准；复检零诊断仅为局部一致观察，不能自动关闭持久任务。

这是真实 ESLint 执行，区别于已有 Node JSON 夹具。它不证明全项目源集、TS parser/tsconfig、本地插件/依赖导入闭包、配置选择、可信工具/规则包、正式CLI及任务门禁覆盖，OpenSpec7.3保持未完成。缓存制品来源和依赖闭包未作为可信批准核验，不将本地摘要当作白名单授权。

原生六轮测试104.56秒通过。普通相关回归68项通过，CLI相关Clippy、fmt、OpenSpec strict与插件git diff --check通过。
