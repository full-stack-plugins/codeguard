# Codeguard 原生适配器契约与生态边界

> **文档说明：**负责生态语义、规则归属与局部调用限制；六类别和 28 检测族定义统一归技术方案。
>
> **文档版本：**1.2.0 · **最后更新：**2026-09-29 · **状态：**当前切片与目标契约分别标注。

[English](Codeguard-Adapter-Contracts.md) · [文档导航](README.zh_CN.md) · [架构](Codeguard-Architecture.zh_CN.md) · [技术方案](Codeguard-Technical-Design.zh_CN.md)

## 1. 能力、配置和本轮运行分开

[技术方案第 6 节](Codeguard-Technical-Design.zh_CN.md#6-静态检测能力目录)是**候选检查目录**，不是每个项目必须执行的清单，也不是已实现能力表。`codeguard capabilities` 说明发行版能适配什么；项目配置探测说明本项目启用了什么；本次检查结果说明原生工具返回了什么。这三者分别反馈。当前 57 种语言在五个候选平台上的六类别单元仍为 `gap`。Rust 负责配置发现、受控调用和结果整理，P3C、Maven、Javadoc 等继续运行各自原生检查器。

```mermaid
flowchart LR
    A[项目语言与构建器] --> B[探测原生检查配置]
    B -->|已配置| C[Rust 调用原生检查器]
    B -->|未配置/无效| D[说明配置缺口与启用建议]
    C --> E[解析原生诊断或工具故障]
    D --> F[智能体对话反馈]
    E --> F
    F --> G[智能体修复代码或配置]
    G --> H[原工具复检并更新反馈]
```

检测族完整标识是“类别.族名”，如 `comments.api_docs`。它用于给已配置的检查器归类，而不要求所有项目逐族启用。`dependencies.graph` 可为 CVE 匹配提供输入，但图解析成功不能宣称没有漏洞；CVE 无命中也不能证明许可证或版本治理合格。Javadoc 检查应按项目已配置的规则解释公共 API、参数/返回、继承文档等诊断，formatter 不算注释检查。

项目配置探测返回 `configured / missing / invalid / unknown`，并指出检查器、配置文件/规则来源和具体处理建议。`configured` 时运行原生命令，把本次诊断、位置或组件、规则依据、工具错误及复检命令直接反馈到智能体对话。`missing` 不等于源码违规，`unknown` 不等于通过；只有批准策略明确要求的缺项影响交付门禁。工具故障、坏报告或数据库过期反馈为未完成，不冒充代码问题。修复后重跑相同原生检查器，依据新结果更新任务。

## 2. Java 原生能力边界

Java 是第一条端到端实现线，不代表只支持 Java。适配计划必须区分以下义务：

| 类别 | 工具/机制候选 | 验证要点 |
|---|---|---|
| 阿里规范 | 官方 P3C PMD 规则实现 | P3C/PMD/JDK 的兼容组合和实际规则加载 |
| 格式与通用规范 | Checkstyle；项目已有格式检查目标 | XML 结构、规则版本、源码级别、只读模式 |
| 注释规范 | Checkstyle Javadoc 规则、Javadoc/doclint | 公共 API 范围、参数返回、继承文档、Lombok/record/生成代码 |
| 安全静态分析 | 项目已接入并经验证的安全分析器，例如 SpotBugs 安全规则或规则扫描器 | 所需字节码、classpath、规则清单与报告完整性 |
| CVE | 经验证的 Maven 依赖扫描适配器 | 解析实际依赖图与版本，漏洞库身份和匹配证据 |
| 构建 | Maven/Gradle 的批准生命周期和目标 | 编译及必要质量任务是否真的执行；测试执行单独声明 |

官方 P3C 包含 PMD 实现；旧插件中的 Checkstyle XML 不应被重命名或包装成“官方 P3C 已通过”。具体兼容版本在真实样本矩阵中确定。[P3C 官方仓库](https://github.com/alibaba/p3c)

Java 解析顺序：项目 wrapper 与声明运行时 → effective model/启用 profile → 模块图/源集/依赖 → 已绑定质量任务 → 缺失义务的受控适配计划。读取 effective model 可能执行构建扩展，因此不是纯静态 `plan` 的隐式行为，需进入执行阶段并留证据。

当前 Maven Javadoc 多文件局部探针只对无继承、依赖、profile、模块、扩展、其它插件和动态配置的简单 POM 重放原文件，并把原 POM 与显式源码一起复制到私有快照。其它 POM 返回未完成，不能用固定模板 POM 产生项目归属的注释问题。此局部执行仍未证明完整生效模型、生成源码和源集覆盖，不签发项目门禁结果。

对于未配置质量工具的项目，返回 `missing` 与具体配置建议；不自动构造或写入 POM/Gradle 质量插件。配置无效返回 `invalid`，无法判定返回 `unknown`。用户或批准策略明确选择启用后再按原生命令运行，不以“缺配置”报告源码违规。`java.commands` 只能提供实际命令，不能以 `echo ok` 满足 P3C。

Maven 父子模块、dependencyManagement、profile、toolchains、annotation processor、测试源集和 Gradle 动态脚本都影响结论。静态模型不完整时保守扩大范围；扩大后仍不能证明规则执行，则未完成。已有 Java 影响闭包可迁移，但版本变动不得自动以 validate/help 替代未证明等价的 lint/CVE 义务。

## 3. 依赖、CVE、注释与安全的专属语义

CVE 的单位是依赖生态和解析后的组件图，不是源码后缀。Maven/Node/Python/Rust/Go 与通用扫描器按清单能力选择；原生缺失不得偷偷 fallback 成不同覆盖的通过。多个语言共享同一依赖图可以共享证据，但每份义务必须可追溯。

漏洞记录保留 CVE/GHSA/OSV 等原标识、别名、包坐标或 purl、版本、传递依赖路径、匹配方式、受影响区间、修复版本、严重度来源和漏洞库时间/摘要。只有模糊 CPE 匹配时，不伪装成精确包证据；待复核项仍可导致完整性不足，不静默丢弃。

缺 lockfile 不必一律失败：适配器若能解析并固化实际依赖图可继续，否则未完成。漏洞库过期、网络断开或数据库未验证不能产生安全 PASS；离线只使用满足批准 freshness 的本地库。未知严重度不能映射为 LOW 或删除：若策略无法比较则未完成；批准“所有已知漏洞阻断”时可直接判违规。

当前 `check java` 按每份 Java 源码的最近构建根展示 Maven Dependency、OWASP Dependency-Check 与 FindSecBugs 的静态声明状态。静态简单 POM 中已配置的 Maven Dependency Plugin 现可用私有原 POM 副本离线运行原生依赖树，反馈组件和传递边；原生日志即使 `BUILD SUCCESS`，只要有缺 POM 等警告便标未完成。OWASP 等其它局部路径的当前状态见下表；并非所有 checker 均已接入；缺失、跳过、未知或多模块混合配置分别给出原因与下一步。局部依赖图不代表全模块、版本治理、许可证、SBOM、CVE 数据库或安全检查覆盖。

注释规则绑定语言语法及 API 范围。规则包应明确是否允许继承文档、接口/实现重复说明、生成代码排除和 test fixture 的专用规则；这些是预先确认的语义，不是扫描失败后的临时豁免。

安全分为源代码规则、配置/IaC、敏感内容及入库路径策略。`.env`/`*.pem` 等命中现行禁止入库模式时报告 `repository_policy_violation`，不得仅凭文件名宣称发现私钥；内容扫描的 secret finding 另有规则和脱敏证据。公共证书等例外需要正式策略变更，AI 不可自动放行。


## 4. 当前已接入路径与证据

下表取代旧稿逐日“解析器尚未执行”的累积日志；均是有限观察，不承诺全项目认证。

| 路径 | 当前能力 / 约束 | 验收入口 |
|---|---|---|
| P3C | 按项目已声明规则集选择固定原生探针；规则集与报告归属不符为未完成 | [P3C](../tests/acceptance/java-p3c-cli-native-local.md) |
| Javadoc | lint java 的 javadoc checker；显式 Maven 上下文用原 POM 多文件探针，不回退隔离单文件诊断 | [项目模式](../tests/acceptance/javadoc-project-mode-selection.md) |
| Checkstyle | 固定 10.21.4 的原配置；JavadocVariable、MissingJavadocMethod/JavadocMethod、MissingJavadocType/JavadocType 的已验收参数/token | [字段](../tests/acceptance/checkstyle-field-javadoc.md)、[方法](../tests/acceptance/checkstyle-method-tag-options.md)、[类型](../tests/acceptance/checkstyle-type-options.md)、[格式](../tests/acceptance/checkstyle-type-formats.md) |
| Ruff | 配置感知扫描及 show-settings/ignore-noqa 对照；D 规则是原生文档诊断 | [规则观察](../tests/acceptance/ruff-effective-settings-observation.md)、[抑制](../tests/acceptance/ruff-native-suppression-observation.md) |
| Cargo | Clippy、库目标 Rustdoc、all-targets/default-features 的 Cargo check；check all 独立调度并同步 | [文档调度](../tests/acceptance/rustdoc-check-all.md)、[构建复检](../tests/acceptance/rust-build-task-verification.md) |
| cargo-audit | 本轮 Cargo.lock 包名/版本/来源/校验和对照；RustSec 来源和时效未认证，未检查 yanked | [原生观察](../tests/acceptance/cargo-audit-native-observation.md) |
| ESLint | 显式 Node/entry/version/config/cwd；目录逐文件及 config-map；原生 JSON/settings 与稳定任务 | [公开调用](../tests/acceptance/eslint-public-lint-feedback.md)、[多根配置](../tests/acceptance/eslint-config-map.md) |
| npm audit | cve typescript/check all/task verify、父工作区与缺输入准备任务；异常退出保留有效局部报告 | [部分结果](../tests/acceptance/npm-partial-native.md)、[公开反馈](../tests/acceptance/npm-public-cve-feedback.md) |
| pip-audit | cve python/check all 的标准 pylock 快照及本轮组件版本集合匹配；PEP 751 环境/组/哈希和数据库覆盖未完成 | [局部报告](../tests/acceptance/python-cve-partial-native.md) |
| Go vet | 逐模块原生 vet、go list 文件选择与工作台；build tags/CGO/平台完整组合待验收 | [Go](../tests/acceptance/go-vet-json-local-probe.md) |

ESLint config-map 的 root/config/cwd 均相对请求目录，按最深目录组件选择；重复、越界、未知字段、内容变化拒绝，不由本地映射授予政策批准。示意配置：

```json
{"schema_version":"1.0","projects":[{"root":"packages/api","config":"packages/api/eslint.config.cjs","cwd":"packages/api"}]}
```

Checkstyle 仅映射已知官方短名、Check 后缀与全限定名，不猜自定义模块；字段必需 token、紧凑构造器、可见性、注解及标签配置遵循固定原工具的真实行为，Rust 不额外补报。无效作者/版本正则应给配置恢复指引，不编造作者或版本填文档。静态配置存在、语义已支持、原生已实测分别记录。

Python 依赖发现区分 requirements 简单固定版本、uv/Poetry 专用锁、标准 pylock.toml/pylock.<name>.toml；多锁保持歧义。不能把 uv/Poetry 专用锁直接当作 pip-audit 标准锁输入，也不因存在锁就声称没有漏洞。

## 5. 调用与扩展契约

适配器声明 ID/版本、语言/方言、输入单位、配置、平台/工具版本、退出/报告契约、缓存条件、修复与网络资源需求。观察与执行分开；adapter 解释注入数据并提出字面命令，不私自 spawn/联网/修改文件。CLI 组合 runtime。JSON/XML 优先，文本工具须固定版本/locale/golden 样本；未知协议保留未完成。共享 ESLint 不等于 Vue/Svelte/Astro/GraphQL parser 已验收。

逻辑阶段为 discover → resolve → plan → parse → verify_coverage → plan_fix；这是职责模型，不声称已存在统一动态 trait。原生输入快照、缓存、隔离与回滚规则统一见技术方案。新增生态先定义正反例和真实工具契约，再接反馈、持久任务和复检。

## 6. 报告与 WASM 扩展

所有生态遵守 [统一对话报告](Codeguard-Technical-Design.zh_CN.md#73-对话报告示例目标呈现)：结论、方式/范围、原生状态、依据、下一步与覆盖边界。Javadoc 示例不能仅写“发现两项”，必须说明已配置/实际执行及缺类路径等阻塞；CVE 零项不得省略数据库覆盖。

S14 的 WASM 是缺少可用原生检查器时的语法初检，不能替代上述注释、类型、依赖或安全契约。原生违规不触发兜底洗白；疑似语法由适用原生能力确认。资产加载兼容与语言语法质量须分别验收。
