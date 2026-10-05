# JavaScript 隔离原生语法对照与诊断保真

日期：2026-10-05。对应 OpenSpec 12.11、14.17、14.19 的增量切片，父任务仍开放。

## 可观察行为与边界

此前 JavaScript 原生回归直接调用 Node，没有共用取消、截止时间、入口身份和差分协议。本次使用 Rust runtime 的显式本机 Node 24.18.0，将冻结源码通过 stdin 交给 `--check --input-type=module`；清空继承环境，工作目录固定 `/`，不执行源码、不解析导入、不推断项目的 CommonJS/ESM 模式，不替代 ESLint。工具和版本、请求入口、源码与 CodeGuard 程序身份在相应阶段核验。Node 入口大小上限独立为128MiB，其它原生对照仍为64MiB；不是完整运行时依赖闭包批准。

0.4 报告包含 JavaScript，保留32语法库存，明确 `native_adapter_reused=false`：新 Node 语法观察器并未复用产品 ESLint 适配器。只选择 Python 的报告继续使用0.3，其它既有语言继续使用0.1。旧报告字节保持。原始 parser 与组合候选独立统计；JavaScript 没有补充结构规则，组合统计与原始统计相同。固定 `status=incomplete`、`delivery_decision=not_evaluated`、`independent_holdout=false`、`grammar_qualified_count=0`。

退出0必须无输出；退出1只有严格可归属的 SyntaxError 可产生行号诊断，不臆造列号，不保存源码或原生文本到公开结果。异常输出、未知版本、取消、超时、制品/入口变化均保留未知，不借标签补齐结论。受控别名重定向反例先 RED：版本调用后入口改变，旧实现仍调用旧的规范化路径；改为保留原请求入口，第二次调用前拒绝，两个执行标记均不存在。执行中取消反例也先RED：Node检查未使用请求令牌，实际返回completed；接入共享AtomicBool后，在原生检查启动后取消会终止进程组，保持incomplete。该修复仅覆盖新增Node观察器，其它语言的完整取消验收仍按各自任务执行。

## 实际18样本与漏检

| 测量层 | TP | FP | FN | TN | 可比较 | 未知 |
|---|---:|---:|---:|---:|---:|---:|
| 原始 JavaScript grammar | 5 | 0 | 2 | 11 | 18 | 0 |
| 组合候选 | 5 | 0 | 2 | 11 | 18 | 0 |

`javascript-module_return`、`javascript-duplicate_binding` 被 Node module 语法检查拒绝，而固定 grammar 未报告恢复节点，两项 FN 原样保留。它们需要语言上下文规则或原生检查确认，尚未修复。包含 throw/console.log 的合法源码仍通过语法检查，以及导入不存在模块的合法源码仍通过，验证本入口不执行源码、不解析导入。原来的13个窄语法回归也保留原始字节及标签，不删除新发现的反例。此样本集不是独立 holdout，不证明全项目低误报或已获发布资格。

证据：[输入](evidence/javascript-native-grammar-input-2026-10-05.json)、[实际报告](evidence/javascript-native-grammar-differential-2026-10-05.json)。Node 扩展后重新执行 Python 18样本，单独保存[0.3回归报告](evidence/python-native-structure-differential-node-extension-2026-10-05.json)，不覆盖此前已提交的0.3报告；原始6TP/10TN/2FN与组合8TP/10TN保持。

## 制品核验性能

实际 Node 二进制120,965,360字节。未优化的开发态 SHA-256 核验使第一次三测试总耗时404.31秒，其中原生回放耗尽既定300秒预算，报告保留 unknown，测试失败而未产生假通过。仅对开发/test配置的 sha2 包使用 opt-level=3，保持所有重复摘要核验、128MiB制品上限、样本和300秒总预算，三测试降至约40秒并通过。该结果仅是当前本机调试构建的改进，不能作为发布构建或全项目性能验收。Cargo.lock 未变，不安装工具或依赖版本。

## CI 启动失败诊断

上次提交854e084的远端 CI37268268655在既有信号测试中取得 SpawnFailure，而预期 Signaled；底层 OS 错误被丢弃，故根因未知。本次新增缺失工具反例先 RED：私有日志中的数值槽为0，丢失 ENOENT；执行器保留可得的 OS 错误码，CGLOG1 的 kind=7 数值槽记录错误码（0仍表示未知/旧记录），其它终止类型布局与原始输出不变。错误码不伪装成 stderr，不自动重试，不将启动失败变为信号结果。既有信号测试失败信息增加终止原因和私有数值槽，下一次 Linux 执行可提供具体诊断；CI也显式接入JavaScript受控差分与三项观察器单元反例，不安装Node或借用不同版本冒充固定原生对照；本机通过不能证明上次 CI 根因已解决。

## 验证命令

```bash
CODEGUARD_NODE_BIN=/Users/wandl/.nvm/versions/node/v24.18.0/bin/node \
  cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test javascript_native_differential -- --include-ignored --test-threads=1
CODEGUARD_RUFF_SYNTAX_BIN=/opt/anaconda3/bin/ruff \
  cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test grammar_native_differential -- --include-ignored --test-threads=1
cargo test --locked -p codeguard-runtime --test private_log_contract
cargo test --locked -p codeguard-cli --test native_version_observation
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked --workspace --all-targets --features wasm-precheck -- -D warnings
/Library/Frameworks/Python.framework/Versions/3.13/bin/python3 \
  -m unittest discover -s tests -p javascript_native_differential_schema.py -v
```

已执行原生 JavaScript 三项全部通过，40.78秒；包含真实Node18样本，0忽略。原生差分七项全部通过，23.99秒，包含真实Ruff18样本，0忽略。默认全工作区244个目标记录合计1358通过、0失败、123条件忽略；忽略项不计入验收，后续取消修改只涉及WASM配置，另执行WASM目标。私有日志七项通过；原生版本观察七项通过，其中无显式真实Ruff环境的条件测试提前返回，不能计入真实工具验收。JSON Schema目标3+5项通过，另验证新Python0.3报告与历史样本分类一致。完整358例不在本次重跑范围；全部语言资格、独立holdout、项目模式、完整原生工具链和远端CI均按各自实际证据验收，不以这个切片关闭父任务。

最终JavaScript报告SHA-256 `3967e723912a067e8eb6fc18f0653c488670b9d2c637edc73b13f90cd933e785`；Node扩展后Python报告SHA-256 `17c9098e38ed0863fc53cffe8e356f45791e7f96376a2a9713725a81e5ef2587`。

最终WASM CLI单元64通过、0失败、3条件忽略（不计入验收）。默认/WASM全目标严格Clippy、定向rustfmt、分层门禁、OpenSpec strict和diff检查通过。lib.rs只检查自身并保留已有声明/导出排序，避免递归重排既有模块。默认完整回归摘要见[执行记录](evidence/javascript-extension-workspace-default-2026-10-05.json)。受保护Erlang草稿摘要仍为 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`，不提交。Rust库ProcessOutcome新增可选spawn_os_error字段，直接构造此结构的源码调用方需补字段；CGLOG1字节长度、原生输出和其它终止类型不变，启动失败数值槽0兼容未知历史。远端CI须由新提交独立确认。
