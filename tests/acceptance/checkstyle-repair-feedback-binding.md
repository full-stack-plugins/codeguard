# Checkstyle 原配置规则关联与修复反馈验收

局部反馈协议升级为 0.2.0，保留 0.1 schema 供旧消费者明确拒绝未知协议。原生自定义 source ID 从同一原配置映射到内置检查类，不按消息相似度、短类名或声明顺序猜测。重复 ID、未知版本、配置不明均拒绝关联；报告出现未绑定 source 时返回 checkstyle_rule_source_unbound，不发布源码 findings。

四类注释规则反馈包含原生 rule_id、checker_class、中文 rule_summary、官方 rule_reference 和 repair_steps。顶层 recheck_argv 使用原始输入的规范化绝对路径与字面参数数组，不引用临时快照，也不交给 shell 执行。修复方向要求核对真实契约、继承或生成源码；零诊断不单独关闭任务，也不自行加入白名单。

验证：普通 CLI 两项与规则绑定两项通过；真实 Checkstyle 10.21.4 CLI 一项通过（7.38 秒），涵盖自定义 ID 反馈、重复 ID 拒绝和不同 ID 多诊断。真实工具仍是显式本地测试依赖，不证明宿主批准、完整项目覆盖或交付通过。

复现：cargo test -p codeguard-cli --test java_checkstyle_cli；cargo test -p codeguard-adapters --test checkstyle_binding_contract。真实回放须显式提供 CODEGUARD_JAVA_BIN 与 CODEGUARD_CHECKSTYLE_JAR，再运行前一命令并追加 -- --ignored。

官方参考链接已核对可访问；在线文档随上游更新，10.21.4 的 source 语义以固定版本原生回放为准。

补充验证：配置/退出范围/XML 回归 14 项通过，CLI/适配器全目标 Clippy 与格式检查通过；schema 两份真实输出通过、五个错误变体被拒、旧 0.1 消费者拒绝 0.2。首次路径断言按未规范化临时目录比较失败，改为规范化原输入路径后通过；未修改产品路径行为。OpenSpec 严格校验和差异空白检查通过。
