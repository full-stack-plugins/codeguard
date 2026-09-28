# ESLint 原生字面命令计划验收

EslintCommand 显式携带 Node 二进制、ESLint JS 入口、原 flat config、完整源码列表、本轮独立 JSON 槽位及可选 max-warnings。args 只构造传给 Node 的 OsString 参数，不启动进程或读取配置。

固定命令形状：Node 的 -- 分界、原 ESLint JS 入口、--no-config-lookup、--config 原配置、--format json、--output-file 独立报告、可选 --max-warnings 原整数、ESLint 的 -- 分界与全部显式源码。没有 npx、Shell、fix、quiet、cache 或 no-ignore，不隐式改写配置、安装工具或删除 warning。来源/忽略配置的有效语义仍需原工具验证。

## 验收证据

API 缺失先编译失败，实现后四项契约测试通过：

- 原配置及带空格/Shell 元字符的多文件路径原样为单独 argv。
- 警告阈值是显式原生整数，不改规则/范围。
- 相对/控制字符/原始 dot 或 parent 段、空/重复源集，以及 Node/JS/config/source/report 任意词法冲突启动前拒绝。
- 源码上限 10,000；单路径最多 4096 UTF-8 字节，全部 argv 预算 128 KiB。超限不能计划成已检查的干净范围；未来调度器需要完整分批归并而非漏文件。

显式 Node 24.18.0 通过统一 runtime 执行测试用 argv 观察脚本，完整转发子工具参数、配置和源码原字节保持不变，$(touch injected) 未被 Shell 执行。该用例 0.05 秒通过；只证明 Node 参数分界及 runtime 字面调用，不是 ESLint 原生执行，不证明 flat config/plugin/TS parser 生效。

## 当前边界

命令不核验物理符号链接/硬链接别名、工具/Node 包及插件闭包或报告新鲜性；这些必须由后续执行上下文在启动前完成。CLI 接线、有效项目配置、真实 ESLint 正反例、TypeScript/monorepo 与依赖审计尚未完成；OpenSpec 7.3 不勾选。未安装或升级工具，ESLint 原生执行仍 NOT_RUN。
