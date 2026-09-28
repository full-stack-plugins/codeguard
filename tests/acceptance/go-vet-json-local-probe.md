# Go vet JSON 局部原生探针（OpenSpec 8.2 进行中）

本机 `/usr/local/go/bin/go` 为 `go1.23.4 darwin/arm64`。在仅依赖标准库、`GOPROXY=off`、`GOSUMDB=off`、`GOTOOLCHAIN=local`、`GOWORK=off` 的独立临时 Go Module 中执行 `go vet -json ./...`：

- `fmt.Printf("%d", "bad")`：进程退出 0，stdout 为空，stderr 为 `# package` 头与 `printf` 原生 JSON 诊断。
- 修正为整数参数：进程退出 0，stderr 为相同包头与 `{}`。这只是本次局部零诊断，不证明完整项目覆盖。
- 改为未定义函数：原生执行/报告失败，解析结果 `Incomplete`，不保留部分 finding。

[Go 命令文档](https://pkg.go.dev/cmd/go)公开 `go vet -json` 入口；退出码与 stderr 组合以上述实机样本为准，不能由一般 `go vet` 的非 JSON 退出语义推断。Rust `parse_go_vet_json` 只接受明确匹配的 Go 1.23.4 版本、退出 0、空 stdout、单个或多个完整包块及规范 JSON；对包归属、分析器、位置和根内 `.go` 路径做保守核对。macOS `/var` 与 `/private/var` 别名在真实测试中先造成范围误判，解析器现对受控根的原路径及规范路径作等价处理，同时拒绝 `..` 与根外诊断。任一块错误时整轮 `Incomplete`、不输出部分 finding。

回归：`cargo test -p codeguard-adapters --test go_vet_contract --offline` 验证退出 0 仍有 finding、空对象不升级完整覆盖、坏 JSON/版本/包/位置/编译错误反例、多包报告与坏块原子失败。真实原生测试显式运行：`CODEGUARD_GO_TOOL=/usr/local/go/bin/go cargo test -p codeguard-adapters --test go_vet_contract --offline -- --ignored`。解析器本身不证明规则包/工具锁、模块/build tags/平台完整范围或可信项目门禁；Staticcheck 注释检测器本机未就绪，8.2 保持未完成。

## CLI 局部接线

`codeguard lint go [path] --go-tool ABS_PATH --format json` 现使用 Rust `ProcessSpec` 字面调用 `go version` 与 `go vet -json ./...`。运行环境清空继承项，强制 `GOPROXY=off`、`GOSUMDB=off`、`GOTOOLCHAIN=local`、`GOWORK=off`、`GOENV=off`、`CGO_ENABLED=0`，并给 Go 私有缓存、GOPATH、HOME 和 TMPDIR；无显式工具、版本不符或输出异常时保持未完成。执行前后核对工具、`go.mod`、可选 `go.sum` 以及只读发现的 Go 源文件字节摘要。公开反馈仅显示原生规则、相对路径、行列和原文摘要，不把原生诊断文本作为智能体指令。反馈协议见 `schemas/go-lint-local-feedback.schema.json`，所有路径仍固定退出 3、`delivery_decision=not_evaluated`、`coverage_proven=false`。

本机 Go 1.23.4 的显式 CLI 测试依次得到违规、局部干净和编译故障反馈；不选工具、无关 `/bin/echo` 及无效超时均没有生成干净扫描。命令尚未接入 `check all`、任务同步/复检、可信工具锁及策略，也没有完整 Go Modules、build tags 或跨平台覆盖；Staticcheck 注释检测继续缺真实适配与测试，8.2 不勾选。

后续范围反例：若原生报告指向根内却未进入本轮只读发现的 `.go` 文件，返回 `go_vet_diagnostic_outside_discovered_sources`、不导出该诊断为 finding；报告或源码身份必须先查清。

局部 CLI 现在识别最多 64 个 Go Module，按相对根排序并在各自目录执行 `go vet -json ./...`。本机真实双模块样本中，嵌套模块的 `printf` 违规以 `nested/main.go` 与模块根 `nested` 展示；根模块有违规、嵌套模块编译失败时，结果保持 incomplete，同时保留已完成根模块的原生发现。执行后重算全部 manifest、各模块可选 `go.sum`、源码和工具摘要；变化则丢弃本轮 finding。反馈协议升至 0.2，逐模块列出完成状态。仍未验证 build tags、目标平台/CGO、工具锁与完整规则策略，不能签发门禁。

`check all [path] --go-tool ABS_PATH` 现复用同一 Go 观察服务，在共享任务调度、deadline/jobs 和 JSON/human/SARIF 汇总中显示 Go 结果。缺工具保持 `native_incomplete`，本机真实 `printf` finding 在 JSON 与 SARIF 可见，但 SARIF `executionSuccessful=false`、交付始终 incomplete。2 秒共同预算下的慢版本探测被终止并标记 `deadline_exceeded`；Java 选择与相对工具路径均拒绝。完整检查反馈协议为 0.19，内部故障协议为 0.4。Go 持久任务、完整规则/平台覆盖与可信门禁仍缺。

## 稳定发现身份（反馈 0.3）

Go 发现新增 `finding_id`、`finding_fingerprint`、`source_sha256`。身份按模块、package、路径、原生规则、去首尾空白的源码行、原生诊断和同锚点序号计算，不绑定绝对行号；本轮文件字节摘要独立记录。原生结果先排序，重复位置/规则/消息去重。诊断位置不存在、超出源码行或落到空白行时，该模块保持 incomplete，不发布部分有效发现；重新读取源码不匹配扫描摘要时丢弃本轮发现。

三项纯身份测试及新增模块位置越界反例通过。显式本地 Go 1.23.4 测试验证重复扫描和前置空行的稳定 ID、源码摘要变化以及修复后的零诊断。统一 `check all` 反馈升至 0.20；两种真实输出均通过 Draft 2020-12 schema 校验。稳定 ID 只是持久任务前置能力，尚未实现 Go work sync/next/task verify，也不赋予白名单或门禁批准。

## 持久任务同步（Go 0.4 / check 0.21）

初始化工作区中的 `lint go` 和 `check all` 现绑定 workspace/run，保存报告并消费本地队列；Go 单独扫描与统一扫描共用同一同步函数。反馈带模块 manifest/go.sum 摘要、同步状态和新增计数。尚未初始化或工作区损坏时不写本地队列。存储失败保留原生结果，反馈 backlog_update_failed，不签发交付通过。

同步器核对运行文件名、工作区、工具/原生状态形状、完成模块计数、诊断计数、模块归属、位置和源码/清单内容。输入已变化的发现只计历史，不新建源码任务。原生失败生成环境任务；成功的局部观察仍有规则/平台覆盖待确认任务，不能把局部干净扫描当作完整交付。next 和 Markdown 任务均给出 Go 修复范围与原工具复扫参数，不回退 Ruff 指引。

`go_work_sync` 普通 3 项覆盖缺工具幂等任务、统一检查复用、错工作区/计数矛盾/过大计数/越界路径/模块错属/位置无效，以及源码或 manifest 变化；显式 Go 1.23.4 原生 1 项验证发现自动保存、重复扫描任务唯一、修复后任务保持 open。两种已初始化真实含违规输出通过 Draft 2020-12 schema 验证。合成报告只验证本地一致性，不能证明可信原生来源、批准策略或白名单生效。Go task verify 及正式关闭/重开仍未实现。

## 原工具任务复检

`task verify ID --go-tool ABS_PATH` 已接入 Go 的共享原生观察、任务租约、尝试绑定、保存/同步及追加事件。源码子报告 0.5 绑定 task/path/本轮源码字节摘要；反馈 0.4、事件 0.3。仍存在为 still_present；同文件同规则新指纹须审查身份，不能凭 ID 变化说已修复；原问题未检出仅为待覆盖/策略核验候选；编译故障为 incomplete，缺工具为 still_blocked。稳定输入的失败也记录；输入变化或租约/存储失败拒绝采纳该轮事件。

Go 普通环境复检与 next 事件读取通过；显式 Go 原生复检验证原问题、身份变化、修复、编译错误，均保持 open。next 会消费本轮报告中的源码摘要，并在后续模块清单变化时要求复扫。四类实际复检反馈及 Go 子报告通过 JSON Schema 验证。目标文件的 build tags、平台、规则全覆盖和可信策略尚未核验；候选不是正式修复关闭，Go 正式关闭/复发重开仍缺。

## 原生文件选择复核（Go 普通 0.6 / 源码复检 0.7 / check 0.22）

先复现 RED：将有 printf finding 的 main.go 加上未启用的 build tag，并保留另一个可构建入口；旧复检因 vet 零诊断返回 candidate_absent_unverified_policy。现在源码任务复检在同一 deadline 内以相同受控环境执行 `go list -json ./...`，区分原生活动文件与 IgnoredGoFiles；包括包内测试及外部测试源文件，不把排除文件当作已扫描。解析拒绝坏 JSON、重复关键字段、错误包、根外/越界路径、活动与忽略冲突及空活动范围。

Go 局部报告记录完整已发现源码的有序摘要映射之 SHA-256；vet 与 list 前后均核对源码、模块和工具。next 在任一 Go 源文件变化后要求复扫；旧报告同步只保留历史。源码目标 selected 才能形成待策略核验的未检出候选；excluded/not_selected 为 rule_coverage_requires_review，清单不可用为 incomplete，任务始终 open。

显式原生 go_work_sync 三项已通过，其中排除测试分别验证自定义标签、CGO_ENABLED=0、非宿主平台标签及随后源码变化。解析器两项、普通同步四项、next 六项、task verify 十项通过。默认忽略的原生测试须用 CODEGUARD_GO_TOOL 显式运行；它们没有证明启用 CGO、跨平台构建、Staticcheck 注释、全部规则或受批准策略。此增量没有白名单批准或任务关闭效力，8.2/9.13 保持未完成。
