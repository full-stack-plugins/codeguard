# 原生差分的解析器与结构规则分层测量

日期：2026-10-05；对应 OpenSpec 12.11、14.17、14.19 的局部验收。

原差分入口仅消费 worker 的 ERROR/MISSING 恢复，丢弃已核验的独立结构规则观察。固定 Python grammar 对缺少函数体、错误缩进均返回零恢复；现有检查链的 `codeguard.python.required_suite` 已能提供原生确认指引，但原差分报告无法独立度量这个补充层。本次实际入口回归先 RED：原生两项均确认 invalid，原始 parser 两项 FN，报告缺少结构层；再实现 0.3 协议。

## 协议与失效行为

- 原有 `wasm_classification`、`comparison` 和语言 TP/FP/FN/TN 始终表示原始解析器结果，不使用结构规则抵消 FN。
- `structural_observations` 保留固定规则 ID、版本、配置字节 SHA-256、直接父节点及原始字节坐标。成功无结构观察为 `[]`；worker 失败或程序身份失效为 `null`。最多 128 个观察，父进程继续核验冻结源码与 grammar 身份。
- `combined_candidate_classification` 只在 parser 和结构观察完整时可判定；恢复节点或独立结构观察任一出现，表示候选 invalid，仍要求原生确认。截断时即使保留已见结构记录，两层分类也保持 unknown。
- `combined_candidate_comparison` 与同一原生 oracle 独立比较。语言 `combined_candidate` 的 TP/FP/FN/TN/compared_count/unknown_count 使用相同样本全集；unknown_count 是双方无法共同比较的样本数，不是删除的样本。
- 原生制品或请求入口变化撤回两层比较；程序变化还撤回全部结构观察和组合分类。取消、超时保留样本与未知统计。
- 仅选择 Python 的开发回放升级到 0.3；选择 Python 与其它语言时同一 0.3 报告分别记录各语言，未选择 Python 的既有回放仍使用 0.1。历史 0.1/0.2 schema 和报告字节不覆盖。

缺失结构状态与已知 parser invalid 的组合也保持 unknown；单元反例先发现 Rust 短路运算可跳过缺失状态，显式核对两项完整性后修复。

这个入口不创建任务、不批准白名单，固定 `incomplete`、`not_evaluated`、`independent_holdout=false`、`grammar_qualified_count=0`；组合分类不是已确认违规或完整项目结论。

## 实际 Ruff 18 样本

通过显式 `/opt/anaconda3/bin/ruff` 0.16.8、隔离 stdin 和明确 py312，对同一 18 样本重新执行原生及 worker；不安装工具或运行用户源码。原生全部可判定、没有标签分歧，工具和程序身份稳定。

| 测量层 | TP | FP | FN | TN | 可比较 | 未知 |
|---|---:|---:|---:|---:|---:|---:|
| 原始 grammar | 6 | 0 | 2 | 10 | 18 | 0 |
| 解析器 + 独立结构规则候选 | 8 | 0 | 0 | 10 | 18 | 0 |

`python-empty_body` / `python-bad_indent` 两项保留原始 FN，独立结构观察均为 `codeguard.python.required_suite`，组合比较分别为 TP。这证明当前回归样本的补充效果，不证明独立 holdout、其它版本/方言、全语言精度或 grammar 已修复。

新证据：[Python 分层实际报告](evidence/python-native-structure-differential-2026-10-05.json)，SHA-256 `b3fc6c23ac30ccb0729e64d8a612e7ca12e0268a84d7cb17a48ffd11fb7ac08a`。旧当前清单报告 SHA-256 保持 `dae43030040a933719fc7b6435abca007fae834b20b824687d97c5ec73bc9f91`；新旧样本 ID、源码/grammar 身份、标签、来源和原始比较逐项一致。

## 验证命令

```bash
CODEGUARD_RUFF_SYNTAX_BIN=/opt/anaconda3/bin/ruff \
  cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test grammar_native_differential -- --include-ignored --test-threads=1
cargo test --locked -p codeguard-cli --features wasm-precheck --lib \
  grammar_native_differential::tests
/Library/Frameworks/Python.framework/Versions/3.13/bin/python3 \
  -m unittest discover -s tests -p native_grammar_differential_schema.py -v
```

最终原生差分目标 7 项通过、0 失败、0 忽略，209.72 秒；WASM CLI 单元目标 61 项通过、0 失败、3 项条件忽略，3.89 秒。3 项忽略不计入验收；本次实际 Ruff 对照在显式原生差分目标中单独执行。协议目标 5 项通过，核对实际两层统计和历史报告，拒绝候选升级权威、结构记录删除/规则伪造、失效身份沿用、未知 parser 变为已判定、错误比较或把结构规则伪装 ERROR。独立 holdout、全部语言原生对照、宿主实际反馈、完整性能与发行验收仍未完成，父任务不勾选。

默认与 WASM 两种配置的全目标严格 Clippy（`-D warnings`）、定向 rustfmt、crate 分层检查、OpenSpec strict 和 `git diff --check` 均通过。本轮未重新执行完整默认工作区测试或全 358 样本回放，不借用其历史结果证明本次完整验收。受保护 Erlang 草稿 SHA-256 保持 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`，不纳入本次提交。
