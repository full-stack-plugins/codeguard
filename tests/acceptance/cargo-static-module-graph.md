# Cargo 静态模块图

范围：OpenSpec 9.18/9.23 的 Rust 项目静态关系切片，不代表完整阶段完成。

新 adapter 在同次有界 Cargo.toml 字节上观察明确成员及直接路径依赖；图 0.3 分开结构 contains、声明 aggregation 与 build_dependency。关系绑定来源清单/SHA-256、声明条件、normal/build/dev 范围。依赖重命名采用 package 字段，真实目标包名不符、源不是包、目标缺失/未解析、路径逃逸或自依赖均不连边。workspace/optional/target、成员通配/排除、外部源、版本约束及 feature 保留具体 unresolved；图完整性始终 false。

init 不执行 cargo、build.rs 或 wrapper，不生成 Cargo.lock/target。历史 schema 0.1/0.2 单独保存；当前 schema 不把旧消费者升级为支持新关系。字段与图变动经已有受控画像刷新，人工任务备注保持不变。

新增 init 用例先因旧图 0.2 不含 Cargo 声明而失败。实现后的用例覆盖 workspace 聚合、重命名、normal/build/dev、本地 sibling path、错包名、缺目标/逃逸、optional/target/workspace 与通配/排除、依赖变化后旧边移除且保留备注和重复刷新幂等。adapter 用例覆盖合法声明、未解析条件/外部源、坏 TOML/重复字段、大小/编码上限、错误字段类型。

实际 CLI init 产物通过当前 JSON Schema 验证；5 个错误 scope/basis/status/摘要/完整性反例拒绝，历史 0.1/0.2 正例有效。schema 验证辅助使用现有环境，不加入 Python 产品实现。

终态：Cargo adapter 3 项、Maven adapter 4 项、受影响 CLI 契约 69 项、CLI 库 18 项通过；workspace all-target Clippy（-D warnings）、格式和 OpenSpec strict/diff 校验通过。没有运行 Cargo 有效模型或全 workspace/全语言原生检查，不能据此确认完整依赖图或质量通过。9.18 保持未完成。
