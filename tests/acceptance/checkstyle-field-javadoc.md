# 原生字段 Javadoc 检查验收

状态：Checkstyle 10.21.4 局部适配和本机验收；完整项目模型、原生全部规则与可信批准仍未完成。

## 实现

支持原配置 TreeWalker/JavadocVariable，保留自定义 ID、完整检查类、原生行列与严重度；修复指导要求核对字段或枚举常量的真实用途。新增 scope/excludeScope 与 ignoreNamePattern 原配置参数仅在该模块上下文接受，原生 Checkstyle 决定范围及正则语义，不在 Rust 重写字段规则。未知/动态/歧义原配置继续要求核对上下文。

现有稳定问题、任务、next 与 task verify 自动消费该绑定。更新当前反馈、观察、复检及简报 schema 的类枚举；历史 v0.1/v0.2/v0.3 反馈 schema 保留，不认可新检查类。旧严格消费者遇到未知类应拒绝解释，不能默默视为通过。

## 验证

TDD 初次绑定测试因未知 JavadocVariable 返回 None 失败；加入检查类后 scope 参数仍被拒，随后补齐原参数上下文。三个绑定、四个配置、四个结果契约和六个 XML 测试（共 17 项）通过。

真实原工具三轮回放通过（24.99 秒）：原配置 scope=public、ignoreNamePattern=log|logger、自定义 fieldDocs；公开 value 字段产生唯一问题，私有 hidden、log、serialVersionUID 和局部 local 无额外问题。原任务仍存在复检为 still_present，补充实际用途文档后为 candidate_absent_unverified_policy，事件保存且任务 open。此例没有宣称枚举常量/全部访问范围已完成原生验收。

实际字段发现记录、准备复检容器、命令反馈、源码/准备简报及状态 schema 通过；工具、配置和源码变化的实际失效投影仍可读取，覆盖/关闭伪造变体被拒。

```bash
cargo test -p codeguard-adapters --test checkstyle_binding_contract --test checkstyle_config_contract --test checkstyle_result_contract --test checkstyle_xml_contract
CODEGUARD_JAVA_BIN=/absolute/java CODEGUARD_CHECKSTYLE_JAR=/absolute/checkstyle-10.21.4-all.jar cargo test -p codeguard-cli --test java_checkstyle_workbench native_field_javadoc -- --ignored
```

参考：[官方 JavadocVariable 文档](https://checkstyle.org/checks/javadoc/javadocvariable.html)。当前文档的 accessModifiers 属于 10.22.0 及后续版本，本轮使用 10.21.4 的原生 scope，不把当前网页当成旧工具协议；行为结论来自固定原工具回放。

最终检查：共 26 项普通适配/CLI/工作台回归通过；CLI 与 adapter 全目标 Clippy（-D warnings）、格式、OpenSpec 严格校验和插件差异空白检查通过。

## 访问范围、枚举常量与 token 选择

新增静态 tokens 参数绑定：仅 JavadocVariable 的 VARIABLE_DEF、ENUM_CONSTANT_DEF 及逗号分隔组合；未知 token 和其它模块借用仍要求上下文。TDD 首次因为缺 tokens 支持失败，修正后四个绑定与四个配置用例通过。

原工具矩阵覆盖 public/protected/package/private、excludeScope、仅配置字段 token 与仅配置枚举 token，并将每组本机原生诊断位置与预期声明逐项比较，保持原生 ID 和工作台同步。任务及已消费收据不会因改变当前配置丢失历史，也不凭某轮未检出自动关闭。

首次矩阵发现同一行 `/** Second value. */ SECOND` 在 10.21.4 下仍得到原生缺注释诊断（第 8 行），不能假设有 Javadoc 字符串便必然通过。测试正常文档夹具改为独立注释行；这属于验收假设纠正，不是产品丢弃原生诊断或增设抑制。用户实际遇到该边界时应复核原工具归属及最小复现，不据本文直接判为误报或批准白名单。

原工具矩阵还证实：仅配置 ENUM_CONSTANT_DEF 时仍检出四个字段与枚举常量。固定 JAR 的 javap 观察显示 getRequiredTokens 返回 token 10（VARIABLE_DEF），getAcceptableTokens 包含 10 与 155（ENUM_CONSTANT_DEF）。因此修正验收预期和规格描述，不修改产品去过滤原生必需字段诊断；不能把 tokens 当成任意范围豁免。

修正原生契约假设后的七组矩阵全部通过（32.86 秒），各组原生位置、fieldDocs ID、稳定工作台同步及 coverage_proven=false 均核验。17 项普通绑定/配置/CLI/工作台回归通过；CLI/adapter 全目标 Clippy、格式、OpenSpec 严格校验与差异空白检查通过。仍未完成所有枚举语法、匿名/局部类、完整项目配置及批准门禁验收。
