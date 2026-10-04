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
