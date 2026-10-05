# 原生版本探测后的入口与制品复核

日期：2026-10-05；对应OpenSpec 3.7、5.4、12.11、14.17、14.19的局部验收，父任务保持开放。

## 实际缺陷与修复

Zig、Erlang、Swift此前在版本探测后直接开始源码检查，只在最后核对规范路径的制品字节。版本调用可替换入口或重定向原请求别名，随后仍执行旧路径/新字节，再在批次末撤回结果。新增六语言两种变化共12种反例先RED：Zig/alias的实际源码调用写出执行标记。

三种观察器现在保存请求入口与规范路径的绑定，版本返回后、源码调用前、源码调用后均核对入口解析和有界制品SHA-256；变化返回既有tool_changed/incomplete观察，不运行源码检查、不产生诊断或原生确认。Python、JavaScript、Kotlin既有阶段复核通过同一反例。目标仅是版本结束后的明确阶段边界，不声称核验与spawn之间已经消除全部TOCTOU，亦不证明SDK/JVM/Swift完整工具链闭包。

## 取消反例的稳定性

加入前置复核后，并行单元测试中旧1秒断言记录1.199秒，混入启动、复核和调度开销。新反例使用30秒阻塞原生命令、10秒请求截止时间，要求取消后5秒内结束；它检验的是不能等到命令完成或截止时间，不作为性能资格。临时恢复Zig旧的独立AtomicBool(false)做反事实验证，仍RED，实际等待10.009秒截止；随即恢复共享令牌，六语言两阶段和入口变化反例都GREEN。没有降低产品预算、增加重试或把SpawnFailure当取消成功。

## 本轮验证与保留范围

WASM CLI单元66通过、0失败、3条件忽略，6.03秒；包括六语言两阶段12种取消情形及六语言两种入口变化12种情形。受保护Erlang草稿保持 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`，不提交。临时反事实修改不在最终工作树中。

真实六工具首轮使用此前JavaScript取消输入，共98例，其中Python只有两个基础样本；保留[98例原报告](evidence/native-differential-six-language-entry-binding-base-2026-10-05.json)，不误称覆盖Python18例。随后仅追加已有python_syntax_regression.json的16例，形成[114例输入](evidence/native-differential-six-language-entry-input-2026-10-05.json)和[114例实际报告](evidence/native-differential-six-language-entry-binding-2026-10-05.json)，原98例源码、标签、grammar及比较均保持。输出为0.4、退出3、stderr为空，程序和六个工具身份稳定，库存仍32、资格仍0、holdout=false。

| 语言 | 样本 | TP | FP | FN | TN | 原生未知 | WASM未知 |
|---|---:|---:|---:|---:|---:|---:|---:|
| Zig | 12 | 4 | 0 | 0 | 8 | 0 | 0 |
| Erlang | 38 | 6 | 0 | 10 | 20 | 2 | 0 |
| Swift | 14 | 4 | 0 | 0 | 9 | 0 | 1 |
| Kotlin | 14 | 4 | 0 | 0 | 8 | 0 | 2 |
| Python | 18 | 6 | 0 | 2 | 10 | 0 | 0 |
| JavaScript | 18 | 5 | 0 | 2 | 11 | 0 | 0 |

原始14处漏检与5例未知全部保留。Python独立结构规则仅在组合层补上两个回归FN，不改变原始统计；其它语种原始/组合缺陷保持。所选语料没有FP不表示32语法零误报；VB.NET已知误报不在本轮六工具范围内。协议2项通过，核对新旧输入、报告、分母、标签和原始缺陷，拒绝以资格/交付覆盖这些结果。

```bash
cargo test --locked -p codeguard-cli --features wasm-precheck --lib
cargo build --locked -p codeguard-cli --features wasm-precheck \
  --bin codeguard --example evaluate_native_grammars
target/debug/examples/evaluate_native_grammars \
  "$PWD/target/debug/codeguard" \
  tests/acceptance/evidence/native-differential-six-language-entry-input-2026-10-05.json \
  1200 zig=/opt/homebrew/bin/zig erlang=/opt/homebrew/bin/erl \
  swift=/usr/bin/swiftc kotlin=/opt/homebrew/bin/kotlinc \
  python=/opt/anaconda3/bin/ruff \
  javascript=/Users/wandl/.nvm/versions/node/v24.18.0/bin/node
```

退出3表示开发报告未完成/交付未评估，不能写成产品检查通过。完整358回放、语言独立holdout、语言资格、原生工具闭包、沙箱与发行仍按对应父任务分别验收。

前序27d4956的[CI37273013548](https://github.com/full-stack-plugins/codeguard/actions/runs/37273013548)及69d56b0的[CI37274299258](https://github.com/full-stack-plugins/codeguard/actions/runs/37274299258)均已成功完成，包含MSRV、Linux WASM回放、npm本地包和默认套件。更早信号测试SpawnFailure的OS根因未得到失败时证据，不能声称已确认；本轮阶段复核和新反例仍须新提交独立CI。

最终六个受影响lint/任务关闭服务目标46通过、0失败、4条件忽略；忽略项不算真实工具验收。默认/WASM全目标严格Clippy、定向rustfmt、OpenSpec strict、分层及diff检查通过。未重跑完整默认工作区或358例；不借前序CI完成本轮父任务。114例报告SHA-256 `f5b3f55bdad5255781c0bb65751deaee11a098ea6db88145707886026c78c555`。
