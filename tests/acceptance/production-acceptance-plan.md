# 四核心生产验收计划局部验收

对应现有 change 的 native-tool-adapters 新计划契约与 S15.1 进展。57 canonical 语言、四核心 228 项义务、五个候选平台及全部32 grammar 归属有明确仓库快照；版本/方言未授予支持资格。364 条构建生态×核心路径包含49 partial、26 wasm_candidate_only、1 configuration_only、288 not_integrated。全部核心资格保持 blocked。

公开入口：`codeguard capabilities [language] --acceptance-plan --format=json`。筛选 Java 时完整义务仍228，Maven CVE partial、Gradle CVE configuration_only；旧capabilities 0.2保持兼容。入口无项目访问、原生执行、WASM加载和交付判定。退出0表示读取成功。

测试先于命令路由实现：新增CLI测试因未知选项失败；实现后default/WASM均3通过。适配器契约3通过，覆盖语言/核心/平台/生态维度丢失、重复身份、未知字段、越界来源和伪造资格。旧库存4、选择3、帮助5通过。证据输出首次因相对路径按crate cwd解析失败；改用绝对输出路径重跑3通过，没有改宽业务断言。

Rust仓库审计示例实际核对52份当前来源SHA256与1312处任务引用，返回mapping_complete_qualification_blocked。真实公开全量JSON归档在[evidence/production-acceptance-plan/all.json](evidence/production-acceptance-plan/all.json)。两份独立schema验证计划和实际输出；伪造qualification/delivery_decision拒绝。default/WASM全工作区all-targets严格Clippy均通过。

边界：文件哈希和任务存在性不证明完整功能/独立精度，实际原生条件、版本、方言、平台、可信宿主和发行仍须完成。S15.1–15.7不勾选，正式grammar资格0/32。历史28e3760远端CI37481053826终态failure：gate固定插件审计源checkout失败，MSRV成功；不绕过审计源。
