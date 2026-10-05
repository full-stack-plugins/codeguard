# Ruby 隔离原生开发差分验收

日期：2026-10-05。对应既有 OpenSpec 12.11、14.17、14.19 的实现切片；这些父任务保持未完成。

## 缺口与实现

Ruby 原有直接 `Command` 对照只验证 13 个样本，未接入统一受控差分入口。新增受控集成反例先 RED：`native_grammar_language_unsupported`。Rust 观察器现接受显式固定 Ruby 2.6.10p210，冻结 UTF-8 stdin、有界入口字节摘要和请求别名；版本调用后、源码调用前后复核身份。运行使用清空环境、cwd `/`、共同 deadline/cancellation、64 KiB 输出预算以及固定 `--disable=gems -EUTF-8:UTF-8 -W0 -c -` 参数。

语法成功必须退出 0、stdout 精确为 `Syntax OK\n`、stderr 为空。退出 1 必须有绑定原始 stdin 行号的完整有界语法报告；只保留行号与 `ruby.syntax`，不传播源码或诊断文案。错路径、越界位置、普通警告、混入未知输出、矛盾退出、版本变化、输出/进程故障保持 incomplete/unknown。最多 32 个定位；超出不截成“完整”结果。不从 caret 推断列。

Ruby `-c` 的“只检查而不执行”语义参照 [Ruby 官方选项文档](https://docs.ruby-lang.org/en/master/language/options_md.html)，并以实际固定旧版本独立验证：BEGIN、END、普通 raise、缺失 require 及 shebang `-r` 不执行或加载。该文档本身不证明本机版本行为，实际证据才是验收依据。

## 报告与实际语料

新增 `ruby-native-syntax.schema.json` 与开发差分 0.5 协议，仅选 Ruby 时也保留 32 语言库存与解析/结构分层。旧 0.1—0.4 协议和历史报告字节不修改。Ruby 单独实际执行 18 例，5 TP / 0 FP / 0 FN / 13 TN，原生和 WASM 未知均为 0；见[冻结输入](evidence/ruby-native-grammar-input-2026-10-05.json)与[实际报告](evidence/ruby-native-grammar-differential-2026-10-05.json)。其中 14 例来自既有固定语料，仅新增 4 个隔离/UTF-8 样本，不把回归语料称为 independent holdout。

七语言输入在此前六语言固定输入后仅追加上述 4 例，原有源码/标签/来源保持不变。实际批次结果见[七语言报告](evidence/native-differential-seven-language-ruby-2026-10-05.json)；协议验收核对工具和程序稳定身份、共同分母、旧六语言分类及剩余缺陷，不覆盖任何旧报告。

实际七语言批次为 132 例：34 TP / 0 FP / 14 FN / 79 TN，另 5 unknown；其中 Erlang 10 FN、Python 2 FN、JavaScript 2 FN 原样保留。25 个未选语言仍留在库存，不能据选中七语言的零 FP 声称全体 32 语言零误报。七个工具和程序身份稳定，报告 0.5、退出码 3、stderr 为空。此前六语言 114 例的源码/grammar/原生和 WASM 分类、原始及组合比较均一致。

本地验证：WASM CLI 单元 70 通过/3条件忽略；四个差分/语料目标 25 通过/6条件忽略；显式 Ruby 实际测试 1 通过。单元包含七语言版本/源码两阶段14种取消，以及七语言别名/字节两种变化14种停止执行反例。协议正反例覆盖 0.5、旧 0.4 与历史六语言证据；不把默认 ignored 算作实际工具验收。

## 可复现执行

```bash
CODEGUARD_RUBY_BIN=/usr/bin/ruby cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test ruby_native_differential \
  pinned_ruby_uniform_replay_archives_frozen_native_and_wasm_evidence \
  -- --ignored --exact --nocapture
cargo test --locked -p codeguard-cli --features wasm-precheck --lib
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test ruby_native_differential --test javascript_native_differential \
  --test grammar_native_differential --test grammar_evaluation
python3 tests/ruby_native_differential_schema.py
```

真实 Ruby 测试需要显式已有工具，不能以默认 ignored 计为通过。开发 schema 验证需要已有 jsonschema；产品检查和原生调用全部由 Rust 执行。Linux CI 接入普通 Ruby 受控集成、解析器单元和七语言阶段取消/入口变化反例，不自动安装或借用不同 Ruby 版本。

## 剩余边界

该能力是开发期原生精度对照，尚不是公开 `lint ruby`、Rubocop 项目适配、可信任务关闭或已安装宿主反馈。Ruby 版本范围仅为固定 2.6.10p210；独立 holdout、完整工具链闭包、check-to-spawn TOCTOU、平台矩阵、资源和发行验收仍缺。已验收发行保持 0/32。旧六语言的 Erlang/Python/JavaScript 原始漏检与未知覆盖不得因 Ruby 零误报而隐藏。

规格 strict validate、crate layering、定向 rustfmt 和 git diff 检查通过；开发协议正反例共11项通过。默认与WASM严格Clippy（workspace/all-targets、-D warnings）均通过；未重复执行默认全workspace测试及全32 grammar回放，七语言实际批次与目标回归不代替它们。
