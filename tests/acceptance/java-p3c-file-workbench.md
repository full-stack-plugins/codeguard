# P3C 单文件原生入口到修复工作台验收

日期：2026-10-04。规格为唯一 change `introduce-rust-codeguard-cli` 的 remediation-workflow / Bound single-file P3C lint SHALL reuse project remediation identity，追踪 RW21；关联 6.2、9.3、9.4、9.7、9.13、12.7。原父任务的完整工具、规则覆盖和闭环验收保持未完成。

## 原缺口和纠正行为

已初始化项目的 `check java` 已生成稳定 P3C 任务，但 `lint java FILE` 原先只输出未绑定反馈。现在显式 P3C 原生入口或已有原生上下文会选择最近已有工作台，也支持 `--workspace ROOT`；只读取祖先 POM 和所选源码，不发现/执行兄弟文件。与聚合入口共用保存/同步、finding 身份和原工具任务复检服务，没有第二套任务队列或为了同步再次运行 Maven。

最近子 POM 未配置/未知时不使用父 POM 或默认十个规则集。既有未绑定局部反馈 0.2 保留；已绑定入口新增 `java_p3c_file_feedback` 0.1，内部仍是项目观察 0.2。显式 `--checker p3c` 原先在 WASM 构建中也会转入语法初检，现保持原生选择，缺上下文成为准备阻塞；默认无上下文候选 WASM 路径仍可用。

## RED 与实际回归范围

新 [CLI 回归目标](../../crates/codeguard-cli/tests/java_p3c_workbench.rs) 使用受控 Maven 协议脚本、JDK/仓库身份夹具，不伪称实际运行了 P3C 规则引擎。首次七项反例实际全部 RED；父目录链接、显式检查器在 WASM 下的错误路由、注释规则被写为命名修复各有独立 RED。

十三项当前场景覆盖：单文件/项目检查归并为一张任务及原生 task verify，单文件无兄弟执行，未选规则不作为 finding，最近未配置 POM 遮蔽父规则，损坏最近工作台不跳过/重建，保存失败保留诊断且无下一步，越界/文件链接拒绝，父目录链接拒绝，零诊断保留 open，未绑定兼容且不初始化，human 展示原生规则和真实任务，显式 P3C 不转 WASM，以及注释规则任务不指导命名修复。

任务七项内容实际逐项断言：问题证据、规则依据、允许范围、修复步骤、复检 argv、历史尝试、关闭条件。共用任务模板不再把所有 P3C 诊断称为命名规则；复检 argv 明确保留 P3C 选择和工作区。

最初绿色实现中两次失败断言分别误用了 blocker kind 和 finding state 的字段值；依据既有协议改为 `blocker` / `state=open`，不是改变产品行为绕过验收。

## 报告协议

[新 schema](../../schemas/java-p3c-file-feedback-0.1.schema.json) 限定单文件、已绑定项目、内部原生局部反馈及未批准身份；保存失败不得带下一步，宣称已同步必须含项目观察/next。内部规则子集可为 1–10，不修改旧未绑定 schema 的十规则集契约。实际报告及伪造报告校验结果在最终终态下补充。旧 schema 文件保持原字节。

## 验收执行与剩余边界

首次完整默认回归与 WASM 特性构建并行共享 `target/debug/codeguard`，默认测试调用到了 WASM binary，导致 `work_sync_contract` 的缺配置任务数量断言失败；同轮 WASM 用例正确通过。这是构建/测试执行污染，不算产品缺陷或通过。最终验收改为串行完整默认回归，再运行特性目标，不在运行中的检查之间覆盖同一 binary。

真实 P3C/JDK/离线依赖闭包的 ignored 用例本轮未执行，不计算为原生规则验收。本机默认 Python 不含 jsonschema，校验使用已存在的 Anaconda Python，不安装依赖；Python 仅用于开发验证，不进入产品运行时。

仍需完整 P3C 56 规则、项目生效配置/参数/源集、多模块原生覆盖、可信关闭/重开、原生 oracle 精度和真实宿主验收。公开 npm 0.1.4、插件锁及 grammar 字节未变。已有 Erlang RED 草稿不在本变更范围内。

## 最终终态

串行默认全工作区退出 0：213 组、1185 passed、0 failed、109 ignored。随后六个 WASM 特性相关目标退出 0：71 passed、0 failed、10 ignored，其中新单文件目标 13 passed/0 failed/0 ignored；两组重叠结果不合计。

198 份 schema 元定义通过；本轮当前 binary 的 13 份实际单文件反馈通过新 schema，12 类伪造结果拒绝。其余捕获的 init/check/verify/旧局部反馈不重复计算为本项 schema 验收。197 份旧 schema 未修改。全工作区/全目标 WASM Clippy `-D warnings`、fmt、分层、OpenSpec strict、git diff 与修改文档链接检查通过。

此前 f158035 的 Linux CI 37172785235 已 completed/success；本变更远端结果须按新提交独立核验，不能借用此前成功。

日志 SHA-256（当前本机具体执行，不是独立精度或发行认证）：

- `/tmp/codeguard-p3c-workbench-red.log`：`8270f9b39a77e6e67c57ad281fbbf71687c0a60259628ed3f69d37095e1cf11c`。
- `/tmp/codeguard-p3c-workbench-parent-link-red.log`：`ce8a1d0b19732ac694064b1b894f58572e865a519a574f24165a063dc358b08d`。
- `/tmp/codeguard-p3c-workbench-checker-red.log`：`ed0910374ea7dc3c5a09852fff0b36e3846d3c1034e65ff44b9c240a7d91e6e7`。
- `/tmp/codeguard-p3c-workbench-comment-red.log`：`89b6f05fc1cd4079bfc0dccaf45a75272e135ab27fc76a80858c17692d326a7e`。
- `/tmp/codeguard-p3c-workbench-workspace-current.log`：`07b3457d0769aea940ec39c33901553aff807e228923e2a09fde51ee80e751b0`。
- `/tmp/codeguard-p3c-workbench-feature-current.log`：`7ac6a182ab75ada122ffd965489addd4110d3ce7a01f20af399311803218a137`。
- `/tmp/codeguard-p3c-workbench-schema-current.log`：`bf5e5e29e85a3703f4c8aab0c9e5355b01e7c2030b53cce18fc41d77fe769705`。
- `/tmp/codeguard-p3c-workbench-clippy-current.log`：`97bbf85f3832645bfd084760a5af94d542c8752a1aefdace71865736689800a0`。

可复现：先串行执行 `cargo test --workspace --all-targets --locked --offline`，结束后再执行 `cargo test --locked --offline -p codeguard-cli --features wasm-precheck --test java_p3c_workbench --test java_syntax_fallback_candidate --test java_p3c_cli --test check_all_java_p3c --test work_sync_contract --test next_command_contract`。如需导出实际报告，可设置 `CODEGUARD_P3C_WORKBENCH_REPORT_DIR` 为独立验证目录；该路径仅供测试使用。
