# CodeGuard — 原生发现到修复任务

当前源码的 Erlang 原生检查不再依赖先有 WASM 错误。已初始化工作区中，`check all` 和 `lint erlang FILE` 会把当前诊断或环境/预处理阻塞直接保存到同一稳定任务；未初始化目录不自动创建工作台。单文件入口选择最近已有 `.codeguard/`，损坏工作台不越过或初始化。

## 执行路径

```mermaid
flowchart TD
    A[check all / lint erlang] --> B[OTP 28 native scanner/parser]
    B --> C{Current source and tool identity?}
    C -->|Changed| D[Withdraw positions; request fresh check]
    C -->|Current| E{Diagnostics or blocker?}
    E -->|Clean; no existing task| F[No new repair task; project checks remain]
    E -->|Diagnostics / environment / preprocessing| G[Persist native observation; no invented WASM]
    G --> H[Reuse workspace + file + language task]
    H --> I[next: repair source or restore environment]
    I --> J[task verify / repair_ready: original native tool]
    J -->|Diagnostics| I
    J -->|Zero diagnostics| K[Preserve evidence; await policy and closure authority]
    J -->|No progress| L[Concrete diagnosis or decision; retain task]
```

```bash
codeguard init . --apply
codeguard check all . --erl-tool /absolute/path/to/erl --format=json
# Or check a single file; it binds to the nearest existing workspace.
codeguard lint erlang src/app.erl --erl-tool /absolute/path/to/erl --format=json
codeguard next . --format=json
codeguard task verify TASK_ID . --erl-tool /absolute/path/to/erl --format=json
```

## 证据、任务和性能边界

- 首次原生证据使用 `syntax_confirmation_observation` 0.2，`observations: []` 明确没有 WASM 观察；`native_evidence` 绑定源码、所选工具和原生位置。
- 文件/语言身份与旧 WASM 确认任务相同。重复扫描只更新证据；每轮有界文件批次只执行一次工作台同步，不为每个文件重扫历史。
- `next` 比较扫描与复检的实际报告时间，返回最新已消费报告引用。初始扫描不伪造 `task_verify` 操作或验证事件。
- 缺工具、版本不匹配和预处理未展开进入环境指引，不能形成源码违规。诊断列为 `unicode_scalar`。
- 报告保存或同步失败保留原生诊断，任务 ID 为 null，并返回 `task_sync_reason`。源码或工具变化撤回当前修复位置。
- 首次零诊断不创建修复任务；已有任务的零诊断保留观察、仍为 open。默认宿主的可信关闭及完整项目 lint/预处理/注释仍需完成；源码 SDK 已接入限定 Erlang 关闭与复发；本切片不提升 grammar 资格。

## 版本化协议

| Output | Version | Schema |
|---|---|---|
| Native-first observation | 0.2 | [observation](../schemas/syntax-confirmation-observation-v0.2.schema.json) |
| Bound forms scan | 0.2 | [scan](../schemas/erlang-forms-scan-v0.2.schema.json) |
| Bound check all | 0.37 | [check](../schemas/check-feedback-v0.37.schema.json) |
| Bound single-file lint | 0.3 | [lint](../schemas/erlang-lint-feedback-v0.3.schema.json) |
| First-scan next guidance | 0.5 | [next](../schemas/repair-brief-preview-v0.5.schema.json) |
| Recheck of native-first task | 0.3 / outer 0.14 | [recheck](../schemas/syntax-task-recheck-v0.3.schema.json), [feedback](../schemas/task-verification-preview-v0.14.schema.json) |

未绑定工作区的检查仍使用原 0.36/scan 0.1、单文件 0.2；历史 WASM 来源任务仍使用 recheck 0.2/外层 0.13。复检后的 next 仍为 0.4，repair_ready 仍为 Hook 0.8；旧 schema 字节未改。

## 实际报告示例

以下是本机 OTP 28 执行的完整原生首次报告和 `next` 输出。路径、工作区身份和运行编号仅属于该次临时验收，不是可复用配置；不是完整交付通过。

```json
{
  "affected_paths": [
    "app.erl"
  ],
  "authority": "local_unverified",
  "blocker_id": "CG-B-8a1ca1e0c61f9964d131d876588ba6d9",
  "build_root": ".",
  "checker_id": "syntax.native_confirmation",
  "coverage_proven": false,
  "delivery_decision": "not_evaluated",
  "execution": "incomplete",
  "fingerprint": "8a1ca1e0c61f9964d131d876588ba6d981175c0b5020a04500dc490ac01c4c93",
  "language": "erlang",
  "native_evidence": {
    "native": {
      "diagnostics": [
        {
          "column": 4,
          "line": 2,
          "rule_id": "erlang.syntax.error"
        }
      ],
      "diagnostics_truncated": false,
      "preprocessing_unresolved": false,
      "reason": "erlang_native_syntax_diagnostics",
      "status": "diagnostics_observed",
      "tool_sha256": "cd03d938d7547ef608076a58a49f5284931b43f39090087baf35efc1665dd5d6",
      "version": "OTP 28"
    },
    "target": {
      "language": "erlang",
      "path": "app.erl",
      "source_sha256": "22b202c1303137676dc8dbf46be3c26771f4faa453a223f1b607aecd2087c9e0"
    },
    "tool_path": "/opt/homebrew/Cellar/erlang/28.5/lib/erlang/bin/erl"
  },
  "observations": [],
  "reason_code": "native_syntax_confirmation_needed",
  "report_type": "syntax_confirmation_observation",
  "run_id": "syntax-confirm-4540-1791079691978950000",
  "schema_version": "0.2.0",
  "scope": "app.erl",
  "workspace_binding": "bound",
  "workspace_id": "ws-47d23398aadfa34bfe6493a1104f6880"
}
```

```json
{
  "authority": "local_unverified",
  "command_status": "complete",
  "delivery_decision": "not_evaluated",
  "disposition": "actionable",
  "exit_code": 0,
  "next_actions": [],
  "operation": "next",
  "reason": "local_task_selected",
  "repair_brief": {
    "action_id": "repair-source",
    "affected_paths": [
      "app.erl"
    ],
    "authority": "local_unverified",
    "build_root": ".",
    "checker_id": "syntax.native_confirmation",
    "closure_condition": "原生工具按相同受控策略完整复检并确认问题已解决；局部查询不能关闭任务",
    "constraints": [
      "先恢复检查完整性",
      "不得关闭检查器或修改无关源码"
    ],
    "disposition": "actionable",
    "evidence_ref": {
      "first_report_sha256": "d395171fb126df699a1593fe712e3405b99f645b9fedca9f9ea97ae97393a4b1",
      "first_run_id": "syntax-confirm-4525-1791079691732760000"
    },
    "history": {
      "attempt_count": 0,
      "awaiting_verification": false,
      "budget": 2,
      "no_progress_count": 0,
      "open_attempt_id": null,
      "recent": []
    },
    "kind": "blocker",
    "native_column_unit": "unicode_scalar",
    "native_confirmation_reason": "erlang_native_syntax_diagnostics",
    "native_confirmation_ref": {
      "report_ref": ".codeguard/reports/syntax-confirm-4540-1791079691978950000.json",
      "report_sha256": "a5aa5e7a3d8b2e3e2d002fa8a2c588ff7e5c13228a2f883e3b62da3b1a0cda6b",
      "run_id": "syntax-confirm-4540-1791079691978950000"
    },
    "native_confirmation_status": "diagnostics_observed",
    "native_diagnostic_positions": [
      {
        "column": 4,
        "line": 2,
        "rule_id": "erlang.syntax.error"
      }
    ],
    "reason_code": "native_syntax_confirmation_needed",
    "recheck_argv": [
      "codeguard",
      "task",
      "verify",
      "CG-B-8a1ca1e0c61f9964d131d876588ba6d9",
      ".",
      "--format",
      "json",
      "--erl-tool",
      "/opt/homebrew/Cellar/erlang/28.5/lib/erlang/bin/erl"
    ],
    "schema_version": "0.5.0",
    "scope": "app.erl",
    "step": "当前源码已有原生 Erlang 语法诊断；核对报告中有界原生位置并修复，然后使用同一工具复检；不要反复安装工具或关闭检查",
    "task_id": "CG-B-8a1ca1e0c61f9964d131d876588ba6d9"
  },
  "report_type": "repair_brief_preview",
  "schema_version": "0.5.0"
}
```

公开 npm 0.1.4 不含本批实现。当前源码的私有包已通过隔离缓存离线安装后的原生首次 lint/check/next/task verify/repair_ready：受控协议与真实 OTP 28 两组分开运行，26 份实际报告经 schema 校验。修复后没有可信策略仍保留 open，持久化失败保留位置且不伪造任务。该证据不代表公开版本已升级；实际宿主自动显示与插件发布仍需独立验收。见[原生链路验收](../tests/acceptance/erlang-native-first-workbench.md)和[安装后验收](../tests/acceptance/npm-erlang-native-repair.md)。

## P3C 单文件项目绑定

原生 `lint java FILE --checker p3c` 选择最近已有工作台，或显式 `--workspace ROOT`。仅检查所选文件及最近 POM 可静态确认的规则子集。子模块缺失/未知配置会遮蔽父 POM，不以默认十个规则集替代。显式 P3C 保留原生准备阻塞；默认无上下文 WASM 是独立的候选路径。

```mermaid
flowchart LR
    A[P3C 原生单文件请求] --> B[已有工作台与最近 POM]
    B --> C[所选规则的一次原生探针]
    C --> V[核对新鲜报告、规则和当前输入]
    V --> D[保留有效诊断]
    V --> X[保留执行失败阻塞]
    D --> E[共用同步与稳定任务]
    X --> E
    E --> F[next / task verify]
    F --> G[原 P3C 工具复检]
    G --> H[记录结果并保留覆盖缺口]
```

```bash
codeguard lint java src/main/java/Example.java --checker p3c --workspace . \
  --maven-tool /absolute/path/to/mvn --java-home /absolute/path/to/jdk \
  --maven-repo /absolute/path/to/offline-repository --repo-sha256 REPOSITORY_TREE_SHA256 \
  --format json
codeguard next . --format json
codeguard task verify TASK_ID . --maven-tool /absolute/path/to/mvn \
  --java-home /absolute/path/to/jdk --maven-repo /absolute/path/to/offline-repository \
  --repo-sha256 REPOSITORY_TREE_SHA256 --format json
```

将示例路径、仓库身份和任务 ID 替换为实际选择的上下文。已绑定反馈为 [0.1](../schemas/java-p3c-file-feedback-0.1.schema.json)，内部项目观察仍为 0.2，未绑定反馈仍为 0.2。单文件路径只读取祖先 POM，不发现兄弟源码；复用聚合检查的保存/同步服务和既有原工具任务复检。工作台损坏、源码越界、源码或父目录链接均在 Maven 启动前阻止该路径。保存失败保留原生诊断与具体原因，不伪造下一步。零诊断在规则覆盖未证明时保留任务 open。公开 npm 0.1.4 不含此变更。[受控协议验收](../tests/acceptance/java-p3c-file-workbench.md) 不证明完整 P3C、生效模型覆盖或真实宿主安装。

项目观察 0.2 在局部状态 `incomplete`、原因 `native_execution_failed` 时，保留非空且已核对的原生诊断。源码/规则/位置投影沿用成功诊断的当前字节和配置检查；同步同时导入源码 finding 与执行阻塞，`observed_file_count` 不增加。原工具复检再次匹配问题为 `still_present`；部分零诊断为 `incomplete`，阻塞复检仍为 `still_blocked`。没有平铺投影的历史 0.2 报告保持只导入阻塞的解释；不增加 schema 字段或批准语义。见[验收](../tests/acceptance/java-p3c-partial-execution.md)。


### Erlang 限定任务关闭与复发（当前源码 SDK）

`verify_erlang_task_resolution` 复用 Zig 的签名核验、共享截止时间、租约、尝试交接和追加父链服务。受保护宿主独立固定信任根、工作区、策略修订、基线和可信时钟；项目文件不提供批准权威。OTP 28 对首次反例和当前字节分别执行 scanner/parser：首次有原生诊断、当前字节改变且完整无诊断，才能记录 `code_fixed`。宏/include、空 forms、截断或执行失败保留待核验；原样本合法进入误报调查。

支持 WASM 首次和原生首次两种任务来源。Erlang 策略 1.1.0、证据 0.2.0 与旧 Zig 1.0.0/0.1.0 独立；原生首次的 `grammar_sha256` 必须为 null，首次工具身份也须一致。内部规则身份使用实际批准策略字节摘要，不伪造 grammar 摘要。历史读取核对首次报告语言、源码和 grammar，重算本地摘要不能跨语言套用。普通 `task verify --erl-tool` 可在同一工具下追加复发重开；没有可信策略仍不能关闭。

这仍是源码 SDK，尚未接入默认插件的可信策略提供者，公开 npm 0.1.4 不含本批扩展；限定语法任务收据不是完整 lint、安全或项目门禁许可。原生工具摘要绑定 launcher，宿主仍须独立保护 OTP 运行环境。执行路径与实际测试见 [Erlang 生命周期验收](../tests/acceptance/erlang-task-resolution-lifecycle.md)。


#### 实际收据示例

以下来自已安装 OTP 28 的原生首次任务测试；信任根和时钟是签名夹具，`host_context_verified` 仅对该测试上下文成立，不代表默认插件已取得批准。`resolved` 只指这一项语法任务，交付仍未评估。

```json
{
  "authority": "host_context_verified",
  "delivery_decision": "not_evaluated",
  "event_ref": ".codeguard/findings/CG-B-adbe0d18568c875773065066871ea3ae/events/lifecycle-event-fb3b032308cacd1f0ac441e5b3cf0634b54d4ec2719dfed22cdee6dbca9e8c96.json",
  "evidence_ref": ".codeguard/state/resolution_evidence/be49a29789222b17f49c539f9e0527faad9f9d0c9726ae874e3506ae8d050ecb.json",
  "evidence_sha256": "be49a29789222b17f49c539f9e0527faad9f9d0c9726ae874e3506ae8d050ecb",
  "identity": {
    "checker_id": "syntax.native_confirmation",
    "scope": "app.erl",
    "task_id": "CG-B-adbe0d18568c875773065066871ea3ae",
    "workspace_id": "ws-61c7cac666cd3addf981def0b69429cb"
  },
  "outcome": "code_fixed",
  "policy_revision": "p1",
  "policy_sha256": "78518f40ca4579bb414a8cac0ac216bebb0a2df14227142402b6aa4ef43e4d0f",
  "report_type": "task_resolution_receipt",
  "schema_version": "0.1.0",
  "state": "resolved"
}
```

完整 [收据](../tests/acceptance/evidence/otp28-native-first-resolved-2026-10-04.json)、[证据](../tests/acceptance/evidence/otp28-native-first-resolved-2026-10-04-evidence.json) 和 [复发收据](../tests/acceptance/evidence/otp28-native-first-reopened-2026-10-04.json) 保留原始字节。


### Erlang 任务复检自动发现原生工具（当前源码）

`codeguard task verify "$TASK_ID" . --format=json`（`TASK_ID` 使用 `next` 返回的真实 `task_id`） 对已有 Erlang 语法任务复用 lint/check 的工具选择：显式 `--erl-tool` 优先，否则从调用方 PATH 的绝对目录固定首个普通可执行 erl。复检核对 OTP 28、当前源码与工具字节，并沿用任务租约、预算、事件和原有报告版本。候选来源及原生首次发现来源均可复检；repair-ready Hook 复用同一入口。

未找到工具才生成 `erlang_tool_not_found_on_path` 的环境观察；相对/空 PATH 与不可执行文件不参与选择。显式坏工具、所选版本或执行失败不改用后续工具，也不自动安装。成功观察后的 `next` 带实际工具的显式复检 argv，后续 PATH 变化不能替换这个入口。局部零诊断依然不自动关闭任务，宏/预处理、可信策略和完整项目能力仍须核验。见[复检自动发现验收](../tests/acceptance/erlang-recheck-discovery.md)。公开 npm 0.1.4 尚未包含本批改动。

### Swift 无定位观察的原生复检（当前源码）

已有 Swift 确认任务现支持 `codeguard task verify "$TASK_ID" . --swift-tool /absolute/path/to/swiftc --format=json`；`TASK_ID` 来自实际 `next`。同参数可传给 `hook execute` 的 `repair_ready`。调用方必须指定已有 Apple Swift 6.4 工具；当前不自动安装或从历史报告启动可编辑路径。缺工具给出定位既有编译器的具体动作，版本不匹配或执行失败保留原任务。

Rust 读取并复核有界源码字节，通过冻结 stdin、固定 `/` cwd、清空环境和共同截止时间执行 `swiftc -frontend -parse -diagnostic-style llvm -no-color-diagnostics -`。只投影原生错误的规则和位置，不把原始诊断文案交给智能体作为指令。Swift 列坐标是 UTF-8 字节，核对字符边界；未知输出、退出码矛盾、超时、位置越界或工具变化均保持未完成。工具摘要绑定启动入口，不独立认证整套 Swift 安装环境。

原生错误使同一任务的 `next` 进入源码修复；源码或工具变化撤回旧位置。修复后零诊断记录 `candidate_absent_unverified_policy`，不自动关闭，也不替代项目 lint、类型检查、宏/条件编译上下文、构建、安全或交付义务。重复无进展仍使用既有尝试预算。本批不提升 Swift grammar 资格或 32 语言精度结论，公开 npm 0.1.4 尚不含此扩展。

新增协议分别为 `syntax_task_recheck` 0.4.0、`task_verification_preview` 0.15.0、`repair_brief_preview` 0.6.0、Hook 反馈 0.9.0（任务摘要 0.3.0）。聚合 `check` 的 `next` 含 Swift 原生简报时用 0.39.0，其他路径保留 0.38.0；旧 schema 原件不改。具体实测和完整报告见 [Swift 原生确认验收](../tests/acceptance/swift-native-task-confirmation.md)。


### Ruby 原工具关闭与复发（当前源码 SDK）

`verify_ruby_task_resolution(&RubyTaskResolutionRequest)` 将固定 Ruby 2.6.10p210 接入现有宿主 SDK。签名策略 1.9.0 与证据 0.10.0 同时支持原生首次及 WASM 首次任务：前者 grammar 为 null 并绑定首次工具，后者保留首次 grammar。原始样例与当前源码共用预算；版本声明及其缺项在请求前后复核。只有原工具确认原问题、当前已修改源码完整无诊断且输入稳定，才关闭限定任务。重复复检幂等；同工具的普通 `task verify --ruby-tool` 可沿父链记录复发。

原始样例原生无诊断时提出误报复核；未知版本、坏输出或声明变化不能关闭。Ruby 诊断只使用原生行号，不制造列号或冒充 RuboCop。签名/信任根仍由独立宿主提供；本批源码能力尚未发布到 npm，也未证明默认插件已能可信关闭。详见 [Ruby 原工具验收](../tests/acceptance/ruby-task-resolution.md)。


### ShellCheck 原规则关闭与复发（当前源码 SDK）

`verify_shell_task_resolution(&ShellTaskResolutionRequest)` 支持固定 ShellCheck 0.11.0 的原生普通问题任务。策略 1.10.0 绑定原任务、规则、原报告、源码、工具、适配器、方言及项目配置；证据 0.11.0 和收据 0.2.0 使用普通 `CG-…` 身份，旧 0.1.0 收据保持不变。原始源码及当前源码用同一工具和冻结配置复检，原规则在当前源码中消失即可关闭该任务；其它规则的发现和任务仍保留。普通复检在同工具同配置下发现原规则复发，会沿同一任务父链重开。

配置变化不能冒充源码修复；禁用注释变化或既有原规则禁用指令没有作用域证明时要求复核，未变化且只涉及其它规则的指令不会自动阻止关闭。坏报告、未完成执行或原样本不能确认原规则时保持开放或待复核。SDK 支持借用租约与消耗已完成尝试，独立宿主签名要求保持不变；这不是普通 CLI 自动关闭、项目全部通过或公开 npm 已包含的证明。详见 [ShellCheck 原规则验收](../tests/acceptance/shell-task-resolution.md)。


### 隐藏 grammar 错误的确认指引（当前源码）

显式 `grammar probe` 遇到语法树 has_error、公开恢复节点不可定位时，使用0.5报告 `parser_error_location_unavailable=true`，要求用原字节做原生确认，再判断源码修复还是grammar调查；零位置不再只给常规lint建议。未确认前保留未完成，不生成虚构定位，不称初检通过。预算耗尽保持已有聚合但不冒充隐藏错误；普通和结构路径保留历史协议。

同一Kotlin/Swift资产在已安装web-tree-sitter0.25.10也出现相同可见性缺口；切换加载器本身不能解决。合法Kotlin对象声明也可能触发隐藏分号，必须避免按has_error直接改源码。实际输出、兼容性及验收边界见[隐藏错误指引](../tests/acceptance/hidden-parser-error-guidance.md)。


独立JDK21路径现按原生消息识别空注释、缺用途及裸参数/返回/异常描述，并保留五种原生规则到稳定修复任务。`lint java FILE --checker javadoc`、`comments java FILE --workspace .`、已识别配置的项目comments及原任务task verify共用源字节绑定解析器；旧解析器和Maven协议不扩大。新增JDK原生0.2、项目0.4、工作台/复检0.3、文件反馈0.7/工作台反馈0.8、修复指引0.4、任务预览0.31、聚合0.68和异常0.19；缺配置/工具/未知格式仍未完成。真实JDK21两种模式各运行4/3/1/0诊断样例，16张原任务逐项确认仍存在及修复后未受信消失，事实仍open；详细中文与合法继承说明不产生诊断。这不是全部Java详细行为契约或生产资格，Maven真实描述验收、Checkstyle完整描述验收、所有语言四核心和可信关闭仍待完成。见[独立JDK详细描述验收](../tests/acceptance/jdk-javadoc-detailed-descriptions.md)。

## Maven详细Javadoc描述：实现与验收分开

Maven原POM多文件路径现接入五类原生描述规则：空注释、缺主用途及空参数/返回/异常描述。新的详细解析入口保留源码行/caret、消息、位置和汇总核验；历史解析入口及schema不扩大。BUILD SUCCESS中的warning也保留为问题；未知输出、工具/配置故障与实际离线插件缺失保持检查不完整，生成准备任务。绝不回退单文件检查绕过Maven失败。

```mermaid
flowchart TD
    A[comments java / check java 原Maven上下文] --> B[原POM多文件检查和输入核验]
    B --> C{输出性质}
    C -->|可定位原生warning| D[稳定源码任务与详细修复指引]
    C -->|插件缓存缺失或未知输出| E[环境或诊断准备任务]
    D --> F[task verify 原工具原范围复检]
    E --> F
    F --> G{原任务身份}
    G -->|同一问题| H[still_present]
    G -->|同文件同规则但新锚点| I[rule_coverage_requires_review]
    G -->|局部无诊断| J[candidate_absent_unverified_policy]
    H --> K[记录尝试，事实保持open]
    I --> K
    J --> K
```

统一入口仍为 `codeguard comments java . --maven-tool /absolute/mvn --java-home /absolute/jdk21 --maven-repo /absolute/offline-repo --repo-sha256 ACTUAL_DIGEST --format json`；复检为 `codeguard task verify CG-task-id .` 并显式提供同样的原工具上下文。替换路径和实际缓存摘要；CodeGuard不自动安装插件或降低规则。修复指引要求说明用途、参数、返回和异常，不能用裸标签替代详细说明。

新增封闭协议：Maven原生/工作台/复检0.2、项目0.5、comments未绑定0.9/工作台0.10、内brief0.6/预览0.3、任务预览0.32、聚合0.69/异常0.20。首次导入重算规则和投影并拒绝版本降级；复检核对已消费首次报告的摘要收据与原任务范围/规则，支持首次证据为复检包裹报告的新任务。零诊断不会自动关闭，可信关闭/复发仍待验收。

受控Maven进程输出完成五规则×成功/警告失败的公开检查、任务归并、原任务复检及修复后未受信消失回归；这不是实际插件诊断验收。本机已有Maven3.9.16/JDK21实际运行空离线库检查与环境任务复检，两次均识别Javadoc3.12.0插件缺失、没有源码问题。缓存缺失，真实插件详细描述4/3/1/0样例及警告失败配置验收尚未执行，独立条件测试保持待运行。完整Java详细行为契约、Checkstyle完整描述验收、57语言四核心、平台/宿主与可信关闭继续未完成；OpenSpec15.3/15.6不勾选，正式语法资格仍0/32。见[分项验收](../tests/acceptance/maven-javadoc-detailed-descriptions.md)。

## Checkstyle详细描述模块：源码实现，原生验收待完成

原配置的 `JavadocStyle`、`NonEmptyAtclauseDescription`、`SummaryJavadoc` 现可通过固定10.21.4静态适配，保留完整类名/短名、自定义ID、severity及各自属性。空描述开关、Java正则、首句/HTML、scope/tokens、标签token、摘要period/禁用片段和非紧凑HTML开关照原XML交给工具，不在Rust中替代原生检查。模块不能借用其它模块参数，未知token/来源和共享ID继续待解析；空period或摘要正则保留原生合法配置，Rust不以自己的正则语法判断Java正则。

统一入口：`codeguard lint java FILE --checker checkstyle --workspace . --config ORIGINAL_XML --java-tool EXISTING_JAVA --checkstyle-jar EXISTING_JAR --format json`。诊断进入稳定任务，`next` 给出详细用途、参数/返回/异常或摘要修复方向，`task verify CG-task-id .` 显式提供原工具/原配置复检。环境恢复产生的新源码任务也可据包裹首次报告复检；局部消失和恢复都不关闭任务。

```mermaid
flowchart LR
    A[原Checkstyle配置和原工具] --> B[原生XML与精确规则绑定]
    B --> C[源码修复任务]
    B --> D[环境准备任务]
    C --> E[next详细指引]
    D --> F[task verify恢复环境]
    F --> C
    E --> G[task verify原工具复检]
    G --> H[记录仍存在或未受信消失，保持open]
```

新协议为局部反馈0.5、工作台/源码复检/准备复检0.2、修复简报与预览0.25、源码任务预览0.33/准备任务预览0.34。历史schema不扩大，首次导入拒绝新配置伪装成工作台0.1；复检容器与scan版本配对。选中详细Checkstyle简报的聚合支持0.70，但本批实际公开聚合选择优先级更高的P3C准备任务，仍用0.58；0.70仅有构造序列化验证，不能称实际路由验收。另修正该实际聚合中不符合旧协议的Javadoc未配置原因码，现使用已有 `javadoc_checker_not_configured`，不虚构配置或运行。

受控XML进程夹具验证三类诊断、归并/修复复检、准备恢复及新任务复检；夹具不是Java或Checkstyle，不证明原模块语义或精度。当前未找到已有10.21.4自包含JAR，真实条件测试未执行；完整描述规则/配置/项目模型、独立误报评测、可信关闭/复发、57语言四核心与平台/宿主生产验收继续未完成，15.3/15.6不勾选，正式语法资格0/32。见[分项验收](../tests/acceptance/checkstyle-detailed-descriptions.md)。
