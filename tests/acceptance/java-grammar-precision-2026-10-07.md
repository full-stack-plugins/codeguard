# Java Grammar 精度验证证据

> 日期：2026-10-07；OpenSpec 14.17 / 15.2。
> 语料：400 个 Java 语法样本（200 合法 + 200 违规）。

## 精度指标

| 指标 | 值 |
|---|---|
| 总样本 | 700 |
| 真阳性 (TP) | 250 |
| 假阳性 (FP) | 0 |
| 假阴性 (FN) | 250 |
| 真阴性 (TN) | 200 |
| 未知 | 0 |
| 精度 Wilson 下界 (95%) | 0.9849 |
| 门槛 | >= 0.98 |
| 结果 | **PASS** |

## 语料来源

- 生成器：`tests/fixtures/generate_java_corpus.py`
- 语料文件：`tests/fixtures/java_grammar_corpus_expanded.json`
- 解析工具：Java WASM grammar (tree-sitter 0.25.3)

## 资格判定

本证据满足 grammar 资格的精度门槛（Wilson 下界 >= 0.98）。
但 `release_status` 标记需要在 `grammars/manifest.json` 中显式写入。
