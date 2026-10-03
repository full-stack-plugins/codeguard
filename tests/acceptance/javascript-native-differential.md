# JavaScript / Node 24.18.0 原生语法与内置 WASM 的局部差分验收

日期：2026-10-03。对应既有 OpenSpec 14.4、14.17、14.19 的局部精度证据，三项任务仍未完成。

固定 13 份独立标注的 `.js` 模块源码，用现存 Node 24.18.0 的 `node --check --input-type=module` 通过 stdin 做原生语法对照，并调用公开 `grammar probe javascript` 的隔离 WASM worker 检查同一字节。测试核对原生退出码 0/1、候选实际解析无输入错误、恢复节点分类逐例一致；候选仍须返回 `grammar_qualified=false`、`delivery_decision=not_evaluated` 和退出码 3。原生对照测试只在显式提供 `CODEGUARD_NODE_BIN` 时运行；同一语料的 worker 回归常规 CI 可运行。

本机 Node 为 v24.18.0，程序 SHA-256 `ee6fb0e015284d83a91e8ec5213f43a157f8a392b58555301682892ba928c04a`。显式原生测试命令：

```bash
CODEGUARD_NODE_BIN=<现存 Node 24.18.0 绝对路径> \
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test javascript_native_differential -- --ignored --nocapture
```

本机结果：常规 worker 回归 1/1、显式原生差分 1/1 通过，13 份样例分类一致。8 份合法样例覆盖箭头函数、可选链/空值合并、私有类字段、模块 import、顶层 await、async 和 Unicode 标识符；5 份破损样例覆盖缺少大括号/圆括号、未闭合字符串、缺失初始化表达式及不完整可选链。

Node 的模块解析语法对照不等于 ESLint、类型检查、项目配置或浏览器方言验收。此结果不证明其它 Node/ECMAScript 版本、JSX、TypeScript、CommonJS 特有语义、规模化误报漏报率或公开发行资格；其余 31 种 grammar 不受该局部证据覆盖。
