# 格式化工具链：真实环境的安装与声明

> 对应 `rulepacks/format_profiles.json`（57 语言档案）与
> `scripts/verify-format-profiles.py`（真机复验）。
> 回答一个具体问题：**真实环境里，格式化工具该由谁装、怎么装？**

## 核心原则

**工具由项目声明，codeguard 只校验不安装。**

理由有三条，都不是风格偏好：

1. **可复现性**：门禁结果必须只依赖源码 + 已锁定的工具版本。若门禁自行安装工具，
   同一份代码在两台机器上可能因工具版本不同给出不同结论——这与「门禁」的定义冲突。
2. **供应链安全**：门禁在 CI 上对任意 PR 运行。让它自动执行 `install` 意味着
   任意贡献者可以通过添加依赖声明，让 CI 执行任意安装脚本。这是提权路径。
3. **可诊断性**：工具缺失时，门禁必须说「缺哪个、怎么装」，而不是偷偷装上然后
   装出一个错的版本让人排查半天。codeguard 现在的行为正是如此——
   缺工具一律 `incomplete` + 退出 3 + `next_action` 指引。

因此三层的职责是：

| 层 | 职责 | 载体 |
|---|---|---|
| **声明** | 项目声明需要哪些工具、哪个版本、从哪来 | `format_tools.json`（本清单） |
| **安装** | 开发机/CI 按声明安装，在门禁之外 | brew / cargo / npm / 下载校验 |
| **校验** | codeguard 按 `--tool NAME=ABS_PATH` 绑定已装工具并校验身份 | `format check --tool ...` |

## 一、随语言工具链分发（推荐）

绝大多数语言的官方格式化器本就是该语言工具链的一部分，装语言环境时自然就有了：

| 工具 | 随什么来 | 覆盖语言 |
|---|---|---|
| `rustfmt` | `rustup component add rustfmt` | rust |
| `gofmt` | Go SDK 自带 | go |
| `clang-format` | LLVM / Xcode CLT | c, cpp, objc, cuda, protobuf, metal |
| `swift-format` | `swiftly` / Xcode 15+ | swift |
| `dart` | Dart SDK | dart |
| `mix format` | Elixir/Erlang | elixir |
| `crystal` | Crystal 语言安装 | crystal |
| `zig fmt` | Zig SDK | zig |
| `ocamlformat` | `opam install ocamlformat` | ocaml |
| `elm-format` | `npm i -g elm-format` | elm |
| `npx prettier` | `npm i -D prettier`（项目本地） | typescript, yaml, css, html, vue, svelte, astro, graphql, ansible |
| `format.sh` | 随 IntelliJ IDEA 分发（`CODEGUARD_IDEA_FORMAT` 可指定路径） | markdown |
| `ktlint` | Maven/Gradle 插件或独立 jar | kotlin |
| `cljfmt` | `deps.edn` / lein | clojure |
| `erlfmt` | `mix` / rebar3 插件 | erlang |

这类工具不需要单独"安装"，声明它们只是让门禁知道去哪儿找。

## 二、需要独立安装

这些不属于语言工具链，必须显式装：

```bash
# macOS（本次复验实际执行的）
brew install prettier google-java-format ktlint swift-format \
            php-cs-fixer rubocop sqlfluff cljfmt

# 通过语言自身生态
npm  i -g @ohos-rs/oxk          # arkts（ArkTS 专用格式化器）
cargo install --locked --git https://github.com/integrated-application-development/pasfmt   # pascal
dotnet tool install -g fantomas # fsharp

# 语言 SDK 自带编译器，直接可用
cobc                            # cobol（已随 GnuCOBOL 装）
```

## 三、当前无法自动安装的（需人工判断）

复验发现这几种在 brew 无 formula、也不在常规包源：

| 语言 | 工具 | 获取方式 | 备注 |
|---|---|---|---|
| nim | nph | 源码构建（Go 实现） | 非 brew 收录 |
| cfml | cfformat | 源码构建（Rust） | 非常新（0.3.x） |
| groovy | npm-groovy-lint | `npm i -D` | 需项目内安装，wrapper 按文件目录解析 |
| liquid | prettier + 插件 | `npm i -D @shopify/prettier-plugin-liquid` | 插件必须项目内预装 |
| scala | scalafmt | Maven：`com.geirsson:scalafmt-cli` | 无独立二进制 |
| pascal | pasfmt | `cargo install --git ...` | 需源码构建 |

**这些必须由项目自己决定是否引入。** codeguard 遇到它们缺失时的行为是
`formatter_not_found` → `incomplete` → 退出 3，并给出
`--tool NAME=ABS_PATH` 的绑定指引——不猜、不装、不静默放行。

## 四、怎么绑给 codeguard

工具装好后有三种绑定方式，按优先级：

```bash
# 1. 逐次显式绑定（调试用）
codeguard format check rust . --tool rustfmt=/opt/homebrew/bin/rustfmt

# 2. 放进 PATH（推荐，CI 与本地一致）
export PATH="/opt/homebrew/bin:$PATH"
codeguard format check all .

# 3. 配置文件固定（团队项目）
#    项目根写 .codeguard/tools.json，声明每个工具的期望路径与版本
codeguard format check all . --tool-lock .codeguard/tools.json
```

> 第 3 种的 schema 已存在（`schemas/tool-lock.schema.json`），
> 但格式化工具的批量绑定尚未接入 `format` 命令——这是待补的接线，
> 不是文档缺口。当前请用 PATH 方案。

## 五、CI 怎么配

**不要让门禁装工具。** 在 CI 的独立 step 里装，门禁 step 只跑校验：

```yaml
- name: Install declared formatter toolchain
  run: |
    # 与 format_tools.json 保持一致；门禁本身不装任何东西
    brew install prettier google-java-format ktlint swift-format \
                php-cs-fixer rubocop sqlfluff cljfmt
    npm i -g @ohos-rs/oxk

- name: Verify formatter profiles against real tools
  run: |
    cargo build --locked -p codeguard-cli --features wasm-precheck
    python3 scripts/verify-format-profiles.py

- name: Run format gate
  run: |
    cargo run --locked -p codeguard-cli --features wasm-precheck -- \
      format check all . --format json
```

装不上工具的 step 应该**不失败**——因为 `format check` 本身会对缺工具的语言
诚实返回 `incomplete`，由你决定这是否阻断本次交付。工具缺失不该让 CI 崩，
而该让门禁说清楚缺什么。

## 六、复验脚本的用法

`scripts/verify-format-profiles.py` 是这套机制的自检工具：

```bash
python3 scripts/verify-format-profiles.py --list   # 看本机能复验多少种
python3 scripts/verify-format-profiles.py          # 全量复验
python3 scripts/verify-format-profiles.py --lang go  # 单语言
```

它的价值在于：**改档案后能立刻知道参数和退出码是否还对得上**。
本次它抓出的 5 处缺陷（ruby/php/elm 的 stdout 误判、clojure 的漏判、
php 的码集错误）全都是人工审档案看不出来的。

## 结论

真实环境的分工是：

- **项目**声明并安装工具 → 锁版本、写进 CI
- **codeguard** 只做校验：找工具、按档案调、判退出码、缺工具就如实说

这样门禁的输出只取决于「源码 + 已锁定的工具」，不取决于「谁在哪台机器上跑」——
这正是门禁该有的性质。工具装不上不是门禁的错，是环境声明不完整；
codeguard 的职责是把这件事说得足够清楚。

## 七、IDEA wrapper：复现 IDE 内 ⌘⌥L 的排版

若团队日常在 IDEA 内排版（尤其 markdown：表格按列宽对齐、列表归一为 `*`），
门禁可用 IDEA 命令行格式化器复现同一效果。wrapper 已内置四项实测防坑：

1. `-d` dry run 做 check（2026.2+ 支持，退出码 0=合规 / 1=需重排）
2. `-allowDefaults` 恒传——否则不在 IDEA 项目里的文件被静默忽略（dry 仍退 0）
3. 丢弃 stdout——成功时也打印版本横幅，会触发门禁 A 类误判
4. `IDEA_PROPERTIES` 独立配置目录——IDEA GUI 开着时单实例锁会拒绝 format.sh

与 prettier 的 markdown 输出**不一致**（独立引擎：空格策略、列表标记、表格样式均不同，
详见档案 notes 的对比实测）。选择了 IDEA wrapper 就选择了与 IDE 内排版完全一致。

CI 注意：该档案依赖 IDE 安装，未装 IDEA 的 runner 对 markdown 诚实降级
（incomplete / 退出 3），不会静默放行。
