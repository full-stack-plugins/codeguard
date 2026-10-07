# CodeGuard OpenSpec未完成任务交接快照

> 日期：2026-10-07；基线 `a30f3efc32e69bd50d557ff3c7d26836d9a229ca`；66已完成/288未完成。
> 源文件SHA256：`6430dccc6838bce31e9c566e4cc268fe73642b6a27fe22a13e80c50bd829405c`。
> 原任务内容保留，Markdown相对链接按交接目录重新定位。只读索引，不勾选此文件。必须回到 [正式tasks](../../openspec/changes/introduce-rust-codeguard-cli/tasks.md) 阅读相邻进展后执行；下列原文中的旧codeguard-cli路径/历史阶段不保证仍是当前实现状态。

## 1. S01 契约与工程基线

- [ ] 1.6 固化应用服务与ports边界：观察/政策/计划/检查/交付/存储/租约/修复及schema所有权；交付：真实领域契约和依赖测试；验收：CLI/MCP共用服务、core无基础设施依赖，新CLI与验收工具为Rust实现，P3C/Maven等通过Rust runtime调用其原生命令，不以空stub计功能完成。

## 2. S02 CLI、结果模型与门禁

- [ ] 2.1 实现统一命令语法、canonical ID/别名及只读 plan；验收：错参无进程副作用、plan 不运行构建/扫描。

- [ ] 2.3 实现 findings 与 completion 双维聚合及新退出码；验收：混合违规/缺工具返回 3 并保留全部发现，取消/内部异常优先级正确。

- [ ] 2.4 实现义务账本和交付决策；验收：类别命令、空目标、缺 adapter、未执行任务均不能签发项目 allow。

- [ ] 2.5 实现统一 human/JSON/SARIF 渲染与私有证据引用；验收：结构化 stdout 纯净、SARIF 未完成可见、公开报告不泄露凭据。

- [ ] 2.7 实现 C01–C36 命令注册、help/版本元数据、操作结果类型与参数支持矩阵；验收：文档/help/CLI/MCP映射无漂移，查询/计划/安装成功不能变为质量allow。

- [ ] 2.8 固化每类命令timeout/jobs默认值、来源与总deadline；验收：子进程/重试不重置预算，非法预算执行前拒绝，清理未完成真实可见。

- [ ] 2.9 实现报告原子导出、可逆路径编码、多位置/包定位及来源保留；验收：output不可写为3且原finding保留，未知major拒绝，不按有损路径或相似文案跨工具抵消发现。

- [ ] 2.10 升级 RunReport 与所有公开消费者的白名单处置协议；验收：新版本明确 raw/active/whitelisted finding、批准引用和 allow_with_exceptions，旧消费者拒绝未知决策而非降级为普通 allow，human/JSON/SARIF/MCP/Hook 语义一致。

## 3. S03 运行时、快照与缓存

- [ ] 3.2 实现并发流读取、总 deadline、输出预算和私有原子日志；验收：洪流/编码错误/日志 symlink 场景无假完整、无越界写。

- [ ] 3.3 实现 Unix 进程组与 Windows Job Object 的取消/终止/回收；验收：子孙进程、超时、Ctrl-C、排队取消有平台实测。

- [ ] 3.4 实现任务 DAG、资源锁及依赖失败传播；验收：独立任务继续、共享 build 目录互斥、失败依赖未完成可见。

- [ ] 3.5 实现工作树/index/ref 快照与原始字节校验；验收：SHA-1/SHA-256、坏批响应、特殊文件、symlink/gitlink/LFS 均明确处理。进行中：相邻 Rust CLI 已按 NUL 协议读取真实 index、前后复核列表，并对有界普通 blob 独立核验 SHA-1/SHA-256 Git OID、记录脱敏字节摘要；symlink/gitlink/LFS、超预算和对象读取失败明确 unresolved，见 `codeguard-cli/tests/acceptance/git-index-safety-preview.md`。工作树/ref 快照、坏批响应注入测试、特殊类型完整语义及 Git 工具身份未完成。

- [ ] 3.6 实现 GIT_INDEX_FILE、初始提交、worktree、多 ref/non-HEAD/删除 ref push 输入；验收：真实临时 Git 仓检查准确且 index 不变。进行中：相邻 Rust 的真实 index 路径安全预览覆盖初始仓库、替代 `GIT_INDEX_FILE` 与默认 index 不变；worktree 和 pre-push 多 ref/删除 ref 尚未实现，不勾选。

- [ ] 3.7 实现执行前后内容身份复核和源码副作用检测；验收：并发编辑或检查器改源码导致 incomplete。

- [ ] 3.8 实现严格缓存及义务等价证明；验收：同 mtime/size 内容替换、规则/依赖/工具/库变化必失效，软缓存不可直接认证。

- [ ] 3.9 实现run/obligation/task/attempt关联轨迹与分阶段计时、缓存/终止原因；交付：脱敏结构化本地事件；验收：部分失败可追溯，公开轨迹不泄露原始argv/env，不默认上传遥测。

- [ ] 3.10 实现证据索引、私有权限、引用与保留策略及受管清理；验收：活动run/lease、被引用证据、用户源码和tracked历史不被清理，磁盘/权限/symlink失败不扩大删除或伪造完成。

- [ ] 3.11 实现离线与可信执行边界port及能力探测；验收：主动联网子进程被拒绝，无法保证时启动前incomplete；可信政策/签发身份不进入项目脚本执行域，真实平台证明由S11/S12补齐。

## 4. S04 规则与策略权威

- [ ] 4.1 实现运行配置与批准质量策略的独立解析及 config explain；验收：CLI/env 无法弱化 required/threshold/exclude。

- [ ] 4.2 实现旧 codeguard.json 显式迁移和原生 suppressions 差异解释；验收：未知/损坏字段不静默丢弃或按默认通过。

- [ ] 4.3 实现 rulepack manifest、来源/许可、稳定规则 ID、摘要和兼容锁；验收：内容篡改/不兼容组合不能执行认证。

- [ ] 4.4 实现基线仅分类 new/existing；验收：未修改文件中的存量违规仍阻断，基线失败不消除 finding。

- [ ] 4.5 实现可信政策修订和例外输入校验；验收：agent 自写批准、无到期、过期、错内容或错范围凭据不生效，批准例外不显示普通 PASS。

- [ ] 4.6 保留点前缀默认策略及两项例外，生成汇总覆盖；验收：F18 全部成立，无未授权的新排除。进行中：相邻 Rust `detect` 0.3.0 已汇总普通发现的点前缀排除根、配置例外文件及未评估入库安全状态；`gate pre-commit` 路径安全预览能在真实 index 看到点前缀 `.env`，见 `codeguard-cli/tests/acceptance/{dot-prefix-scope-baseline,git-index-safety-preview}.md`。完整安全内容扫描与正式 Git 门禁未接线，不能勾选。

- [ ] 4.7 接入 rules list/config validate/explain 命令；验收：有效规则、原生suppression与批准来源可解释，静态验证不执行项目脚本，配置错误为未完成而非源码违规。

- [ ] 4.8 实现精确误报白名单 schema、可信来源解析和 Rust 匹配器；验收：仅同一原生规则、目标/内容、工具/规则包/适配器及有效批准身份命中；通配、过期、冲突、自批和坏报告均不放行，原始 finding 保留。

- [ ] 4.9 接入 `rules whitelist list/explain/propose` 与 config validate/explain；验收：propose 只从本轮完整原生 finding、工具/适配器/rulepack 身份和内容复核生成候选；缺任何身份则报告未完成与补证据动作，不能填占位摘要；查询解释批准与失配原因，普通项目文件不能自授权，系统性误报走经批准的规则修订。进行中：相邻 Rust CLI 已提供显式候选文件的只读 `rules whitelist list/explain`；explain 可对显式提供的发现身份解释精确匹配或失配，观察身份与批准来源均未核验，始终标记 `authority=unverified`、`gate_effect=none`；坏文件、重复 ID、缺失目标均不能呈现为批准。`propose` 已能从本地稳定任务返回缺本轮身份的未完成预览，固定 candidate=null、无门禁效果，见 `codeguard-cli/tests/acceptance/whitelist-propose-incomplete-preview.md`。`config validate/explain` 已有只读候选解析但不能解释可信批准、期限或本轮 finding；自动发现与完整候选生成仍未完成。

- [ ] 4.10 实现白名单纠错闭环与规则级误报升级路径；验收：候选/驳回/批准/过期/撤销/失配关联稳定 finding 和任务，已批准例外不伪装为代码修复；误批或过宽条目采用旧决策撤销、新候选引用旧决策、独立批准的修订链，冲突或未批准期间不得放行，原任务重开；纠错提案含原决策/稳定 finding/结构化原因/本轮复检引用，替代候选有新 ID 与 replaces_decision_id；只撤销不制造替代、引用链无环、旧新原子切换、扩大范围拒绝；同规则多目标误报生成规则或适配器根因调查，须以正反例和受保护策略修订处理，不自动扩大白名单；评测保留原始发现和人工裁定误报率。进行中：相邻 Rust 快照协议 1.1 可固定撤销 ID，绑定时撤销优先于旧字节命中；可信发布、修订链事件、真实门禁和任务重开仍缺。

## 5. S05 工具适配框架与准备

- [ ] 5.1 实现 capability/discover/resolve/plan/parse/coverage/fix 协议与编译期注册；验收：adapter 不绕开运行时启动进程或联网。

- [ ] 5.2 实现工具解析、二进制身份、doctor 和 tool lock 验证；验收：wrapper/受管缓存/系统工具均匹配锁，缺工具返回恢复步骤。

- [ ] 5.3 实现独立 tools install 流程、下载清单与校验；验收：普通 check/plan/doctor 不隐式安装；真实主动联网 wrapper 在离线隔离中被阻止，无法隔离则启动前 incomplete。

- [ ] 5.4 实现报告产物 freshness、scope/rule 执行覆盖核对；验收：陈旧或伪造空成功报告不能通过。

- [ ] 5.5 建立 adapter conformance harness；验收：F01–F10、F17 的有效与畸形报告可独立复用，mock 与真工具证据分层。

- [ ] 5.6 实现 tools list/verify/install 的库存、制品身份和默认预览/显式安装边界，以及doctor准备报告；验收：身份通过不等于可启动，安装部分失败可恢复，准备证据带run_id并可幂等同步。

- [ ] 5.9 实现动态构建模型解析任务与共享扫描义务映射；验收：detect/init/plan仅保留待解析条件，执行期经统一runtime留证据，多语言/多构建根不丢范围，adapter不私自I/O。

- [ ] 5.10 实现项目检查器配置探测；验收：按构建根识别 Javadoc、依赖/CVE、lint 与安全检查器的 `configured/missing/invalid/unknown`，说明配置位置与启用建议；28 个检测族只作候选目录，不自动生成全部必需义务。进行中：相邻 Rust `detect` 已只读解析 Maven POM 的六种检查器声明，保守处理父 POM、pluginManagement、profile 和 Gradle；另沿 Python 文件层级识别 Ruff TOML 的配置优先级、lint/format 区别及坏配置，见 `codeguard-cli/tests/acceptance/{maven-config-discovery,ruff-config-discovery}.md`。其他生态、Maven effective model、项目全量执行与宿主反馈接线仍未完成，故不勾选。

## 6. S06 Java 全链路

- [ ] 6.1 实现 Maven/Gradle/JDK/wrapper/profile/module/source-set 观察；验收：多模块、父 POM、动态未知范围与错误 JDK 有真实样本。

- [ ] 6.2 确定并锁定官方 P3C/PMD/JDK 兼容组合，实现原生报告解析；验收：正反例证明规则实际加载，不能用 Checkstyle 名称替代。

- [ ] 6.3 实现 Checkstyle/Javadoc 注释检查；验收：公共 API、参数返回、inheritDoc、record/Lombok/生成代码正反例。

- [ ] 6.4 实现 Java 安全静态检查及 CVE 依赖图适配；验收：真实安全/CVE 样本、坏报告、库过期与模糊匹配分开记录。

- [ ] 6.5 实现 build 等级和测试执行声明；验收：默认静态构建不谎称测试通过，要求测试的策略不可自动跳过。

- [ ] 6.6 接入 Java 完整义务与影响闭包；验收：verify 未绑定质量任务、自定义 echo、50/51 文件、未修改调用方失败均不假通过。

- [ ] 6.7 用代表性 Java 项目完成 check java/check all/doctor/修复前后对照；验收：每条旧新差异人工裁定并有产物引用。

- [ ] 6.8 识别 Java 依赖治理及 CVE 检查配置并调用已配置的原生工具；验收：区分依赖图/版本/许可证/SBOM 与漏洞诊断，未配置时给智能体具体建议；动态版本、父 POM 或私服不可达不得伪造清洁结果。

## 7. S07 首批共享生态

- [ ] 7.1 实现 Rust lint/rustdoc/build/依赖与安全义务；验收：workspace/features/targets 和所有 F01–F10 适用场景。

- [ ] 7.2 实现 Python（python）Ruff/注释/依赖与安全义务；验收：目标 Python/配置继承/锁缺失解析有真实样本。进行中：Ruff 已有项目 TOML 发现、逐文件原生扫描和公开 `lint python` 局部反馈命令；真实 E501/干净文件共存、排除文件不假通过、根/子目录不同配置各自生效，以及 CLI F401 回报通过；异源 JSON 诊断被过滤且本次标未完成。扫描后源码或配置变化也使受影响文件转为未完成，旧诊断只作待复查证据。命令因尚无完整策略固定返回3，见相邻 `codeguard-cli/tests/acceptance/{ruff-config-discovery,python-lint-scan}.md`。项目全量源集、规则/锁/政策及注释、依赖、安全适配仍缺，不勾选。

- [ ] 7.3 实现 Node/TypeScript/JavaScript（typescript）adapter 基础与依赖审计；验收：parser/tsconfig/本地插件和 monorepo 范围正确。

- [ ] 7.4 实现 Shell（shell）方言与 Dockerfile（dockerfile）/IaC 基础；验收：zsh 不静默丢弃、Hadolint 与配置安全报告不互相掩盖故障。

- [ ] 7.5 实现 CVE 共享生态映射、依赖归并和数据库 freshness；验收：UNKNOWN/离线/原生缺失/重复依赖均符合规格。

- [ ] 7.6 实现跨生态静态检查配置识别与原生结果归类；验收：注释/API 文档、依赖治理、SAST/秘密/IaC/容器等检查器在已配置时运行并反馈，未配置/无效/不可用分开解释，修复后按原工具复检。

## 8. S08 全语言补齐

- [ ] 8.2 实现 go 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.3 完成 go 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.4 为 csharp 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.5 实现 csharp 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.6 完成 csharp 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.7 为 kotlin 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.8 实现 kotlin 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.9 完成 kotlin 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.10 为 swift 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.11 实现 swift 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.12 完成 swift 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.13 为 php 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.14 实现 php 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.15 完成 php 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.17 实现 ruby 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.18 完成 ruby 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.19 为 scala 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.20 实现 scala 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.21 完成 scala 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.22 为 elixir 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.23 实现 elixir 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.24 完成 elixir 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.25 为 c 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.26 实现 c 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.27 完成 c 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.28 为 cpp 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.29 实现 cpp 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。 新增C++17局部原生/WASM对照：12例实际执行，包含2例语义边界；1测试通过、0忽略，未授予生产资格，见[验收记录](../../tests/acceptance/cpp17-native-wasm-differential.md)。 编译数据库静态解析基础已补充：有界参数数组、原序多配置、命令字符串不执行及畸形/超限反例；2测试通过，已接到项目发现，记录双读稳定摘要与unknown配置；发现测试先失败后2项通过。尚未接到原生执行，不升级构建生态状态。 公开init定向4项默认/WASM均通过，补充六类参数上下文阻塞，涵盖静态摘要持久化、command-only阻塞及配置变化/删除时保留任务；见[初始化验收](../../tests/acceptance/c-family-compilation-database-init.md)。 C++17类内普通方法及显式构造已有实际Clang结构观察，构造不要求返回说明；解析阶段已有2000声明/256参数预算，超限明确阻塞不截断。类/析构/模板等仍未解析，见[局部验收](../../tests/acceptance/cpp17-method-documentation.md)。 成员文档公开CLI稳定任务和原Clang复检默认/WASM实际通过，修复仍保留未批准关闭状态，见[工作台验收](../../tests/acceptance/cpp17-member-workbench.md)。；明确占位说明已有独立Clang AST适配器/封闭协议与2项测试（含真实原生），公开单文件comments已以0.10同次扫描反馈占位位置；稳定任务/原任务复检和共享check/hook尚未接入，见 tests/acceptance/clang-documentation-placeholders.md，父任务保持未完成。；占位独立包work sync导入已验收默认/WASM稳定文件任务、消费收据、漂移复用、清空不关闭与伪造拒绝；公开comments已自动持久化占位包并回传稳定任务（0.11）；next已有0.34收据绑定只读诊断及旧定位撤回；受控尝试/修复权限与专用verify仍未接入，返回具体未接入原因。；占位task verify身份preflight已验收原工具替换及跨检查器错参拒绝（无执行/事件），执行、结果消费和受控尝试仍未接入。

- [ ] 8.30 完成 cpp 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.31 为 objc 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.32 实现 objc 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.33 完成 objc 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.34 为 dart 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.35 实现 dart 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.36 完成 dart 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.37 为 vue 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.38 实现 vue 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.39 完成 vue 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.40 为 svelte 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.41 实现 svelte 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.42 完成 svelte 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.43 为 astro 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.44 实现 astro 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.45 完成 astro 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.46 为 css 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.47 实现 css 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.48 完成 css 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.49 为 html 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.50 实现 html 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.51 完成 html 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.52 为 graphql 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.53 实现 graphql 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.54 完成 graphql 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.55 为 solidity 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.56 实现 solidity 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.57 完成 solidity 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.58 为 terraform 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.59 实现 terraform 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.60 完成 terraform 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.61 为 nix 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.62 实现 nix 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.63 完成 nix 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.64 为 sql 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.65 实现 sql 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.66 完成 sql 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.67 为 protobuf 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.68 实现 protobuf 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.69 完成 protobuf 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.70 为 yaml 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.71 实现 yaml 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.72 完成 yaml 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.73 为 markdown 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.74 实现 markdown 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.75 完成 markdown 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.76 为 toml 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.77 实现 toml 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.78 完成 toml 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.79 为 haskell 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.80 实现 haskell 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.81 完成 haskell 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.82 为 ocaml 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.83 实现 ocaml 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.84 完成 ocaml 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.85 为 fsharp 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.86 实现 fsharp 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.87 完成 fsharp 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.88 为 perl 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.89 实现 perl 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.90 完成 perl 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.91 为 groovy 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.92 实现 groovy 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.93 完成 groovy 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.94 为 clojure 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.95 实现 clojure 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.96 完成 clojure 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.97 为 powershell 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.98 实现 powershell 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.99 完成 powershell 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.100 为 zig 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.101 实现 zig 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.102 完成 zig 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.103 为 nim 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.104 实现 nim 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.105 完成 nim 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.106 为 crystal 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.107 实现 crystal 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.108 完成 crystal 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.109 为 julia 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.110 实现 julia 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.111 完成 julia 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.112 为 elm 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.113 实现 elm 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.114 完成 elm 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.115 为 lua 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.116 实现 lua 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.117 完成 lua 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.118 为 luau 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.119 实现 luau 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.120 完成 luau 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.121 为 pascal 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.122 实现 pascal 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.123 完成 pascal 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.124 为 r 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.125 实现 r 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.126 完成 r 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.127 为 cfml 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.128 实现 cfml 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.129 完成 cfml 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.130 为 vbnet 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.131 实现 vbnet 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.132 完成 vbnet 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.133 为 erlang 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.134 实现 erlang 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.135 完成 erlang 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.136 为 liquid 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.137 实现 liquid 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.138 完成 liquid 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.139 为 cuda 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.140 实现 cuda 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.141 完成 cuda 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.142 为 ansible 固化六类别适用性、候选工具/方言/版本及缺口；验收：每个槽位有实际依据，not_applicable 不得用缺工具解释。

- [ ] 8.143 实现 ansible 的 lint/comments 适配与规则；验收：真实工具正确样本和违规样本、错误配置/版本/报告反例通过，格式化不能冒充注释检查。

- [ ] 8.144 完成 ansible 的 dependencies/CVE/security/build 适用能力及整体验收；验收：依赖生态映射和逐类别真实证据齐备，缺口未解决不升级 stable。

- [ ] 8.148 机器核对最新注册表与全部验收条目；验收：54 个原 stable 六槽位与真实证据完整，3 planned 如实披露，无丢项/重复/别名漂移。

## 9. S09 持久问题与修复工作流

- [ ] 9.1 实现 init dry-run/apply 与工作区 schema/.gitignore/受管路径；验收：不覆盖用户文件，不重复建立质量配置，未初始化只用私有用户缓存存原始报告。进行中：相邻 Rust CLI 已提供默认只读预览、精确受管目标预检查和局部 apply，含根 AGENTS 受管摘要；当前 apply 固定 partial/退出3，不虚构完整初始化。准备任务、刷新及私有原始报告缓存尚未接线；见 `codeguard-cli/tests/acceptance/init-workspace-preview.md`。

- [ ] 9.2 实现自有产物精确范围规则与工作区校验；验收：运行副本不递归扫描，codeguard/src 用户源码正常检查，入库 secret 仍阻断。进行中：相邻 Rust discover 已精确跳过自有文件及记录目录，保留 `codeguard/src`；真实 Git index 路径安全预览独立运行，但自有记录内容安全与正式 gate 仍缺。见 `codeguard-cli/tests/acceptance/init-workspace-preview.md`。

- [ ] 9.3 实现 finding/blocker 的稳定身份与重命名匹配；验收：十次相同扫描只有一个问题，行号变化不生成无意义重复，不确定匹配不误关闭。进行中：相邻 Rust CLI 的 Ruff 局部反馈 0.3 产生不依赖行号的 ID/指纹；真实双轮扫描加本地 sync 验证同一发现仅一份 finding/task。重复 finding 的每轮证据现写入忽略入库的 `state/observations/`，避免改动首次事实与无意义审计事件，崩溃重试幂等。局部 Ruff 环境 blocker 以 checker、构建根、原因和范围生成稳定 ID，重复扫描复用任务。Rust Clippy 局部 finding 现按原生规则、目标与源码行内容形成稳定身份；重复扫描只保留一张任务，缺 Cargo blocker 同样稳定。重命名关联、跨工具身份和不确定匹配协调仍缺，不勾选。

- [ ] 9.4 实现 run_id 游标、全量未消费报告幂等 sync；验收：先 lint 后 CVE 不丢问题，部分/过时报告不能跨范围关闭旧问题；事件写入后游标写入前崩溃可恢复且不重复，一份坏报告不阻止其它有效报告持久化，整体仍返回3。进行中：Rust `lint python` 在已初始化工作区按 workspace_id 保存脱敏本地报告并自动调用 `work sync`；后者遍历全部未消费 Ruff 报告，先落 finding/event/task 后落 `(workspace_id,run_id,摘要)` 消费标记，重复导入和标记丢失重试不重复任务；旧源码只计历史，坏报告不阻止有效报告，同键不同摘要拒绝。跨类别 RunReport、准备诊断、严格事务和并发协调仍缺，见 `codeguard-cli/tests/acceptance/work-sync-ruff-baseline.md`，不勾选。

- [ ] 9.5 实现 append-only 事件、父关系、状态机与可重建 Markdown 投影；验收：手改勾选无复检不关闭，缺父/分支冲突触发协调。进行中：Ruff 与 Rust Clippy 初见 finding 写固定 `observed` 事件和任务投影，重复报告不改 tracked 文件；缺失受支持任务的Markdown投影现由9.5.1恢复；其余事件、父关系校验、完整状态机及分支协调仍缺，不勾选。

- [ ] 9.6 实现任务依赖与 blocker 归并；验收：多个模块共缺 JDK 形成一个前置任务，各义务仍完整可见。进行中：Rust `work sync` 已将同一 Ruff 构建根、同一不完整原因的多个受影响文件归并成一个 `kind=blocker` 任务，分别保留原路径与观察事件；不同构建根不误合并，见 `codeguard-cli/tests/acceptance/work-sync-ruff-blockers.md`。JDK 跨模块公共前置任务、义务依赖边及跨检查器归并尚缺，不勾选。

- [ ] 9.7 实现 status/next/show 与 RepairBrief/版本化 recipe；验收：给出修复目标、范围、步骤和复检条件，诊断中的指令不可执行。进行中：Python Ruff 局部反馈给出有界修复提示；初版 sync 生成脱敏 Markdown 任务。Rust `next` 已从本地结构化 Ruff fact 生成只读简报，含范围、静态步骤、复检与关闭条件；任务 Markdown 指令不进入简报，缺工具/配置优先指向准备工作。已有本地status/show、Unix尝试/租约历史及多检查器原工具复检切片；完整版本化recipe、可信关闭、跨宿主与全部平台仍缺，因此不勾选，见 `codeguard-cli/tests/acceptance/next-local-brief-preview.md` 及后续分项验收。

- [ ] 9.8 实现 claim/heartbeat/release 跨进程租约；验收：同工作区只有一个有效领取者，过期和中断可恢复，不声称跨机器全局锁。进行中：相邻 Rust CLI 已有 Unix 本地跨进程锁、随机 token 摘要、generation、5 分钟期限及基本过期接管；并发四领取者只有一人成功，旧 token 和同名 owner 不能修改新租约；未结束 attempt 阻止 release，过期接管先落唯一 abandoned 事件。verify/fix 绑定、完整崩溃事务与 Windows 实现未完成，不勾选。

- [ ] 9.9 实现 attempt start/finish、动作与 patch 身份、无进展预算；验收：复检前失败/no-change/abandoned 均计数，耗尽后不重复推荐同一动作。进行中：Unix Rust CLI 已有受控 action-id、租约绑定、前后受影响路径摘要、不可覆盖 start/finish 事件、同结果幂等重放和同输入两次无进展预算；`ready-to-verify` 必须有绑定该 attempt-id 的原工具复检才可再次 start，复检仍显示问题/未完成时计入预算，私有报告丢失须重新复检。`next` 展示最近历史并停止推荐耗尽的动作。缺完整 patch/环境身份、跨平台实现、正式 fix 自动登记、可信策略和任务关闭，不勾选。 C/C++确认编辑文档候选已接通显式Clang及c11/c++17，缺上下文保持不完整、已有工作区重复编辑任务ID稳定，默认/WASM实际工具局部测试通过；项目配置发现、头文件及已安装宿主仍未完成，见[局部验收](../../tests/acceptance/c-family-documentation-edit-hook.md)。 Claude受控协议已投影C/C++原生文档/结构规则及同步任务指引，默认/WASM各15实际工具测试通过；过期/恶意/无问题边界通过，真实宿主安装仍未验收，见[对话验收](../../tests/acceptance/c-family-documentation-host-guidance.md)。 结构尝试input-v2新增编译解析器/校验器及复检实现摘要，不读取项目规则源码；正式跨版本旧历史验收仍待完成，见[身份检查点](../../tests/acceptance/clang-documentation-engine-identity.md)。

- [ ] 9.10 实现 task verify 的证据关闭与重开；验收：工具超时、加 ignore、移动到排除目录不自动 resolved，真修复有完整身份绑定；环境任务先确认恢复再重跑原受阻义务，自有验证租约收尾释放、调用方租约保留、错token不启动检查器。进行中：Rust `task verify` 已复用原生 Ruff 路径，对本地 finding/blocker 保存新报告、同步新问题并追加 `verification_observed`；问题消失只给待策略核验候选，fact 保持 open，`next` 对照报告复核候选；Unix 本地版本新增复检自有/借用租约、扫描前错误 token 拒绝、提交前再次核验及结束后自有租约释放，真实 Ruff 0.16.8 样本已覆盖借用场景。Rust Clippy finding/blocker 现复用租约、原生复检、同步与观察事件；真实抑制后原生 `--force-warn` 对照重新检出旧规则并要求抑制核查；对照无效或输入变化不生成修复候选。见 `codeguard-cli/tests/acceptance/task-verify-native-observation.md`、`task-verify-lease-binding.md` 与 `check-all-partial-native.md`。可信策略/覆盖、长操作自动续租、Windows 等价实现、正式关闭和重开仍缺，不勾选。

- [ ] 9.11 实现 code/dependency/environment/target/policy 的处置归因与例外标签；验收：policy_resolved 不计代码修复，例外保留未解决事实和期限。

- [ ] 9.12 提供受控 fix 的 attempt 与验证事件接口；验收：noop、部分修改失败及并发编辑的事件样本分别记录且不生成假修复；真实 formatter 接线在 S10 完成。

- [ ] 9.13 为插件提供启用后的 scan→sync→brief API；验收：新问题给下一步，重复无 Git 噪声，同步失败保留原 gate 并说明 backlog_update_failed。进行中：初始化项目的公开 Rust `lint python` 已自动保存、同步并在同一 CLI 对话反馈中返回局部 Ruff 简报；重复原生扫描不改 task。`check all` 的局部 Rust Clippy finding/blocker 也能保存、同步并在 JSON/human 对话反馈返回下一步。同步或简报失败仍显示原生结果并分别标明状态，见 `codeguard-cli/tests/acceptance/lint-python-auto-brief.md` 与 `check-all-partial-native.md`。插件 Hook/MCP 接线、正式 RepairBrief/门禁及其它检测族尚缺，不勾选。

- [ ] 9.14 验证持久记录隐私、篡改和删除边界；验收：日志默认不入 Git，删除所有任务不影响真实 gate，跨机器无原始日志可重新复检。进行中：Ruff 本地报告/消费标记默认 Git 忽略，tracked finding/task 不含原生消息，所有同步结果固定 `not_evaluated`；跨机器、删除记录和入库内容安全验收仍缺，不勾选。

- [ ] 9.15 实现项目边界与多构建根画像；验收：monorepo、多仓、worktree 和混合语言不会互相覆盖或越界观察。

- [ ] 9.16 定义 project.json/module-graph.json schema 与来源身份；验收：产品/目标/解析/本机版本分别记录，未知值不由其它字段猜测。进行中：相邻 Rust CLI 已产出画像 0.2.0 与模块图 0.1.0 局部 schema、manifest/锁/已识别规则配置摘要及 workspace 内容摘要；`package.json` 包版本另列，语言目标与未探测本机版本保持 unknown，解析版本与完整输入清单仍缺。

- [ ] 9.17 接入各已实现 adapter 的 manifest/锁/wrapper 静态观察；验收：init 不执行项目脚本，声明版本和已解析版本分别有依据。

- [ ] 9.18 实现分类型、带条件和完整性的模块图；验收：聚合不当依赖，动态未知边不支撑缩小检查范围，过期 CodeGraph 不作为当前证据。进行中：相邻 Rust CLI 仅生成有 manifest 依据的 contains 边并将依赖关系标 unresolved；构建依赖、源码引用、条件及 CodeGraph 身份仍缺。

- [ ] 9.19 实现多维架构画像与确认任务；验收：domain/controller 命名不自动认定 DDD，文档/源码冲突保留，推断不能激活阻断规则。

- [ ] 9.20 生成 architecture.md 与 AGENTS 精简受管区块；验收：事实/推断可追溯、详情链接有效，不泄露本机路径或凭据，文本指令不被升级为政策。进行中：相邻 Rust CLI 已生成 unknown 架构投影和仅含结构化画像身份/相对链接的根 AGENTS 区块；完整多维架构与来源归属仍缺。

- [ ] 9.21 实现 AGENTS 区块合并和文件身份保护；验收：人工内容、其它工具区块、子目录指令保留，人工修改/重复 marker/并发写入返回冲突。进行中：相邻 Rust CLI 已保留区块外原字节，并验证初始化后新加的人工前后文在刷新时仍保留；拒绝重复/缺失 marker、人工改写及规划后字节变化。外部编辑器不协作时最终比对与替换之间仍有竞态，子目录指令作用域尚未完整验收。

- [ ] 9.22 实现 init 默认 dry-run 与受控 apply 的可恢复事务；验收：无隐式安装/构建/服务启动/Hook接管，第二文件失败不伪称全成功或覆盖用户更改。进行中：相邻 Rust CLI 已预检受管目标、逐文件记录已创建文件/目录并对 AGENTS 再次核对；模拟画像先写而 workspace 标记未更新的中断后可重试完成且重复运行幂等。跨文件恢复日志及完整并发保护仍缺，故 apply 固定 partial/退出3。

- [ ] 9.23 实现输入清单与增删感知的幂等画像刷新；验收：新增模块、锁/规则变化均失效，刷新保留 findings/events/备注。

- [ ] 9.24 生成质量配置映射、测试/环境前置清单及准备任务；验收：缺工具或未知架构不虚构代码违规，不自动增加排除或存量豁免；dry-run 仅返回计划，apply 才持久化任务。

- [ ] 9.25 实现 init_status/readiness/next_actions；验收：仅按适用必需前置条件及有效证据汇总 ready/incomplete/unknown，可选工具不误阻塞；初始化成功不等于 check/gate 通过，给出可继续执行的 doctor/check/sync/next 动作。

- [ ] 9.26 固化 status历史freshness、next五类disposition及工作区缺失行为；验收：无任务不等于通过，过期结果不显示当前allow，待租用/待决策/待验证各有具体下一步。进行中：Rust `next` 局部查询区分未初始化、待同步、缺配置需决策、缺工具可准备、源码变化需重检和空任务需全量验证；尚无 status 历史新鲜度、租约 waiting、预算耗尽决策及完整五类覆盖，不勾选。

- [ ] 9.27 落实token/generation、action-id、attempt-id、幂等finish及释放恢复；验收：旧owner/token不能写新租约，重复finish不重复预算，未结束attempt正常release被拒绝，报告同键不同摘要拒绝；接管前abandoned/预算写入失败不授予新租约，恢复重放不重复计数。进行中：Unix CLI 已实现同一任务锁下的受控 action-id、随机 attempt-id、generation/token 核对、不可覆盖 start/finish、重复 finish 同结果返回原收据、release 拒绝 open attempt、过期接管先写 abandoned；跨进程崩溃矩阵、写入失败注入、verify/fix 共享租约和 Windows 等价实现仍缺，见相邻 `codeguard-cli/tests/acceptance/task-attempt-local-ledger.md`。

- [ ] 9.28 实现误报调查任务和 `whitelisted_false_positive` 事件/投影；验收：任务包含原生证据、最小复现、裁定、范围、复检与尝试历史；候选/失效/撤销后重新待处理，决策引用不能自行授权。

## 10. S10 修复能力

- [ ] 10.1 实现 dry-run 计划与隔离副本修复、变化清单；验收：只读请求不改源码，计划包含内容身份与副作用范围。

- [ ] 10.2 实现前置哈希核对和受控 patch 应用；验收：用户并发编辑不被覆盖，路径逃逸不执行。

- [ ] 10.3 实现同策略复检和修复归因；验收：noop/失败后部分修改/复检器副作用分别呈现，不虚报 fixed。

- [ ] 10.4 将依赖升级和规则调整分开；验收：修 CVE 不自动放松阈值，修 lint 不关闭规则或更改测试真值。

- [ ] 10.5 实现fix的task/owner/token接入及租约所有权；验收：复用已领取任务不重复attempt，批量租约冲突在应用前失败，dry-run不消耗预算，verify关闭原问题时仍保留新发现。

## 11. S11 宿主与分发接入

- [ ] 11.1 构建候选平台二进制，固化 ABI/MSRV/签名身份/校验清单；验收：各 target 运行 smoke，未测平台不标 stable。

- [ ] 11.2 实现插件 runtime lock 和原子下载/切换；验收：摘要不符、缺二进制、离线均无静默 Python fallback。

- [ ] 11.3 实现 MCP 相同核心 API 与版本化兼容工具；验收：F22、凭据脱敏与超时/取消正确。

- [ ] 11.4 更新五类宿主入口及 hooks/__protocol__.md 的旧新模式表；验收：保存反馈与严格交付区分，skipGate/env 不能降级新模式。

- [ ] 11.5 实现真实 Git pre-commit/pre-push 与 CI 入口；验收：alternate index、多 ref、非 HEAD、未知 Shell 边界正确，remote参数/stdin及ci input schema显式验证，不默认HEAD或由input自授策略权威。进行中：Rust CLI 暴露 `gate pre-commit` 的真实 index 路径安全预览，固定 3/incomplete，不是可放行的完整 Git Hook；见 `codeguard-cli/tests/acceptance/git-index-safety-preview.md`。

- [ ] 11.6 在独立 codeguard-skills 源仓更新调用与修复指引，再按 vendor 流程同步；验收：不直接编辑受管副本，不建议规避门禁。

- [ ] 11.7 明确 legacy 弃用与安全回滚路径；验收：旧数字保持，但旧通过不能被新 CI 认证。

- [ ] 11.8 完成macOS arm64候选制品和平台适配验收；依赖11.1/11.2；证据：真实启动、进程树/取消、路径/私有权限、原子写入、离线能力和代表性工具，未支持项明确gap。

- [ ] 11.9 完成macOS x86_64候选制品和同一平台验收清单；依赖11.1/11.2；验收：不能用arm64或仅交叉编译结果替代目标平台运行证据。

- [ ] 11.10 完成Linux x86_64候选制品、libc/最低系统约束和平台清单；依赖11.1/11.2；验收：固定实际目标环境，不把一种libc通过泛化为全部Linux。

- [ ] 11.11 完成Linux aarch64候选制品、libc/最低系统约束和平台清单；依赖11.1/11.2；验收：本平台真实工具/取消/隔离证明，不复用x86_64成功声明。

- [ ] 11.12 完成Windows x86_64候选制品与平台清单；依赖11.1/11.2；验收：Job Object、特殊路径、文件共享/权限和取消真实运行，不用Unix测试替代。

- [ ] 11.13 完成Codex宿主命令/MCP/保存反馈/Git阻断及恢复验收；依赖11.2–11.5与S09/S10；证据：实际绑定版本、原始请求及返回、scan→sync→brief链，不以安装缓存替代运行。

- [ ] 11.14 完成ZCode宿主同一接入清单；依赖11.2–11.5与S09/S10；验收：按其真实协议映射、不透传新CLI退出码，无法阻断的入口如实披露并验证真实Git/CI接入。

- [ ] 11.15 完成Kimi宿主同一接入清单；依赖11.2–11.5与S09/S10；验收：实际调用二进制、失败可恢复、任务链闭合，不以其他宿主通过代替。

- [ ] 11.16 将配置探测与原生检查结果反馈到 Codex/ZCode/Kimi 智能体对话；验收：已配置检查器、未配置项、有效诊断、工具故障、修复建议和复检命令清楚可见；backlog 同步失败仍显示本次结果，原始工具文本不能作为智能体指令。进行中：相邻 Rust `conversation_feedback` 已从结构校验后的 RunReport 生成 JSON/human 状态、规则 ID、可用的位置、未完成原因与复检 argv；`lint python` 把真实 Ruff 扫描配置、诊断、故障及 backlog 同步状态渲染成 CLI human/JSON，存储失败仍显示原生 finding，均排除原始工具文案；见 `codeguard-cli/tests/acceptance/{conversation-feedback-baseline,python-lint-scan,work-sync-ruff-baseline}.md`。可信来源绑定、完整规则解释与三宿主自动对话接线未完成，故不勾选。

- [ ] 11.17 实现事件驱动的检查档位与宿主接线；责任：core/CLI/plugin。验收：启动只发现、成功编辑局部快检、失败写入无源码检查、未知写入重定范围、修复按原任务复检、提交/推送/CI 各取本轮真实范围；软结果身份等价才可复用，无法阻断的宿主不宣称硬门禁。进行中：core 已有纯事件路由及[反例目标测试](../../tests/acceptance/hook-trigger-routing-candidate.md)；CLI 已有严格、有界的只读 `hook plan`，1.1 响应对逐文件快检限制 8 个不同路径、单路径 512 字节、路径总计 2 KiB，超预算明确重定范围而非截断或假称检查，保留 1.0 历史 schema，见[CLI 局部验收](../../tests/acceptance/hook-plan-cli-candidate.md)。Claude Code `UserPromptSubmit` 已经由 Rust 候选入口映射成固定、非阻断的检查时机建议，提示内容不改变扫描或门禁范围，见[局部验收](../../tests/acceptance/claude-post-tool-hook-candidate.md)。档位到完整真实检查器的命令映射、工具/配置/源码身份、时间/并发预算、三宿主默认 Hook 和真实 Git/CI 接线尚缺，不勾选。 C/C++确认编辑文档候选已接通显式Clang及c11/c++17，缺上下文保持不完整、已有工作区重复编辑任务ID稳定，默认/WASM实际工具局部测试通过；项目配置发现、头文件及已安装宿主仍未完成，见[局部验收](../../tests/acceptance/c-family-documentation-edit-hook.md)。 Claude受控协议已投影C/C++原生文档/结构规则及同步任务指引，默认/WASM各15实际工具测试通过；过期/恶意/无问题边界通过，真实宿主安装仍未验收，见[对话验收](../../tests/acceptance/c-family-documentation-host-guidance.md)。

## 12. S12 质量评测与验收

- [ ] 12.1 全量执行 F01–F26 适用矩阵与 schema/协议 golden 测试；验收：必备场景零假通过、零工具故障误判。

- [ ] 12.2 冻结分层语料、oracle和统计门槛，划分独立holdout；验收：样本/来源/裁定与批准阈值可追溯，不为提高结果改动holdout；计算器由12.10实现，实际评测由12.11执行。

- [ ] 12.3 完成真实旧新对照及人工差异裁定；验收：历史错误不作为新 oracle，兼容回归与意图纠偏分开。

- [ ] 12.4 测量冷/热启动、p50/p95、内存和并发；验收：覆盖与 findings 等价后再比较性能，不以跳检提速。

- [ ] 12.5 执行策略弱化、缓存污染、错误快照、报告缺项和修复越界测试；验收：本地边界如实记录，受保护 CI 不接受未批准策略；项目脚本无法改写可信政策、工具锁或签发收据。

- [ ] 12.6 Codex/ZCode/Kimi 各做真实命令、MCP、保存与 Git 阻断流程；验收：绑定预期二进制，失效场景不能假通过。

- [ ] 12.7 完整运行发现→同步→next→attempt→修复→verify→关闭/重开→全量 gate；验收：跨类别报告不丢失、失败可恢复、耗尽重试不逃逸、任务勾选或清空不改变真实门禁。

- [ ] 12.8 完整运行项目 init→AGENTS 读取→准备任务→检查→修改构建输入→画像刷新；验收：混合语言、版本冲突、架构未知与人工内容均处理正确，初始化不签发交付认证。

- [ ] 12.9 按 C01–C36 建立逐命令验收矩阵；验收：合法/非法参数、格式、操作退出码、副作用、未初始化、取消/重放/恢复及核心服务复用均有证据，MCP请求失败与服务生命周期分离。

- [ ] 12.10 实现分层评测计算与证据不足判定；依赖1.4、12.2的语料/阈值冻结产物；验收：TP/FP/FN、Wilson区间、零分母、争议样本和覆盖差异有已知期望值，不能以零finding报100%准确。进行中：相邻 Rust Core 已实现纯分层计算与相关正反例，见 `codeguard-cli/tests/acceptance/quality-evaluation-baseline.md`；尚未接入经批准冻结的 oracle/阈值与真实回放，不能勾选。

- [ ] 12.11 建立并执行确定性离线回归、独立holdout与真实工具/漏洞库评测的分离入口和报告归档；依赖12.2、12.10；验收：按语言/类别公布n/TP/FP/FN/区间及库身份/freshness/日期，网络波动不污染离线oracle，性能比较先证明发现/覆盖等价，不自动创建云定时任务。

- [ ] 12.12 完成误报白名单正反例与全格式门禁验收；验收：用户指出误判后能沿同一稳定 finding 完成原工具复核、候选、独立批准、再次检查与对话反馈；精确命中为 allow_with_exceptions、真实阻断为 deny、工具/覆盖未完成为 incomplete；过期/错文件/错内容/错版本/自批/原生 suppression 均不放行，原始 finding、审批引用及到期提示在 CLI/MCP/Hook/SARIF 一致。

## 13. S13 发布、文档与规格收敛

- [ ] 13.1 对照全部 requirement/scenario/tasks 建立最终证据索引；验收：缺证据的项目仍未完成，不为发布删除要求。

- [ ] 13.2 完成 Rust fmt/clippy/tests、工具链回归、插件既有适用测试、vendor 离线与在线检查、OpenSpec strict；记录真实结果。

- [ ] 13.3 形成 release notes、CLI/策略迁移指南和实际能力矩阵；验收：候选、stable、planned 与未验证平台表述一致。

- [ ] 13.4 发布并逐层核对源码/tag/制品/插件 lock/market/installed runtime；验收：版本及摘要闭环，不以本地成功替代安装证据。

- [ ] 13.5 完成受保护 CI 与已安装三宿主的最终运行复核；验收：实际检查链全部可追溯且未发生自动降级。

- [ ] 13.6 只有全部实施验收完成后 sync/verify/archive 本 change；保留本次设计验证与后续运行验证的独立记录。

- [ ] 13.7 对照实施覆盖索引验证规格、命令、语言、平台、宿主和任务的双向追踪；验收：没有无任务需求或无依据任务，新增设计同步索引，证据缺失仍保留未完成。

## 14. S14 WASM 语法初检完善

- [ ] 14.1 资产清单与来源。责任：adapters / 发布。规格：SP04。依赖：无。验收：固定 CodeGraph 来源提交、grammar 版本、许可证、SHA-256、ABI、语言/方言；缺来源、错散列和不兼容样本拒绝加载。进行中：已从固定 CodeGraph 提交复制 Java/Python/TypeScript/TSX WASM 与 MIT 许可证，清单和 Rust 校验拒绝来源/字节/声明 ABI 漂移；四份资产已由 Rust 离线加载并实测 ABI，Python 尚无 lint 路由；见[Python 资产局部验收](../../tests/acceptance/python-grammar-asset-candidate.md)；[局部证据](../../tests/acceptance/grammar-asset-candidates.md)。新增与批准资产清单分离的[完整来源覆盖库存](../../grammars/codegraph-coverage.json)及只读 `grammar status`：固定提交中的 30 份随仓 WASM 已逐字节核对，另两种依赖资产标为未固定，COBOL 的 16.4 MB 超出当前 8 MiB 加载限额；[覆盖验收](../../tests/acceptance/codegraph-grammar-coverage.md)。其余 grammar 的上游许可证/版本、Rust 加载，以及所有候选的语言版本/方言、语料及可发行验收仍缺，不勾选。 Python 资产已接入可选源码构建的 `lint python` 局部候选路径，但版本/方言和发行验收仍缺；见[Python 兜底局部验收](../../tests/acceptance/python-syntax-fallback-candidate.md)。 2026-09-29 增量：CodeGraph 来源更新至 `1072f82ce24db3d133258d30165cef6b74d108b2`，30/30 随仓字节与新提交一致。新增 Zig 第五份候选：保留新版原始 WASM、许可证和固定哈希；原始模块因 `__main_argc_argv` 不被 Rust `WasmStore` 接受，使用固定输入/输出散列的同签名导入适配后，可解析合法空容器及损坏样例，仍未做完整语料和原生对照；见[Zig 局部验收](../../tests/acceptance/zig-grammar-candidate.md)。2026-09-29 后续增量：Objective-C 与 Solidity 从 CodeGraph 锁定的 `tree-sitter-wasms@0.1.13` 获取，依赖包完整性、两份原始 WASM、上游许可证及适配后字节均固定；Rust 将旧 `dylink` 元数据转为 `dylink.0`，离线加载、基本正反例及隔离 worker 候选观察通过。候选累计 7，已验收发行仍 0；完整语料、原生对照和公开 lint 路由仍缺，见[依赖 grammar 局部验收](../../tests/acceptance/dependency-grammar-candidates.md)。后续增量：C、Go、JavaScript、Rust 的 CodeGraph 随仓 WASM 及上游版本标签许可证已固定，实测 Rust ABI、基础正反例和隔离 worker 候选状态；累计 11 份候选、21 种仍缺资产，见[主流语言局部验收](../../tests/acceptance/mainstream-grammar-candidates.md)。再后续增量：C++/C#/Lua/Luau 的 CodeGraph 随仓 WASM 和上游许可证固定；C++ 真实 ABI 14（测试先按 15 失败后纠正），C# 的加载符号固定为 `c_sharp`，四份均由 Rust 离线加载并经隔离 worker 验证。累计 15 份候选、17 种仍缺资产，见[局部验收](../../tests/acceptance/cpp-csharp-lua-luau-grammar-candidates.md)。本项及全量语言覆盖仍未完成。

- [ ] 14.3 进程资源隔离。责任：runtime。规格：SP05。依赖：14.2。验收：验证超时、内存、输出、取消与崩溃；损坏 grammar 不阻塞其他模块、不遗留进程。进行中：Unix 候选 worker 复用受控进程组内核，输入 1 MiB、输出 64 KiB，父进程有共同截止时间和取消；崩溃后代清理、输出洪泛、运行中取消及随后正常解析均增加真实子进程反例。Linux 2 GiB `RLIMIT_AS`、3 GiB 分配负例与 worker 故障隔离在 [Ubuntu CI](https://github.com/full-stack-plugins/codeguard/actions/runs/36478461118) 真实通过；Wasmtime 虚拟预留缩小。macOS 本机硬限制实测不支持，候选仍不具内存隔离权威；见[局部验收](../../tests/acceptance/syntax-worker-candidate.md)。其它目标平台、损坏 grammar 隔离及正式调度仍缺，不勾选。

- [ ] 14.4 语言覆盖与源码映射。责任：adapters。规格：SP06。依赖：14.2。验收：识别 ERROR/MISSING，合并恢复节点；测试 Unicode、换行、嵌入语言、新语法和未知版本，不将不支持当源码违规。进行中：Rust 已提取两类原始节点，adapters 核对 UTF-8/CRLF 字节锚点并转换 Unicode 标量列；结构祖先归组保留并列错误，预算截断保持不完整；[局部证据](../../tests/acceptance/syntax-recovery-candidate.md)。级联归并语料、版本/嵌入语言及原生对照仍缺。

- [ ] 14.5 原生工具准备探测。责任：adapters。规格：SP01,SP03。依赖：现有配置发现。验收：识别项目本地工具、wrapper、版本和无效配置；PATH 缺失不等于未安装，不对坏配置重复建议安装。进行中：ESLint 本地包/入口及 Maven Wrapper 脚本/发行配置已有只读候选分类，`detect` 与 `check` 已输出固定路径观察的候选；`node_modules` 普通源码遍历被排除，链接和损坏配置保留阻塞；发现协议升级为 0.4.0，聚合协议升级为 0.31.0 并保留旧 schema；[局部证据](../../tests/acceptance/native-tool-readiness-candidates.md)。单文件 TypeScript 本地候选已接通受控原生版本探针，见 [原生优先局部验收](../../tests/acceptance/native-first-eslint-candidate.md)；单文件 TypeScript 现进一步区分本地包确实未观察到与配置歧义、链接入口、包身份损坏等环境阻塞；阻塞不被误当成工具缺失而触发 WASM，见同一[原生优先局部验收](../../tests/acceptance/native-first-eslint-candidate.md)。显式工具自动选择、其它语言及多模块原生优先调度、更多构建器和真实多平台验收尚缺，不勾选。

- [ ] 14.6 逐模块原生优先调度。责任：CLI/core。规格：SP01。依赖：14.4,14.5。验收：原生可用先执行；原生违规不得被 WASM 洗白；多根分别选择并保留运行失败与兄弟结果。进行中：可选 CLI 特性在 `lint typescript`、`lint java` 显式单文件且完全缺少原生上下文时附加候选语法观察，部分原生上下文、符号链接、其它扩展名及 Javadoc/Checkstyle 选择保留原路径，见 [TypeScript 局部验收](../../tests/acceptance/typescript-syntax-fallback-candidate.md)、[Java 局部验收](../../tests/acceptance/java-syntax-fallback-candidate.md)。新增 `.tsx` 独立 grammar 的有效/无效 JSX 反例与候选报告方言绑定，见 [TSX 纠错验收](../../tests/acceptance/tsx-grammar-candidate.md)。本地 ESLint 10 候选与单一 flat config 可见且 PATH 提供普通 Node 时，单文件 TypeScript 路径现在自动调用既有原生探针并保留其局部发现或版本失败；Node 不可解析或本地入口/配置/包身份不可信时保持具体准备反馈，不启动 WASM；本地包确实未观察到时仍可初检，见 [本地原生优先局部验收](../../tests/acceptance/native-first-eslint-candidate.md)。项目原脚本参数、多模块调度、真实 ESLint 10 与混合结果保留仍缺，不勾选。 Python `lint` 现先保留 Ruff 原生结果；工具缺失或项目未声明 Ruff 时，对有界文件附加 WASM 疑似位置。无效配置与已有原生诊断不被 WASM 洗白；见[Python 兜底局部验收](../../tests/acceptance/python-syntax-fallback-candidate.md)。

- [ ] 14.7 初检协议与 schema。责任：core/CLI。规格：SP02,SP11。依赖：14.4。验收：固定版本和状态枚举、输入/资产身份、范围及缺口；空文件集、部分解析和未知版本不能 clean；未知 major 拒绝。进行中：core 已有状态聚合及[局部证据](../../tests/acceptance/syntax-precheck-status-candidate.md)；adapters 候选报告 1.0.0 的封闭 schema 与严格读者拒绝重复键、未知版本、身份漂移与伪造 clean，见[候选报告证据](../../tests/acceptance/syntax-precheck-report-candidate.md)；可选 CLI `lint typescript` 与 `lint java` 单文件入口分别产生 TypeScript 0.3.0、TSX 0.4.0、Java 0.1.0 局部候选反馈及位置化疑似观察，见 [TypeScript](../../tests/acceptance/typescript-syntax-fallback-candidate.md) 与 [Java](../../tests/acceptance/java-syntax-fallback-candidate.md) 验收。TypeScript/TSX 疑似位置现可进入已初始化工作台的版本化任务报告；正式全范围/嵌入区域/排除原因、Java 与多模块任务引用、所有消费者未知 major 拒绝与发布兼容仍缺，不勾选。 新增 Python 0.14.0 局部反馈 schema，明确原生状态、候选范围、源码和 grammar SHA-256、缺口与疑似位置；零观察仍 incomplete，见[Python 兜底局部验收](../../tests/acceptance/python-syntax-fallback-candidate.md)。 Python 已初始化单文件反馈升级至 0.15.0，独立 0.1.0 脱敏确认报告绑定源码、固定 grammar 和真实任务引用；同步失败保留空 ID 与原因，见[局部验收](../../tests/acceptance/python-syntax-fallback-candidate.md)。

- [ ] 14.8 安装建议与必须确认策略。责任：core。规格：SP03。依赖：14.6,14.7。验收：可选原生缺失且完整 clean 仅推荐；疑似或已有必需义务要求恢复原生确认；无适配器给具体决策。进行中：core 已有纯决策函数，区分缺工具、坏配置、执行失败和无适配器；仅非空、完整、已验收且无恢复节点的 clean 才允许可选建议，伪造 clean 状态不能降级。见[局部验收](../../tests/acceptance/syntax-setup-guidance-candidate.md)。真实工具准备来源、CLI/工作台任务投影及宿主反馈尚未接线，不勾选。

- [ ] 14.9 四类可读与 JSON 报告。责任：CLI。规格：SP07,SP11。依赖：14.7,14.8。验收：覆盖原生成功、缺原生且 clean、疑似语法、初检不可用；结果范围/行动/交付语义一致；不伪造任务 ID。进行中：启用可选 WASM 特性的 TypeScript 与 Java 单文件在无显式原生上下文时提供 JSON/human 候选疑似观察；无恢复节点仍为 incomplete。未初始化时任务引用为空；TypeScript/TSX 在已初始化工作区同步成功后用 0.5.0 报告返回真实稳定任务 ID，见[工作台局部验收](../../tests/acceptance/typescript-syntax-confirmation-task.md)。Java 超大输入保留 worker 未完成，见 [TypeScript](../../tests/acceptance/typescript-syntax-fallback-candidate.md) 与 [Java](../../tests/acceptance/java-syntax-fallback-candidate.md) 局部验收。正式四类报告、多模块范围与宿主渲染仍缺，不勾选。 Python 0.14.0 JSON/human 局部反馈现区分 Ruff 缺失、配置未声明、WASM 疑似及不可用范围，始终返回 incomplete、退出码 3 且任务 ID 不伪造；见[局部验收](../../tests/acceptance/python-syntax-fallback-candidate.md)。 Python 已初始化单文件结果只在实际同步成功后返回稳定 `setup.task_id`；0.15.0 区分持久化成功/失败，未初始化仍是 0.14.0 和空 ID，见[局部验收](../../tests/acceptance/python-syntax-fallback-candidate.md)。

- [ ] 14.10 稳定任务与下一步。责任：core/CLI。规格：SP08。依赖：14.8,14.9及现有工作台。验收：重复扫描更新同一准备/确认任务，失败尝试有记录，无进展给具体诊断；推荐项不持续占用 next。进行中：单文件 TypeScript 已能把“本地 ESLint 可见但当前 PATH 无 Node”记为 `eslint_node_runtime_unresolved`，初始化工作区连续两轮仅更新同一环境任务，`next` 要求受控 Node 原生复检而非重复安装 ESLint 或修改源码；见[原生优先局部验收](../../tests/acceptance/native-first-eslint-candidate.md)。配置歧义、链接入口和损坏包清单连续出现时亦更新同一 ESLint 环境任务、变更具体诊断而不生成源码 finding；见同一[局部验收](../../tests/acceptance/native-first-eslint-candidate.md)。TypeScript/TSX 候选 WASM 初检现在将已初始化工作区的源码范围接到一张稳定的 ESLint 原生确认阻塞任务；重复扫描和后续零恢复节点不关闭，见[工作台局部验收](../../tests/acceptance/typescript-syntax-confirmation-task.md)。TypeScript/TSX 有界疑似位置已用源码/grammar 身份保存到 0.2.0 准备报告并由任务报告摘要引用，导入拒绝伪造 grammar 和越界坐标；失败尝试/无进展预算、推荐项非阻断投影、Java 与多模块任务尚缺，不勾选。 Python 单文件候选观察已接入既有 work sync，工作区/源码范围稳定归并；反复扫描及后续 WASM 零恢复节点不关闭任务。导入拒绝伪造 grammar 与越界位置；多文件任务和能力匹配关闭仍缺，见[局部验收](../../tests/acceptance/python-syntax-fallback-candidate.md)。

- [ ] 14.11 能力匹配复检。责任：core/adapters。规格：SP08。依赖：14.10。验收：同模块/输入/方言的原生语法能力确认才能关闭；仅安装、后续 WASM clean 或格式检查不能关闭；复发重开。

- [ ] 14.12 精确误报纠错。责任：core。规格：SP09。依赖：14.11及既有白名单提案。验收：绑定 grammar/规则/范围/版本和可信审批，变更失效；原生反证保留历史并进语料，不撤销原生义务。

- [ ] 14.13 缓存与失效恢复。责任：runtime/core。规格：SP10。依赖：14.7。验收：输入、grammar、runtime、规则和配置变化失效；不缓存不完整为 clean；升级回滚不丢历史。

- [ ] 14.14 Claude Code 实际对话接入。责任：plugin。规格：SP07,SP12。依赖：14.9,14.10。验收：用真实宿主触发检查并显示有界脱敏摘要、任务下一步；去重及持久化失败可见；拒绝诊断中的指令注入。

- [ ] 14.15 Codex 实际对话接入。责任：plugin。规格：SP07,SP12。依赖：14.9,14.10。验收：独立验证真实 Codex 对话可见反馈、去重、故障和权限边界；CLI JSON 不能替代宿主验收。

- [ ] 14.16 Gemini CLI 实际对话接入。责任：plugin。规格：SP07,SP12。依赖：14.9,14.10。验收：独立验证真实 Gemini 对话反馈、去重、故障和权限边界，不借用其他宿主结果。

- [ ] 14.17 逐语言误报与性能评测。责任：评测/adapters。规格：SP06,SP12。依赖：14.3,14.4,14.11。验收：独立合法/非法 oracle、版本/方言语料、原生对照；分别报告误报、漏报、未知覆盖及冷暖启动/包体/内存；先实测再定预算。

- [ ] 14.18 打包兼容与分阶段发布验收。责任：发布/CLI。规格：SP04,SP11,SP12。依赖：14.12-14.17。验收：每个平台验证内置资产/许可证/散列和离线 npx；保持退出码、统一入口及旧报告读取；同步中英文示例；未验收语言/宿主仍标 gap。

- [ ] 14.19 CodeGraph 32 种 grammar 全量接入与 Zig 误报回归。责任：adapters/runtime/CLI/发布。规格：SP01,SP04,SP06,SP11,SP12。依赖：14.1-14.18。验收：逐语言固定可再分发来源、许可证、原始和发行字节、真实 ABI、Rust worker 离线加载、合法/非法版本语料、原生工具对照、原生优先统一命令、任务与对话反馈、资源预算及发行包实装；覆盖 arkts/c/cfml/cfquery/cfscript/cobol/cpp/csharp/dart/erlang/go/java/javascript/kotlin/lua/luau/nix/objc/pascal/php/python/r/ruby/rust/scala/solidity/swift/terraform/tsx/typescript/vbnet/zig。此前十五份候选（C/C++/C#/Go/Java/JavaScript/Lua/Luau/Objective-C/Python/Rust/Solidity/TypeScript/TSX/Zig），已验收发行 0；COBOL 超出 8 MiB 上限，其他 17 种仍无资产清单和执行路由。C++/C#/Lua/Luau 的固定来源、许可证、真实 ABI、离线加载及基础正反例见[局部验收](../../tests/acceptance/cpp-csharp-lua-luau-grammar-candidates.md)。C/Go/JavaScript/Rust 的固定来源、许可证、离线加载及基础正反例见[局部验收](../../tests/acceptance/mainstream-grammar-candidates.md)。不得以 `grammar status` 库存、WASM 可加载或未验收候选替代原生 lint 或语言验收。 2026-10-07当前源码全量原始回放：358例/32语言/35来源全部实际执行，252.79秒；73TP/1FP/10FN/269TN/3unknown/2pending，仍fails_fixture_threshold及0/32资格，不能以测试通过替代质量通过；见[本轮证据](../../tests/acceptance/grammar-current-full-replay-2026-10-07.md)。

## 15. S15 四类核心能力逐语言生产验收（2026-10-06 用户明确要求）

- [ ] 15.1 建立57语言×四核心能力×版本/方言×构建器×声明平台的验收映射；关联既有8.x逐语言任务、实际适配器和证据，列明缺口；不得用空目录或泛化覆盖替代。

- [ ] 15.2 验收逐语言WASM语法和原生lint/编译器的联合路径：配置发现、原生优先、缺工具初检、版本/方言兼容、精确定位、取消/超时及安装指引；32个候选与其它语言缺口均须独立解决。

- [ ] 15.3 验收逐语言详细文档注释：按语言规范检查用途、参数、返回、错误及行为契约等适用内容；Java覆盖类型/方法/字段Javadoc及Maven/Gradle项目配置；裸标签或空注释不能冒充合规。

- [ ] 15.4 验收逐语言开发规范原生检查：Java P3C及各生态适用规范工具，覆盖实际配置/规则组、原生诊断、合法反例与误报纠正；formatter-only不能代替规范lint。

- [ ] 15.5 验收逐语言实际依赖生态漏洞检查：Java必须分别完成Maven和Gradle插件路径；其余语言按实际构建/包生态接入，覆盖锁/解析图、多模块、漏洞源身份及时效、缺工具/离线故障及修复复检。

- [ ] 15.6 验收四类能力的真实闭环：检查→对话反馈→稳定任务→修复→原工具复检→关闭→复发重开；缺配置/环境恢复与源码违规分开，不允许降低规则或白名单自批逃逸。

- [ ] 15.7 执行逐语言独立标注语料、原生真实运行、声明平台/宿主、故障恢复和性能验收，产出四能力生产资格矩阵；所有未通过单元阻断全语言生产声明，不以开发回归或任务勾选替代。

