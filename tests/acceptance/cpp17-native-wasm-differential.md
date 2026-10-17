# C++17 原生与 WASM 对照局部验收

对应 introduce-rust-codeguard-cli 的 8.29、15.2、15.7。使用现有 Apple clang 21.0.0 (clang-2100.3.34.2)，不安装工具。

命令：`CARGO_PROFILE_TEST_DEBUG=0 CODEGUARD_CLANG_BIN=/usr/bin/clang cargo test --offline --locked -p codeguard-cli --features wasm-precheck --test cpp_native_differential -- --include-ignored`。

格式化后的实际结果：1 passed、0 failed、0 ignored，17.81秒。12例全部执行原生和CLI grammar probe：7例合法语法（namespace/template/lambda/structured binding/constexpr if/fold expression/raw string）、3例损坏语法、2例类型或未知符号错误。最后两例原生拒绝而WASM接受，刻意不将语法成功解释为语义检查通过。全部检查源码未被修改、候选资格false、delivery_decision为not_evaluated。

测试源码SHA256：`10c540b6e49de7fad3551dc34578f040721d961481442fc2663a3a38ffd4f867`。

这是作者回归语料与本机真实工具对照，不是独立盲测、项目构建配置、头文件、平台矩阵或真实宿主验收。不升级C++资格，不勾选父任务，32 grammar生产资格仍为0。

[机器证据](evidence/cpp17-native-wasm-differential.json)记录工具版本、编译器及CLI摘要、逐例输入摘要和实际原生/WASM结果；仅在全部断言通过后写出。证据采集环境变量为CODEGUARD_CPP_DIFFERENTIAL_EVIDENCE，必须为绝对路径。
