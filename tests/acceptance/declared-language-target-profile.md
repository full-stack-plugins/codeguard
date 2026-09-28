# 逐清单语言目标画像

范围：OpenSpec 9.16/9.17 的 Maven/Cargo 静态目标切片；其它生态、有效模型和本机工具版本仍未实现，不勾选整项。

project profile 0.3 新增 language_targets，每条记录 language_id、target_kind、value、declared_only、build_root、manifest_ref 和清单 SHA-256。Maven 仅观察直接 properties 中的 compiler release/source/target；Cargo 仅观察直接 package rust-version/edition。不同模块分别保留，不从包版本、默认 edition、父/profile/workspace 或本机版本生成全局语言版本。汇总 declared_version 与 installed_version 仍为空、状态 unknown。原始清单读取有界且目标依据与同次摘要绑定。

重复、变量、嵌套/注释分段 XML、继承/错误类型及非法值不产生目标，具体缺口进入画像 unknown_conditions；模型的其它未解析信息仍保留，不宣称配置有效或运行时已安装。历史 profile 0.2 schema 单独保存，当前新增目标不会回写成旧版本支持。

新增 init 用例首先因旧 profile 为 0.2 而失败；实现后测试同项目 Java 17/21 与 Rust 1.70/2021、1.85/2024 的逐根声明，不把包版本 9.9.9 当语言目标，本机版本保持未探测。反例覆盖变量/重复/嵌套/分段/继承/非法字段；声明从未知改为 17 后刷新画像、清除旧未知项且保留任务备注，重复刷新幂等。

目标 adapter 4 项及已有 Cargo/Maven 3/4 项通过；受影响 CLI 契约 71 项（init 33、detect 20、check-plan/config 各 6、边界/plan 各 3）与 CLI 库 18 项通过。真实 init 产物通过 JSON Schema 验证，6 个语言/edition/status/摘要/来源/本机版本错误反例拒绝，旧 profile schema 正例有效。schema 验证辅助不作为 Python 产品实现。workspace all-target Clippy（-D warnings）、格式、OpenSpec strict 和 diff 检查终态通过。

未运行全 workspace/全语言原生检查、Maven/Cargo 有效模型、安装工具或端到端质量交付；画像声明不能作为工具准备或 gate 通过。
