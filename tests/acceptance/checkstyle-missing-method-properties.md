# MissingJavadocMethod 原配置参数验收

状态：固定 Checkstyle 10.21.4 局部静态参数适配；完整项目模型与批准门禁仍未完成。

## 实现

新增该模块的 scope/excludeScope、allowedAnnotations、allowMissingPropertyJavadoc、minLineCount 和 ignoreMethodNamesRegex 原参数。按官方名称映射校验参数上下文，布尔/整数/注解名称采用静态有界检查，名称正则交给原工具。原 XML 不重写，未知/动态/跨模块参数不降级为默认规则或源码缺注释。

固定 JAR 的公开 setter 签名与构造器/私有方法字节码供本轮参数和边界核对；本地字节码观察不授予工具批准。配置排除不是 CodeGuard 的误报白名单批准，不证明项目必需规则覆盖。

## TDD 与夹具边界

完整配置绑定在实现前返回 None，修正后六项绑定和四项配置回归通过。跨模块借用及无效布尔/整数拒绝。

首次原生对照错误假定单行 getter/setter 必然得到缺注释报告；省略属性参数及显式 false 都只报其它方法。固定 JAR 的构造器将 minLineCount 初始化为 -1，方法行数对单行非空体计算为 -1，私有 isContentsAllowMissingJavadoc 按行数阈值允许缺失。因此不能把该结果误解成默认 getter/setter 豁免，更不能改写原诊断。

正常属性方法夹具改为多行 getter/setter，原文档/配置条件保持明确。四组配置比较注解/属性/名称排除、仅 public 范围、显式属性参数 false，以及长度阈值 2；最终零诊断观察仍保持历史任务 open。

```bash
cargo test -p codeguard-adapters --test checkstyle_binding_contract --test checkstyle_config_contract
CODEGUARD_JAVA_BIN=/absolute/java CODEGUARD_CHECKSTYLE_JAR=/absolute/checkstyle-10.21.4-all.jar cargo test -p codeguard-cli --test java_checkstyle_workbench original_missing_method_properties -- --ignored
```

本轮没有验证所有构造器、record 紧凑构造器、注解声明或全部 token/配置组合；这些与完整模型、批准来源和任务正式关闭仍属于后续范围。

最终真实四组回放通过（19.03 秒）：组合原参数只报 needsDoc；public 范围及显式属性 false 均报注解方法、普通方法、getter/setter 与 ignoredHelper；长度阈值 2 为零诊断但不关闭历史任务。共 19 项普通绑定/配置/CLI/工作台回归通过，CLI/adapter 全目标 Clippy 已通过。

夹具修正后的单测试目标 Clippy、格式、OpenSpec 严格校验与插件差异空白检查通过。
