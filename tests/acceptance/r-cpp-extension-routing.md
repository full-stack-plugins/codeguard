# R/C++ 后缀发现与候选检查验收

日期：2026-10-05；对应现有 OpenSpec 14.4、14.6、14.19 的局部范围，父任务保持开放。

## 缺陷和依据

旧 registry 与 grammar 路由遗漏大写 `.R` 和六种 C++ 后缀，源码存在但没有候选检查。[R 官方工具文档](https://www.stat.math.ethz.ch/R-manual/R-devel/library/tools/html/fileutils.html)示例使用 `.R`；[Clang 官方后缀分类](https://clang.llvm.org/doxygen/FrontendOptions_8cpp_source.html)将 `.C`/`.cp`/`.CPP`/`.c++`/`.cxx`/`.hxx` 明确归为 C++。本次不扩大到模块、预处理文件或未知大小写组合。

## RED 与实现

`r_and_cpp_explicit_suffixes_keep_case_sensitive_routes` 原路由对 `analysis.R` 返回零项，退出 101。`detect_includes_r_and_cpp_explicit_suffixes_without_guessing_shared_headers` 旧发现只有 `lower.r`，缺 `upper.R`，退出 101。测试为不同大小写采用不同 basename，避免 macOS 不区分大小写文件系统覆盖样例。

registry、兼容 C++ gate 模板及候选路由补齐上述后缀；保留原 C `.c`、registry 对 `.h` 的既有观察行为，以及 grammar 对 `.h` 的不猜测行为。

## 实际检查

默认工具隔离的真实 `check all` 执行八份文件：R 两份、C++ 六份，含合法 `.r`/`.hxx` 对照与非法其它样例。全部产生对应语言的 `candidate_observed`，非法样例有恢复、合法对照无恢复；所有 grammar 仍未资格验收，退出 3、交付 `incomplete`。

三个 WASM 测试目标：check_all_grammar_candidates 13、detect_cli 34、grammar_route 7，共 54 个通过、零忽略。包含既有全部 32 种 grammar 项目路由及编辑 Hook 的实际执行。本次不改变 WASM 字节，不重写历史语料，不证明原生 C++/R 检查器、版本兼容或完整质量验收。

默认构建下 detect_cli 34、capability_inventory 4、capability_selection 3、legacy_v1_protocol_contract 3，共 44 项通过，验证 registry 的默认发现、能力库存和兼容协议。WASM 全目标严格 Clippy、定向 rustfmt、OpenSpec strict、crate layering 与 diff 检查通过。
