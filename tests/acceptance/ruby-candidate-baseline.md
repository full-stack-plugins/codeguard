# Ruby 六类别候选档案验收（OpenSpec 8.16）

日期：2026-10-05。此记录验收语言适用性、候选工具、版本选择方式及缺口契约；不是 RuboCop/Bundler/CVE/构建适配器的真实执行或发行验收。现有固定 MRI 2.6.10p210 语法链路保留其自身局部证据，不能覆盖六类别完整能力。

`rulepacks/ruby_static_candidate_v1.json`、`schemas/ruby-static-candidate.schema.json` 和 Rust `parse_ruby_candidate_profile` 固化六个不重不漏类别。MRI/JRuby/TruffleRuby 与五候选平台分别待验收，不能从 MRI 的局部样本推导兼容。工具版本采用 `project_locked / unvalidated_project_lock_required`：未来计划必须取得真实锁版本和工具制品，不浮动选 latest，也不把这份档案当作版本解析已实现。

| 类别 | 原生候选和适用性 | 一手依据及限制 |
|---|---|---|
| lint | RuboCop，Ruby 项目 | [配置与目标版本](https://docs.rubocop.org/rubocop/latest/configuration.html)；有效配置、继承、插件、解析器和规则报告未验收。`ruby -c` 只提供语法观察。 |
| comments | 显式 Style/Documentation 和 Style/DocumentationMethod | [原生规则](https://docs.rubocop.org/rubocop/latest/cops_style.html#styledocumentation)；分别针对类/模块和方法的注释存在性，方法规则非默认；不证明参数、返回和语义完整。 |
| dependencies | Bundler 项目的 `bundle list` | [应用依赖与锁](https://bundler.io/guides/using_bundler_in_applications.html)、[命令](https://bundler.io/man/bundle-list.1.html)；列表不能证明完整图，groups/platform/git/path、许可证和来源仍缺证据。 |
| cve | Bundler 项目的 bundler-audit 与 ruby-advisory-db | [原生 README](https://github.com/rubysec/bundler-audit)、[CLI 源码](https://github.com/rubysec/bundler-audit/blob/master/lib/bundler/audit/cli.rb)；必须绑定数据库快照及时效、锁节点及 advisory。原生 ignore 不等于 CodeGuard 批准。数据库缺失可能触发自动下载，未来执行层必须前置拒绝；`--no-update` 不足以形成离线保证。 |
| security | RuboCop Security；Rails 项目另有 Brakeman | [Security 规则](https://docs.rubocop.org/rubocop/latest/cops_security.html)、[Brakeman 范围与选项](https://brakemanscanner.org/docs/options/)；Rails 检查不适用于所有 Ruby，置信度不等于已证明违规。 |
| build | gem 项目的 `gem build SELECTED_GEMSPEC`；项目依赖判定 | [RubyGems 打包说明](https://guides.rubygems.org/make-your-own-gem/)；gemspec 是 Ruby 代码，需独立执行政策。任意脚本不能因为没有 gemspec 变为已构建；Rake/Rails/自定义构建应按项目正式配置决定。 |

所有类别固定 `gap`，build 使用 `project_dependent`，其余类别适用但各候选工具有具体范围。缺工具始终是准备/环境缺口，不是 not_applicable。档案只读 JSON，不读项目配置、不运行 Gemfile/gemspec/插件、不联网、不安装工具、不生成批准。

```mermaid
flowchart LR
    A[Ruby 六类别档案] --> B[候选工具与范围]
    B --> C[项目版本和有效配置待绑定]
    C --> D[可信执行计划待建立]
    D --> E[原工具正反例和报告契约验收]
    E --> F[修复工作流与平台验收]
    F --> G[完整能力资格]
    H[当前固定 MRI 语法观察] --> I[局部事实]
    I --> C
```

## 测试证据

初始新目标因缺少 Ruby profile API 编译失败（RED），日志 `/private/tmp/codeguard-ruby-candidate-red.log`；不是声称已运行工具精度测试。实现后 Ruby/Go 候选两个目标均通过，再追加版本输入、数据库更新命令、来源控制字符、未知批准字段、平台重复和字节预算反例。新 Ruby 目标四项覆盖六槽以及上述失败条件，源/方言/版本不足时不能虚报能力。

schema 使用真实内置档案，拒绝 implemented/verified、类别缺项/重复、工具错类别、formatter/语法探针冒充文档、Brakeman 泛化、gem 打包泛化、浮动版本、自动更新命令及伪批准。JSON 重复键由 Rust 严格读者拒绝；schema 不声称能发现 JSON 解析前重复键。

完整 adapters/default/WASM 回归、严格 Clippy、分层、格式及 OpenSpec strict 结果在完成后追加。只在这些档案验收通过后完成 8.16；8.17/8.18 的真实原生 lint/comments、依赖/CVE/security/build 和平台完整验收仍开放。

## English acceptance boundary

This closes only OpenSpec 8.16: six-category applicability, runtime dialects, native candidates, project-lock version selection, and explicit gaps. It executes no candidate command and grants no capability or approval. MRI syntax observations remain distinct from full RuboCop, documentation, dependency, CVE, security, and build coverage. Brakeman requires Rails; gem packaging requires a gem project. Native execution, precision, task closure, cross-platform and release acceptance remain in 8.17/8.18. The Rust parser bounds bytes and rejects duplicate/unknown fields and false capability declarations; JSON Schema independently verifies the bundled artifact and mutations.


最终本地 adapters/all-targets 共46个结果目标：197通过、0失败、8忽略，日志 `/private/tmp/codeguard-ruby-candidate-adapters-final.log`；新Ruby目标4通过、Go既有目标3通过。独立schema验证1通过，包含正例及18类变异反例。定向edition2024格式、分层及OpenSpec strict通过。

曾尝试仅对 adapters 指定 wasm-precheck，Cargo 明确拒绝：该crate没有此feature（记录 `/private/tmp/codeguard-ruby-candidate-adapters-wasm.log`）；不将此记作产品失败或WASM验收通过。该只读档案代码与features无关，WASM消费兼容性通过整workspace的WASM严格Clippy另验。未重复整CLI/full WASM测试或真实六类别原生执行，不以先前提交测试替代本批证明。

默认/WASM整工作区/all-targets严格Clippy均通过，日志 `/private/tmp/codeguard-ruby-candidate-clippy-{default,wasm}.log`；只是消费者编译及静态质量证据，不是WASM grammar资格。远端CI按提交独立核验。
