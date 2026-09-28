# ESLint 静态配置发现验收

Rust detect 及 init 现在观察 ESLint 六种 flat config：js/mjs/cjs/ts/mts/cts，并保留旧 eslintrc 文件和 package.json eslintConfig 线索。配置普通文件按同次发现读取的字节记录 SHA-256；package.json 的二次线索读取须与清单原摘要一致。不会执行 JS、创建默认配置或自动迁移旧配置。

官方配置文件与类型加载、原生选择优先级依据：https://eslint.org/docs/latest/use/configure/configuration-files 。原生有确定优先级，但尚未冻结项目 CLI 显式参数及版本上下文时，发现层不擅自选择或混合多候选。此层只展示 unknown：动态规则未求值、多候选选择未解析、TS loader 未核验、旧配置需要工具版本上下文、所在 package 根未观察到配置。最后一种不证明外部/显式配置不存在。

## 行为与反例

- 六候选文件内容均为抛异常脚本，detect 仍只读返回六条未知候选、不执行脚本、不创建 codeguard 状态。
- 有 package 但无 flat config，不创建默认规则；旧 eslintConfig 返回独立待核查原因，不将其按现代 flat config 标已配置。
- 根 JS 配置与子包 CTS 配置保持不同目录归属；不继承/混合其规则，要求原生逐文件配置和 loader 核查。
- init 把可见配置字节纳入画像；修改/删除仍使画像过期，工作记录与用户文本保留。既有测试的空配置列表断言更新为精确 node.eslint/unknown，保留“不能声称检查器就绪”的原验收目标。

新增前两项先RED（无node.eslint配置观察），实现后通过。不是原生ESLint兼容、规则执行、依赖闭包或完整项目配置验收；7.3未完成。

最终67项相关回归通过：detect23、init41、discovery端口3。CLI相关Clippy、fmt、OpenSpec strict及插件git diff --check通过。新增行为不改变报告协议版本或质量门禁权威。

## 输入复核及独立源码补充

新增发现端口反例覆盖 package.json 复核时改变/不可读，分别有/无 flat config 四种组合。原实现即使身份失效仍可能 observation_complete=true，先RED后修正：发现不完整、清单阻塞路径和准备原因保留，已有flat config不隐藏错误。新增公开detect反例：独立app.js没有package/config也返回配置准备线索及源码引用，不创建清单。目标及初始化回归66项通过（detect24、init41、端口1）。
