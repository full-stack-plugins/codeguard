# Kotlin/Swift/Ruby 六类别适用性固化与规则配置发现

日期：2026-10-07。对应 OpenSpec 8.7（Kotlin 适用性）、8.10（Swift 适用性）、8.16（Ruby 已发布档案的 CLI 呈现）、15.2–15.5（六类别槽位路径）。本记录是**语言适用性研究档案 + 只读规则配置发现**的验收，不是任何原生工具的执行、报告契约或平台资格验收。

## 交付内容

1. **适用性固化（Rust 研究档案）**：`crates/codeguard-cli/src/check_kotlin_scan.rs::applicability_profile` 与 `check_swift_scan.rs::applicability_profile` 以纯 Rust 登记六类别（lint/comments/dependencies/cve/security/build）权威候选工具、方言、版本策略与缺口；`check_ruby_scan.rs::applicability_profile` 不复制数据，直接经 `codeguard_adapters::bundled_ruby_candidate_profile()` 呈现 8.16 已发布的 `rulepacks/ruby_static_candidate_v1.json`，避免双源漂移。
2. **独立交付路径（规则配置发现）**：三文件各自的 `observe_rule_config(root, deadline, cancelled)` 只读发现项目规则配置（Kotlin=detekt `config/detekt/detekt.yml`+`detekt.yml`；Swift=SwiftLint `.swiftlint.yml`+`.swiftlint.yaml`；Ruby=RuboCop `.rubocop.yml`+`.rubocop.yaml`），状态机为 **configured / missing / invalid / unknown**；`rule_config_recheck` 复核输入变化。

## 六类别槽位与依据

所有槽位 `status="gap"`，版本策略统一 `project_locked / unvalidated_project_lock_required`（不浮动选 latest）。**缺工具一律是准备缺口，任何槽位都不以 not_applicable 解释**；build 槽位为 `project_dependent`。

### Kotlin（8.7）

| 类别 | 候选 | 一手依据与限制 |
|---|---|---|
| lint | detekt（`./gradlew detekt`） | [detekt 文档](https://detekt.dev/docs/intro)；kotlinc 单文件编译只观察语法，不是项目风格 lint |
| comments | detekt comments 规则集 | [Comments Rule Set](https://detekt.dev/docs/rules/comments/)（UndocumentedPublicClass/Function）；存在性规则，不证明参数/返回/异常契约质量，KDoc 交叉引用未验收 |
| dependencies | Gradle `dependencies` / Maven `dependency:tree` | [Gradle 依赖查看](https://docs.gradle.org/current/userguide/viewing_debugging_dependencies.html)、[maven-dependency-plugin](https://maven.apache.org/plugins/maven-dependency-plugin/)；锁绑定、许可证、SBOM、来源未验收 |
| cve | OWASP dependency-check（gradle/maven 插件） | [gradle 插件](https://jeremylong.github.io/DependencyCheck/dependency-check-gradle/)、[maven 配置](https://jeremylong.github.io/DependencyCheck/dependency-check-maven/configuration.html)；NVD 数据库快照身份与时效必须绑定，自动下载必须前置拒绝 |
| security | semgrep（Kotlin general analysis） | [Semgrep 支持语言](https://docs.semgrep.dev/docs/supported-languages/)；detekt 无 security 规则集，官方 Kotlin SAST 缺位是登记事实，第三方规则集来源未绑定 |
| build | `./gradlew build` / `mvn package` | [Gradle CLI](https://docs.gradle.org/current/userguide/command_line_interface.html)、[Maven 生命周期](https://maven.apache.org/guides/introduction/introduction-to-the-lifecycle.html)；适用性依项目构建目标，wrapper 校验未做 |

### Swift（8.10）

| 类别 | 候选 | 一手依据与限制 |
|---|---|---|
| lint | SwiftLint（`swiftlint lint --reporter json`） | [SwiftLint README](https://github.com/realm/SwiftLint/blob/main/README.md)；swift-frontend 单文件 parse 只观察语法；嵌套/父目录配置解析需工具上下文 |
| comments | DocC（`swift package generate-documentation`） | [DocC](https://www.swift.org/documentation/docc/)；DocC 诊断坏注释（重复/未知参数），不证明存在性或参数/返回/Throws 完整性；docc-plugin 存在性未核验 |
| dependencies | SPM `swift package resolve` + Package.resolved | [Swift Package Manager](https://www.swift.org/documentation/package-manager/)；resolve 执行 SPM 需无项目脚本副作用政策；v1/v2 锁绑定未验收 |
| cve | osv-scanner（`--lockfile Package.resolved`） | [osv-scanner 支持的锁文件](https://google.github.io/osv-scanner/supported_lockfiles/)；无官方 SPM advisory 扫描器是登记事实；OSV 数据库时效必须绑定，联网须前置拒绝 |
| security | semgrep（Swift 实验性支持） | [Semgrep 支持语言](https://docs.semgrep.dev/docs/supported-languages/)；实验性/可能回退 generic 模式，SwiftLint 无 security 规则集 |
| build | `swift build` / `xcodebuild -scheme <selected> build` | [SPM](https://www.swift.org/documentation/package-manager/)、[xcodebuild 命令行构建](https://developer.apple.com/documentation/xcode/building-from-the-command-line-with-xcodebuild)；scheme/destination/签名选择需项目模型 |

### Ruby（8.16，CLI 呈现）

六槽 RuboCop（lint/comments）/Bundler（dependencies）/bundler-audit（cve，需数据库时效）/RuboCop Security+Brakeman（security）/gem build（build，project_dependent）维持原档案，见 [ruby-candidate-baseline.md](ruby-candidate-baseline.md)；本次仅新增 RuboCop 规则配置发现，不改档案内容。

## 规则配置发现状态机

- **configured**：唯一候选存在、常规文件、UTF-8、≤1MiB、无制表符缩进、无 C0 控制字符、且含行首 `Key:` 映射键（保守 YAML 子集，兼容 `Style/Documentation:` 部门键与 CRLF 行尾）。`semantics` 恒为 `unresolved`、`tool_execution` 恒为 `not_attempted`——**configured 不证明规则语义、生效继承或工具可用**。
- **missing**：候选全缺。不伪造 selected/sha256/size。
- **invalid**（可证伪）：非 UTF-8、空白、控制字符、制表符缩进（YAML 禁止）、无任何映射键（纯散文）、非常规文件。
- **unknown**（不可判定即不猜）：多候选并存（无法不运行构建模型判定生效者）、符号链接（不跟随，防位置伪造）、读取失败、预算超限（不完整读取不计算摘要）、`request_cancelled`、`request_deadline_exceeded`。
- 输入变化：`rule_config_recheck` 对 selected 摘要复核给出 `rule_config_changed_after_check` / `rule_config_removed_after_check` / `rule_config_read_failed_after_check`；缺配置观察在候选后来出现时给出 `rule_config_now_present_after_check`。

## 测试证据

初始新目标因缺少三个 API 编译失败（RED，3 处 `unresolved imports`，日志 `/private/tmp/codeguard-ksr-applicability-red.log`），与 ruby-candidate-baseline 记录的 RED 先例同型。实现后：

```bash
CARGO_PROFILE_TEST_DEBUG=0 cargo test --offline --locked -p codeguard-cli --features wasm-precheck --lib applicability_tests
# test result: ok. 21 passed; 0 failed（kotlin 7 + swift 7 + ruby 7）
```

反例覆盖：合法配置（configured 且摘要等于明文 SHA-256）、缺配置（missing 不伪造字段）、坏配置五类（tab 缩进/非 UTF-8/空白/无映射键/控制字符 → invalid 各自理由）、双候选歧义、符号链接、预算超限、取消、超时、输入变化（改动/删除/缺转现）。缺转现（`missing_observation_becomes_stale_when_config_appears`）单独做了 RED 证明：临时还原旧 `rule_config_recheck` 后 3 个目标全部失败，恢复修复后 21 个全过。整库 `--lib` 当次 141 通过、3 失败均位于并行智能体在改的 `javadoc_workbench.rs` 与 `javascript_syntax_probe.rs`（非本切片文件），日志 `/private/tmp/codeguard-ksr-applicability-green.log`。`cargo check -p codeguard-cli --features wasm-precheck` 对三个改动文件零警告。

## 边界与未完成

- 三个档案与配置发现**未接入任何对外报告**：kotlin-compile-scan-v0.2 / swift-parse-scan-v0.2 / ruby-syntax-scan-v0.2 及 lint-feedback schema 均为 `additionalProperties:false` 的共享文件，本切片无权扩展；接线需 schema 版本升级与 check_command/hook_fast_scan 改动（已在结果中登记 needsSharedFiles）。
- 未运行 detekt/SwiftLint/RuboCop/Gradle/Maven/SPM/semgrep/osv-scanner 任何工具；工具条件、报告契约、精度、平台资格全部未验收。
- 版本策略是登记策略，不是版本解析实现；`configured` 是结构合理性，不是语义或覆盖证明。
