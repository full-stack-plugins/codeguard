# 四核心生产验收计划

生产目标覆盖登记的 57 个语言、每语言四核心共 228 项义务。当前映射是仓库计划快照：全部资格仍阻塞，版本/方言未验收，五个平台仅为候选目标。读取成功不等于项目检查通过。

```bash
codeguard capabilities --acceptance-plan --format=json
codeguard capabilities java --acceptance-plan --format=json
```

此入口不扫描项目、不运行工具、不加载 WASM、不授予资格。旧能力库存 0.2 协议保持不变。筛选只缩小显示，完整义务数仍为 228；不能与旧六类别筛选混用。机器结果明确输出 `qualification: not_granted`、`delivery_decision: not_evaluated`。

语法必须分别验收 WASM 和原生工具；详细文档规范、开发规范和漏洞检查不能由语法解析替代。Java Maven/Gradle 独立登记，Gradle CVE 已有显式原任务/报告局部路径，但实际 OWASP 正反例仍缺；受控报告和真实缺插件阻塞不计完整扫描验收，Maven 局部工具结果也不是完整生产验收。没有生态原生 CVE 工具的语言需明确实际部署/依赖扫描路径及限制，不能虚构“官方 lint”。

32 个 grammar 按语言归属登记；TypeScript 包含 JavaScript/TSX，CFML 包含 CFQuery/CFScript。候选资产不等于发布能力，正式资格仍为 0/32。原生工具缺失时的初检报告不能关闭原生验收义务。

仓库计划保存在 `rulepacks/production_acceptance_plan_v1.json`；每项记录构建生态、适配器、证据、已有 OpenSpec 任务和未完成条件。Rust 审计示例核对引用文件摘要及任务存在性，不证明证据涵盖完整生产行为：

```bash
cargo run --offline --locked -p codeguard-cli --example audit_production_acceptance_plan
```

唯一规格事实源仍是 `openspec/changes/introduce-rust-codeguard-cli`，尤其 S15.1–15.7。S15.1 仍未完成：具体支持版本、方言、平台、独立精度和真实宿主结果必须继续补齐。以下是目标范围，不是已支持声明。

| 语言 | 构建/运行生态 | WASM 候选 | 详细文档/方言要求 |
|---|---|---|---|
| java | maven, gradle | java | Javadoc用途/参数/返回/异常及继承/record/生成范围 |
| rust | cargo | rust | Rustdoc用途/参数/返回/错误/Panics/Safety及目标特性 |
| typescript | npm, deno | typescript, javascript, tsx | JSDoc/TSDoc与TypeScript/JSX声明模式及重载契约 |
| python | pip, poetry, uv, pdm | python | docstring用途/参数/Returns/Yields/Raises及Google/NumPy约定 |
| go | go-modules | go | 导出API的Go文档与参数/返回/错误及泛型契约 |
| csharp | dotnet | csharp | XML documentation与参数/返回/异常及生成代码 |
| kotlin | gradle, maven | kotlin | KDoc与参数/返回/异常及Java互操作 |
| swift | swift-package-manager, xcode | swift | Swift API documentation及参数/返回/Throws与并发 |
| php | composer | php | PHPDoc与参数/返回/throws及类型契约 |
| ruby | bundler, rubygems | ruby | Ruby/YARD文档及参数/返回/raise与DSL |
| scala | sbt, scala-cli | scala | Scaladoc与类型参数/返回/throws及Scala版本 |
| shell | standalone-shell, enclosing-project | — | shell函数/脚本用途、参数、退出码和环境副作用 |
| dockerfile | container-build | — | 构建阶段/镜像依赖/运行入口和安全假设说明 |
| yaml | enclosing-project | — | 所属配置schema、字段用途、默认值和安全约束说明 |
| elixir | mix | — | ExDoc/moduledoc/doc/spec与返回/异常契约 |
| css | node-stylesheet-project | — | 样式公共接口、变量用途、约束及组件范围说明 |
| c | standalone, cmake, conan, vcpkg | c | Doxygen/API用途/参数/返回/错误与内存所有权 |
| cpp | standalone, cmake, conan, vcpkg | cpp | Doxygen/API用途/模板/异常/生命周期与线程安全 |
| objc | xcode, clang-project | objc | Objective-C API文档、NSError、ownership与nullability |
| dart | pub, flutter | dart | Dartdoc用途/参数/返回/异常与Flutter公共API |
| vue | npm-vue | — | Vue组件props/events/slots及脚本公共API |
| svelte | npm-svelte | — | Svelte组件props/events及生成代码边界 |
| astro | npm-astro | — | Astro组件props/slots与客户端服务端行为 |
| solidity | foundry, hardhat | solidity | NatSpec notice/dev/param/return及权限/失败条件 |
| terraform | terraform-providers-modules | terraform | 模块inputs/outputs/providers及副作用与敏感字段 |
| nix | nix-flakes, nix-expression-project | nix | Nix函数/选项、输入输出及求值/构建约束 |
| html | web-project | — | 组件/模板用途、属性、可访问性与嵌入脚本边界 |
| sql | database-dialect-project | — | schema/过程/查询用途、参数、事务与错误条件 |
| graphql | schema-client-project | — | schema description与字段/参数/返回/弃用约束 |
| protobuf | buf, protoc-build | — | message/field/service/rpc契约及兼容性 |
| markdown | documentation-project | — | 文档结构、示例有效性和事实时效，不冒充API源码检查 |
| toml | enclosing-project | — | 所属manifest/config字段、默认值及依赖范围说明 |
| haskell | cabal, stack | — | Haddock与类型/错误/效果契约 |
| ocaml | dune, opam | — | odoc与参数/返回/异常和模块签名 |
| fsharp | dotnet | — | XML/API文档与参数/返回/异常契约 |
| perl | cpan, perl-project | — | POD公共接口、参数/返回/错误及上下文 |
| groovy | gradle, maven | — | Groovydoc/DSL用途和Java互操作契约 |
| clojure | tools-deps, leiningen | — | docstring公共var、参数/返回及异常/副作用 |
| powershell | powershell-modules | — | comment-based help参数/输出/错误和权限 |
| zig | zig-build | zig | Zig doc comments用途/参数/返回/error set及分配器 |
| nim | nimble | — | Nim doc comments与参数/返回/raises/effects |
| crystal | shards | — | Crystal API文档与参数/返回/异常 |
| julia | julia-project | — | Julia docstrings与方法签名/参数/返回/异常 |
| elm | elm-packages | — | Elm module/value文档与类型/失败建模 |
| lua | luarocks, embedded-host | lua | Lua API文档与宿主参数/返回/错误/栈约束 |
| luau | luau-project, embedded-host | luau | Luau API文档与类型/宿主返回/错误约束 |
| pascal | freepascal, delphi | pascal | Pascal API文档与参数/返回/异常及编译器方言 |
| r | renv, r-package | r | roxygen2/R API文档与参数/value/错误/副作用 |
| cfml | cfml-engine-project | cfml, cfquery, cfscript | CFML函数/组件文档与参数/返回/异常及嵌入SQL |
| cobol | gnucobol, enterprise-cobol | cobol | COBOL程序/数据接口与返回状态及运行时约束 |
| vbnet | dotnet | vbnet | XML API文档与参数/返回/异常及生成代码 |
| erlang | rebar3, otp-application | erlang | EEP-48/EDoc/spec与参数/返回/异常/进程行为 |
| arkts | hvigor-ohpm | arkts | ArkTS公共API文档与声明限制/参数/返回/错误 |
| metal | xcode-metal | — | Metal函数参数、资源、线程组与运行约束说明 |
| liquid | shopify-theme, embedded-template-host | — | Liquid模板/过滤器接口及宿主输入输出约束 |
| cuda | cmake-cuda, nvcc-project | — | CUDA API文档、内存、kernel参数与同步/错误条件 |
| ansible | ansible-collections | — | Ansible模块/角色/collection参数/返回/权限和副作用 |
