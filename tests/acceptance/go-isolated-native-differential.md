# Go SDK 整文件原生开发差分验收

日期：2026-10-05；对应既有 OpenSpec 12.11、14.17、14.19，父任务保持未完成。

## 整文件与双制品契约

Go 原有直接 `Command` 对照只消费 gofmt 的退出码，未接统一受控入口。新增集成反例先 RED（native_grammar_language_unsupported）。选择 `go=/usr/local/go/bin/go` 明确选择该 SDK，辅助工具只取请求入口同目录 `gofmt`，缺失/不可执行时拒绝，不从 PATH 替代或安装。Go 与 gofmt 的请求绑定、规范入口、字节摘要均冻结；核对 `go version` 和 `go version ABS_GOFMT` 的固定 go1.23.4，版本和源码阶段复核两者，批次末辅助制品变化会撤回此前比较。

[Go 官方文档](https://go.dev/cmd/gofmt/?m=old)明确无文件参数的 stdin 模式接受程序片段。本机固定版本实际验证：`x := 1` 和 `func f() {}` 在该模式退出0，但在 `gofmt -e /dev/stdin` 模式退出2。因此观察器用后者将冻结 UTF-8 stdin 作为完整文件解析。清空环境，显式 GOENV=off、GOTOOLCHAIN=local、GOWORK=off、GOPROXY=off、GOSUMDB=off，cwd `/`，共同 deadline/cancellation；不传 -w/-r，不修改用户源码，不解析导入或执行 init。格式变化不算语法违规，也不能把 formatter 当项目 lint。

成功要求退出0、UTF-8非空格式输出和空stderr；错误要求退出2、空stdout和完整有界 `/dev/stdin:line:column` 报告。只发布 `go.syntax` 与原始字节位置，不传播格式后的源码或诊断文案。未知/混合输出、工具/版本/入口变化、取消、超时或输出预算耗尽保持 incomplete/unknown。两个制品各64MiB、输入1MiB、输出64KiB、最多32个定位；预算不是性能或发行资格。

## EOF 与逻辑位置

[go/token](https://pkg.go.dev/go/token)定义列为一基字节数。Go 的终止换行不增加最终行，EOF 可锚定最后一行换行之后。第一轮真实回放把 missing_brace/missing_init/missing_paren 的这种诊断误判为越界，留下[修复前报告](evidence/go-native-grammar-eof-before-2026-10-05.json)。原生 EOF 单元反例先 RED，修复为保留换行的行字节，只有最后一行允许 EOF 锚点，UTF-8边界继续核对；修复后同一源码的三项从 native unknown 变为 invalid，不改变 grammar 分类或原始标签。

Go `//line` 和 `/*line` 可重映射逻辑位置。当前尚无完整原始位置映射；发现这些原始标记时保守返回 go_syntax_logical_positions_unresolved，不猜坐标。标记检测可能也遇到字符串内的相同字节，这属于明确的未知覆盖限制，不声称精确识别全部指令，不将未知作为源码违规。

## 实际结果与反例

[冻结20例输入](evidence/go-native-grammar-input-2026-10-05.json)与[修复后报告](evidence/go-native-grammar-differential-2026-10-05.json)：5TP / 0FP / 2FN / 12TN，另1原生未知、0WASM未知。两个FN是缺少 package 的语句/函数整文件；Tree-sitter未产生恢复，不能据此批准合法。原始与组合候选均保留FN2。新增样本也验证缺失导入不解析、init不执行及仅格式变化不违规。位置重映射样本保留在分母。

辅助工具批次反例有两条真实启动的受控观察：第二次 gofmt 调用替换自身字节。临时移除末尾 companion 复核会RED，第一条仍保留 true_negative；恢复复核后全部比较和身份撤回为unknown，样本不删除。版本/辅助版本两阶段的别名/字节变化4情形停止源码调用；三个Go阶段的执行中取消，以及八语言两阶段16情形均通过，不等待30秒阻塞进程自行完成。字节完全相同的替换不冒充内容变化测试。

新增0.6开发协议和Go观察协议；0.1—0.5及历史报告不改。八语言扩展仅在此前七语言输入末尾添加6例，原132例源码/标签/来源不变；共同结果见[八语言实际报告](evidence/native-differential-eight-language-go-2026-10-05.json)。全部32语言仍留库存，资格保持0、holdout=false、delivery=not_evaluated。

真实八工具152例最终为39TP / 0FP / 16FN / 91TN与6unknown；其中Go新增2FN与1unknown，其余七语言132例所有源码/grammar/原生/解析及组合分类均与原报告一致，24个未选语言仍在库存。这不是全32语言零误报或完整精度验收。

协议正反例共17项通过，覆盖0.6、旧0.5/0.4、历史六语言与前序版本输入证据。Go SDK摘要失稳的汇总必须使其所有原生及组合比较撤回，保留旧观察而不把它当当前确认。strict OpenSpec、crate layering、定向格式和diff检查通过；没有重复default全workspace或32 grammar全量回放。

## 复现与剩余工作

```bash
CODEGUARD_GO_BIN=/usr/local/go/bin/go cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test go_native_differential \
  pinned_go_uniform_replay_archives_whole_file_and_companion_evidence \
  -- --ignored --exact --nocapture
cargo test --locked -p codeguard-cli --features wasm-precheck --lib \
  --test go_native_differential --test ruby_native_differential \
  --test javascript_native_differential --test grammar_native_differential \
  --test grammar_evaluation
python3 tests/go_native_differential_schema.py
```

本地WASM单元75通过/3条件忽略；五个集成目标28通过/8条件忽略；显式实际Go20例测试1通过。默认ignored不计原生验收。Linux CI接入受控Go输出/双制品/取消/批次反例，不自动安装或借用不同版本SDK。

公开 `lint go` 的 go vet 行为不变。这一观察只用于开发期语法精度对照，不证明类型/依赖/CVE、独立holdout、项目配置闭包、完整工具链/TOCTOU、跨平台资源隔离、任务可信关闭或已安装宿主验收。两个缺package漏检的grammar或结构规则修复、逻辑位置映射及完整发行验收仍开放；这些证据不关闭语言能力父任务。

默认与WASM两配置严格Clippy（workspace/all-targets、-D warnings）通过。前序Ruby提交1e80a15的[完整CI](https://github.com/full-stack-plugins/codeguard/actions/runs/37286928365)最终success（包含Rust1.85 MSRV、32 grammar回放、本地包检查与default workspace）；它不替代本次Go提交的远端验收。
