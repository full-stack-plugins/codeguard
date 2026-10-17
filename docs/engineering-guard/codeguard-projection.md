# CodeGuard公共结果投影：K07候选

沿用add-engineering-guard-core的EG-K07，不新建规格或替换旧报告。当前是CodeGuard本域的候选适配模块，未勾选K07；Ubuntu真实CLI smoke与独立审查尚待实际运行证明。没有修改CodeGuard旧检查、退出语义或修复工作台。

## 两个实际入口

`codeguard_cli::engineering_projection::project_raw(raw_bytes, optional_context)`分别处理：

1. `run_report`的精确1.0/1.1/1.2/1.3/1.4，复用既有`run_report::parse_run_report`。类型化completion/findings/coverage映射保留已确认发现，继续调用`codeguard_core::evaluate_delivery`，不重写质量规则。1.04等非规范拼法也拒绝。
2. 实际`check_command::run`输出的`check_feedback`，按已有0.38..0.73各具体schema文件识别，不存在/未知版本保持Incomplete。包含107份编译期白名单Schema的完整引用闭包；已有file/URN引用先改写到内存`$defs`，由GuardCore的`guard-contract::SchemaValidator`真实求值，不读取调用者路径或联网。`scripts/verify_engineering_schema_sources.py`只读核对全部引用与编译期清单。

不是把run_report构造样本当作真实check输出。实际check_feedback仍为local_unverified、未冻结必需义务；native_results原样保留在私有原件中，不从本轮扫描到的文件反推完整义务，更不把空public结果当零违规。现已对三项明确原生profile提取已确认发现：rust.cargo_clippy/rust_clippy_local_observation@0.2.0、rust.cargo_rustdoc/rustdoc_local_observation@0.4.0、rust.cargo_audit/rust_cve_local_observation@0.1.0。精确checker、制品摘要、批准原生规则映射、目标路径与输入摘要匹配后，blocking发现映射VIOLATION；覆盖始终Partial或Unknown，绝不生成SATISFIED。其他checker明确unsupported，原件保留且结果Incomplete。新增真实provider smoke使用缺工具Java/Maven及all选择，预期exit3，不安装SDK或初始化规格。

## 原件、上下文与身份

返回`ProjectionResult`借用原始字节，原字节SHA256与adapterVersion独立保存；没有用JSON重序列化替换原报告。原消费端可继续读取完全相同字节，新投影并列读取。解析拒绝未知report_type、未知版本、原始重复键、坏枚举/未知字段与矛盾旧状态；Malformed的投影及门禁明确Incomplete，不得到Allow。

`ProjectionContext::new`由控制端从K03冻结计划、TaskBinding、批准RuleDefinition、固定ProducerIdentity、本次`ProjectionRun`、已私有存储的原件EvidenceRef及完整`ProjectionCheckerPin`集合独立构造。每项pin明确批准verifier与NativeCheckerBinding、固定native版本及制品摘要，以及事前冻结的native_rules规则/影响映射；无映射发现留在私有原件并报告缺口，不能解释成零违规。依赖报告无锁文件路径时必须提供显式feedback_target且属于批准目标，不猜测Cargo.lock。报告tools声明与finding.tool_id须精确匹配，不能把一个工具的结果挂到另一个批准验证器。上下文字段私有且无Deserialize，不从候选报告复制批准范围。复核任务契约、动作、单仓库范围、精确路径、owner/verifier及原件摘要/长度；旧路径报告不能证明selector或跨仓库范围，所以明确拒绝。本地Rust调用者仍可构造类型声明；这不是K09信任根，控制端必须核验真实批准、生产者、实际候选与运行来源后才可提升信任。

公共结果的runId/operationId/sequence、startedAt/finishedAt/exitCode来自明确运行上下文，不取当前时间补缺，不把Completion::Complete直接当实际进程完成。缺上下文返回Incomplete并保留原件/发现；Completed缺时间或退出码时上下文拒绝。原件摘要相符仅证明字节完整性，不独立证明该报告确在当前Git候选上执行。K04/K06和受信控制端仍须提供并核对这段绑定。

## 执行、判定、覆盖与信任分离

- 原生blocking发现即使请求incomplete或超时也保留；有确定违规映射VIOLATION，未知影响保持UNKNOWN，修复建议不执行。
- CoverageComplete要求明确批准目标集合与报告expected/observed逐项相等、完成性有证明；缺模块/不完整/无法证明保持Partial或Unknown，保留完整批准义务与缺口ID。
- SATISFIED须真实Completed、完整覆盖且无发现；不适用的reason文字不能伪造适用性证据。
- 原生severity包括UNKNOWN原值保留；公共Severity取批准RuleDefinition，不把未知native severity臆测成Info。
- 返回信任固定UNVERIFIED；local_unverified、trusted-ci文本、旧allow、exit0、解析成功均不设置trusted_bindings_verified。局部not_evaluated仍NotEvaluated；完整项目即使旧报告自报Allow，缺受信绑定重算仍Incomplete。输出不签发GateDecision ALLOW或ExecutionGrant。

专业映射只在codeguard-cli；它依赖core/runtime/adapters与GuardCore通用protocol/contract，GuardEngine不依赖CodeGuard。crate_boundaries增加严格CLI-normal-only边界测试，core/runtime/adapters不能引入这些共享库。不修改既有cfg来伪称Windows已支持Unix provider。

## 本地验证与待完成证据

Windows使用`crates/codeguard-cli/tests/engineering_projection_harness`独立包，直接包含相同生产模块和既有run_report/check_request源码，依赖真实core/runtime/adapters。原因是旧full-cli在Windows包含既存Unix-only引用，不能把它们改成空实现或绕过cfg。该harness不复制投影算法，也不作为发布产品。

实际TDD先看到缺少project_raw API的编译RED；随后未知schema/重复键的gate误留NotEvaluated行为反例实际失败，修复为Incomplete后通过。历史回放包括原始cfquery-comment-boundary-2026-10-05.json、归档0.56报告的native_results及0.72实际C-family报告；后两者从归档容器取报告子对象后作为回放输入，不冒称新运行或原归档文件逐字节采集。

Windows不能据此宣称真实Unix CLI已执行。CI早期步骤在SDK准备和全语言矩阵前运行：

```text
python3 scripts/verify_engineering_schema_sources.py
cargo test --locked -p codeguard-cli --lib engineering_projection::tests -- --test-threads=2
cargo test --locked -p codeguard-cli --test engineering_cli_smoke -- --nocapture --test-threads=1
```

本地focused命令：`cargo +1.85 test --locked --offline --manifest-path crates/codeguard-cli/tests/engineering_projection_harness/Cargo.toml`。不得用harness成功替代Ubuntu实际provider运行，CI配置文件存在也不等于已运行成功。K06接线、K07真实CI与完整专业结果消费仍需后续验证；不勾任务，不宣称P0可交付。

当前Windows focused源码回归29项（18项投影与11项真实装配单测）、分层边界8项通过；107 Schema引用闭包独立检查通过。最终fmt/clippy另按实际命令核对，Ubuntu smoke仍待运行，不作已通过声明。

## 实际原生报告的能力边界

实际Clippy/rustdoc反馈缺少tool_version、冻结义务全集/完整expected集合和可用于公共执行声明的完整墙钟元数据；这里只能比较独立pin的精确制品摘要并保留版本缺口，实际时间仍要求外部运行上下文。source_sha256仅与已捕获目标逐项比较，不构成K04实际采集或生产者认证。cargo-audit报告另缺锁文件路径、独立可信数据库时间/来源证明；路径只能由宿主显式冻结，数据库未核验作为公开limitation保留，不宣称完整漏洞覆盖。未知severity（真实报告中为unverified）原值保留，公共严重级别来自独立批准规则。

新增实际TDD先回放clippy-documentation-aggregate-native-wasm.json中含违规的check_feedback：确认发现丢失（0而非1）实际RED；提取后GREEN。cargo-audit-path-discovery-2026-10-06.json真实未知severity回放、缺规则映射、错制品摘要、错源码摘要、缺显式锁路径均有负例；这些反例始终Incomplete/Unknown，不变成clean或提升UNVERIFIED。历史报告子对象回放不是当前候选新执行的证明，Ubuntu实际CLI采集仍须独立验证。

实际报告副本local_scan_complete=false的闭合schema仍有效；该标志只描述扫描完整性，已绑定工具/规则/源码摘要的阳性诊断保留为VIOLATION，覆盖Partial，Execution沿用实际运行上下文。该用例先观察0条丢失的RED，再修为只记录扫描缺口而保留诊断。Java/Maven继续扩展以下两项专业类型化映射，不以三项Rust profile缩小EG-K07范围或勾选任务。


## Java/Maven专业映射与独立本次源码证据

Java明确支持java.maven.javadoc（java_javadoc_project_probe@0.5.0 → maven_multifile_probes[].observation maven_javadoc_multifile_probe@0.2.0）与java.maven.p3c（java_p3c_project_observation@0.2.0 → files[].observation java_p3c_local_feedback@0.2.0）。不递归搜索任意findings字段。Javadoc复用codeguard-adapters的parse_detailed_maven_javadoc_output；P3C复用parse_pmd_xml和p3c_rule_in_selected_rulesets，并沿用现有has_projectable_findings对原生失败保留已确认阳性的条件。没有运行新的质量规则或扫描文件反推批准义务。

宿主先通过ProjectionSourceProof::new(plan, run, exact_target, native_path, source_bytes, private_evidence_ref)提供实际捕获源码；适配器校验私有引用hash/size、精确冻结目标，并绑定runId/subjectDigest。它不进行文件采集，不认证宿主。Context::with_java_execution(checker_id, raw_log, private_log_ref, native_exit, parser_version, source_proofs)再绑定原始日志，拒绝异run/subject/目标和重复路径/原生路径映射。Java原生log通过既有解析器获得的诊断与报告行逐项匹配，工具制品与批准规则映射也须一致；原始报告行号或JSON source字段不能独立自证。确认后输出VIOLATION、Partial和UNVERIFIED，并携带报告/日志/源码私有EvidenceRef；仍无完整规则覆盖证明。

历史0.69 Maven报告确有JavadocEmptyComment，但无独立本次源码/日志原件时，仅产生UnboundNativeDiagnostic；该独立类型集合不进入已确认Finding或VIOLATION。受控新源码+日志的正例复用实际报告格式测试解析器绑定，不冒称是历史源码采集，也不声称真实Maven已执行；错run、错字节和日志无法匹配均拒绝或保留unbound。P3C受控XML测试验证失败执行仍保留原生阳性、错source摘要转unbound；不是当前Maven/P3C实际运行证明。

尚待Ubuntu实际控制端捕获本次Java/Maven源码/原生日志、与K04/K06 subject和run联动的smoke。Java依赖/CVE及其他native profiles暂明确unsupported，不宣称全语言结果适配完整；任务EG-K07保持未完成。


## 同次Maven捕获支路与真实Linux验证入口（尚待执行）

原有maven_javadoc_probe已在同次调用持有SourceSnapshot和ProcessOutcome，但只返回Value，私有scratch退出即删除；不能把另一次mvn输出挂到CLI报告。现增加显式check_command::run_observed(args, observer)，旧run/run_lint_all仍传None，观察者不改变同版本原生报告和退出路径；当前顶层wire另按下述0.73修正。observer逐层传入java_javadoc_scan和maven_javadoc_probe，仅在原有输入未变、工具身份复验、诊断位置检查之后，给出同次子报告、源码原字节/原生临时路径、stdout/stderr/终止原因。MavenNativeCapture无Serialize；不向产品JSON增加原始日志，不宣称回调已将证据持久化。回调Err/可展开panic由ObservedCheckOutcome.capture_errors独立返回，不抹掉原产品报告。

捕获时间是调用run_process_recorded之前与返回之后的实际观察边界；前者不是OS精确spawn时刻，后者包含私有日志落盘时间。本库未提供OS级进程trace或K06执行资格，不能用observer调用时刻补造spawn。真实smoke公开结果仍UNVERIFIED，并将观测范围明确保留在测试说明中。

新增engineering_maven_smoke真实Unix测试显式要求既有Maven/JDK21和隔离Maven工件缓存；JDK21是旧Javadoc profile原有要求，未降级为JDK17。CI早期gate步骤从预装JAVA_HOME_21_X64和mvn路径取工具，不安装用户SDK/CLI。可选warmup只为隔离缓存解析插件工件，其日志永不作为后续检查证据。smoke使用本次真实guard-subject Worktree捕获、实际check run_observed和原有--output私有导出，对比capture.native_report与同次check_feedback observation；运行后再次verify_current，私有保存本次source/stdout/stderr，创建对应EvidenceRef后调用公共投影。成功回调与回调拒绝各自使用自己的同次Maven执行，绝不复用另一轮日志；exit0仍不授信。

本批新增/修改范围：maven_native_capture.rs、maven_capture_observer.rs、observed_check_outcome.rs；check_command.rs、java_javadoc_scan.rs、maven_javadoc_probe.rs、maven_javadoc_task_recheck.rs、task_verify_command.rs及lib.rs的最小传递/默认None；engineering_maven_smoke.rs、CLI dev-only guard-subject依赖与锁文件、ci.yml独立步骤。稳定engineering_projection生产模块本批未改。Windows只验证17项现有投影回归、格式和静态完整性；新Unix调用链与真实Maven smoke尚待实际Ubuntu证据，不勾EG-K07，不把它当Q01/K06生产全链路完成。

```text
cargo test --locked -p codeguard-cli --test engineering_maven_smoke -- --ignored --exact real_maven_same_execution_projects_product_feedback_with_captured_sources_and_logs --test-threads=1
```

该命令使用默认full-cli feature，必须在Unix、有真实JDK21/Maven且工件缓存明确准备后执行；缺工具时失败，不静默skip成PASS。


## 0.73精确wire修正

真实Ubuntu CI38964163817证明Rust1.85编译通过、生产投影17项通过；真实缺工具CLI输出随后被闭合Schema拒绝，Maven smoke尚未执行。原因是当前native_results装配已采用“未观察槽位缺省”的明确语义，却仍声明历史0.38..0.72要求必需null槽位的版本。直接复用真实assemble/finalize对Maven历史子报告结构重放，已实际RED证实该矛盾。

当前check和lint-all生产输出统一声明精确0.73；保留既有命令和退出语义，属于顶层wire版本修正，不能再声称新旧报告字节完全不变。旧0.38..0.72 Schema及历史原件保持不变，历史报告仍双读。0.73使用最新闭合顶层字段、固定注册语言枚举和显式lint请求约束；native_results仅取消未观察槽位必需性，属性白名单仍闭合，每个存在的payload精确引用既有已支持原生版本。未知顶层/原生属性、坏原生版本、缺必需头字段、未知或非规范版本继续拒绝；缺槽位不等于零违规或覆盖完成。

compiled Schema闭包现107份。Windows focused回归增加真实生产装配函数与其既有单测，共30项，包含root新增的产品注册表与协议selection枚举全集一致性检查；明确保留Kotlin无源码时槽位缺失的现有断言，没有把新语义改回假null。旧归档不经过新producer重写；所有版本expectation修改只针对当前实际CLI/npm包输出。实际0.73 Ubuntu CLI/Maven smoke还需新CI证明，EG-K07仍不关闭。

### P3C 项目报告精确版本修正

真实 Linux CI38972395021/job117213637719 的受控 CLI 原件已保存在 `tests/acceptance/evidence/k07-cli-073-controlled-java-2026-10-17.json`（8824 字节，SHA256 `83770811649c0ab1bb8ce1eabc6fac74eba9e1206b7f835ec2c7484bb5a19360`），并固定 Git 原始字节。精确失败来自 `java_p3c.files[].configuration_reason` 和 `configuration_next_action`：生产器输出了新增字段，但仍声明原生项目版本 0.2；其旧闭合 Schema 不包含这两字段。

现仅将 `java_p3c_scan::observe_project` 的项目报告升到 0.3，新增独立 `java-p3c-project-observation-v0.3.schema.json`；0.73 显式支持该新身份，继续支持历史 0.2 原定义。新项目和逐文件对象保持闭合，两项新增配置说明均必需且精确为 string/null。独立 `java_p3c_local_feedback` 仍是 0.2，没有改其身份或旧 Schema。已有坏版本原件不被修写、放宽或升级为通过；回放保持拒绝并保留全部原始字节。测试中的新 wire 仅将该准确报告的项目版本改为 0.3 来验证新定义，同时断言类型错误、未知字段、冒称旧顶层版本仍拒绝。

`feedback_java` 接受两个明确项目身份，保留既有工具/源码证明、PMD 版本匹配及部分执行阳性逻辑。有效结构的缺配置报告仍 `UNVERIFIED`、`incomplete`、交付门禁 `NotEvaluated`；不产生批准或覆盖完成。内存闭包增加到108份。真实 Unix smoke 新增对当前生产器原生0.3的断言；Windows仅执行精确报告回放与生产装配/投影模块，不声称已运行Unix CLI或真实Maven smoke。EG-K07仍待新CI验证，不勾选。

同一原生身份也必须适用于内部失败/取消路径。实际 `aborted_task_report` 装配逻辑移至现有纯 `check_report_assembly` 模块，Unix入口仍调用该唯一函数；只在保留的P3C项目报告为0.3时输出新 `check_aborted` 0.23。新闭合中止定义沿用0.22的错误/取消/导出语义，允许现有注册表选择，保留原先基础槽位必需性，C文档槽位仍仅在实际存在时出现；原生负载引用0.73的显式白名单并要求P3C精确0.3。历史0.13..0.22 Schema和选择分支保持。此支持只用于原生wire装配/验证；公共投影仍把 `check_aborted` 作为不支持的原件保留，不颁发执行或交付资格。

新增实际装配回归先真实失败（错误选中0.13），再通过0.23内部失败与取消两支，核验精确闭合Schema、完整原生P3C保留、未知字段拒绝、旧版本冒称拒绝及不授信。当前内存Schema闭包为109份。该测试直接执行从原入口抽出的实际装配函数，不能冒充Windows已执行Unix CLI或真实原生进程。

本地 Rust1.85 完整 focused harness 最终33/33通过（27.66秒），harness all-targets Clippy `--no-deps -- -D warnings`、受影响文件fmt、layering与109份闭包核验均通过。额外crate_boundaries构建因E盘空间不足而未执行，不能记为通过；测试环境缓存清理被自动审批策略拒绝，未绕过限制。最终Linux生产CLI和同次Maven验证仍待新CI；不因本地回放通过勾选EG-K07。


### P3C 修复复验消费者与精确外层协议

项目报告0.3的消费者同时包含任务分类、已保存复验事件、next候选与work sync。它们现共用从原入口抽取的 `java_p3c_task_recheck` 分类/身份函数，精确读取历史0.2、当前普通扫描0.3和任务复验0.4；未来版本、错误checker、错误覆盖声明或不匹配事件分类仍拒绝。既有文件摘要、工作区、原件与输入证明检查留在各调用方，不用版本匹配替代这些检查。复验输入不稳定仍incomplete；既有违规仍still_present，缺违规仍需覆盖审阅，不会关闭任务或授信。

当前任务入口在真实输入稳定性检查后输出项目0.4，显式要求boolean `task_input_stable`，外层 `task_verification_preview` 精确0.40；普通0.3没有该复验字段。任务入口不执行backlog sync，0.4不要求伪造backlog元数据。当前单文件信封使用 `java_p3c_file_feedback` 0.2并仅嵌入已绑定的项目0.3；独立 `java_p3c_local_feedback` 0.2保持原语义。三个新Schema均闭合，旧版本与历史原件未改；登记冻结表同步显式记录新身份。编译期Schema闭包现112份。

实际生产分类测试先在native0.3失败，再验证历史/当前读取及保存事件一致性；生产绑定/文件信封函数与闭合Schema互验，覆盖未知字段、缺稳定性字段、错嵌套版本和伪造接受分类。Windows Rust1.85 focused harness最终35/35通过（24.96秒），all-targets Clippy（不检查依赖告警）、112份闭包、layering及8项crate_boundaries均通过。前次E盘容量阻断之后，boundary已在独立D盘target实际完成，未清理旧缓存。

Ubuntu早期CI新增既有 `single_file_lint_and_project_check_reuse_the_same_p3c_task_and_native_recheck` 生产入口用例，并断言文件0.2→项目0.3→任务0.4/外层0.40和原有still_present/事件保存/not_evaluated。该集成用例复用已有受控Maven输出夹具，证明真实CLI修复链消费者；它不冒充真实Maven工具资格或同次真实Java执行，后者由独立smoke继续验证。EG-K07仍未验收。


### 真实 Maven CI 规则映射诊断

CI38989061198/job117267115339已实际执行同次Maven并捕获一次callback，退出0，产品原生报告包含 `JavadocEmptyComment` 和 `JavadocDefaultConstructorMissingComment`；测试的预先冻结清单只有EmptyComment/MissingComment，导致默认构造器诊断按既定生产边界正确保留unbound，随后“所有诊断已绑定”的断言失败。这一轮不能记为完整smoke通过。

smoke现与受控投影回归共用固定 `engineering_maven_smoke_native_rules.json`，明确预先声明上述三条本夹具适用规则，不从扫描结果动态生成批准清单。未改生产解析/确认边界，未移除原empty断言或跳过诊断。新回归先用旧两条规则真实复现相同unbound失败，加入第三条后通过，并反向删除第三条再次验证1项确认+1项unbound；仍Partial/UNVERIFIED。受控日志只用于parser/source绑定单测，不冒充CI私有日志或另一轮原生运行证据。

受影响Windows Rust1.85完整harness现36/36通过（22.47秒），all-targets Clippy `--no-deps -- -D warnings`、两处Rust格式检查、layering、diffcheck通过。修正后的真实Unix同次Maven仍需新CI重新执行；EG-K07保持未验收。
