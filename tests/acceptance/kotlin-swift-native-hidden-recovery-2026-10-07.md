# Kotlin与Swift隐藏恢复节点原生复检

对应introduce-rust-codeguard-cli的14.17/14.19/15.2。当前源码，offline/locked、wasm-precheck；使用已有kotlinc2.4.10与Apple Swift6.4，不安装或下载工具。

Kotlin固定语料、隐藏恢复完整性和实际编译器差分3通过、0忽略，44.72秒。原生分类与固定标签一致，可判定WASM分类无分歧；object和missing_parameter_type仍unknown。Swift候选语料1通过；首次原生调用错误传入CODEGUARD_SWIFT_BIN导致缺少CODEGUARD_SWIFTC_BIN，不能隐去该失败。修正为CODEGUARD_SWIFTC_BIN=/usr/bin/swiftc后仅重跑受影响目标，1通过、0忽略、49.73秒；bad_param仍unknown，已判定样例无分歧。

[来源绑定与结果记录](evidence/kotlin-swift-native-hidden-recovery-2026-10-07.json)保存测试源码/grammar清单摘要与初次失败、修正结果。该记录是测试级证据，不是逐诊断原生报告或独立holdout；不授予语言资格，不证明自动路由、宿主、跨平台、版本范围或发行。

树has_error而可靠遍历位置缺失时继续保留unknown，不能伪造源码违规或clean。原生编译器可判定这些样例，但grammar问题仍需源级修复及固定字节重建，不能仅解析S-expression字符串制造位置。
