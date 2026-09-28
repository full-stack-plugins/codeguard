# Checkstyle 配置正则失败的脱敏诊断与修复方向

## 目标与实现边界

固定 10.21.4 正常退出 254 的预算内 stderr，同时具有官方 CheckstyleException 非法属性值因果行，以及 PatternSyntaxException 因果行时，识别原 authorFormat/versionFormat/ignoreMethodNamesRegex/ignoreNamePattern 配置正则错误。只返回 checkstyle_configuration_regex_invalid，不返回属性值、路径或堆栈。未核验原生报告一致性时才参与失败细分，不能覆盖已有一致的诊断。

公开 lint 反馈 next_actions 明确要求修正原配置并用原工具复扫；稳定配置准备简报给出正则参数修复方向，要求不修改无关源码。失败仍 incomplete/无覆盖权威，不是源码 finding 或白名单。

## TDD 和执行证据

纯失败分类测试先因 API 缺失失败，实现后通过；错误退出码、未知属性/异常/包名、坏编码均不细分。统一 runtime 的 probe 用例覆盖缺报告、空报告、非预期退出、一致诊断和输入变化。

新增缺报告且输入改变的反例先失败（错误沿用 regex_invalid），修正为复核四个输入与共同取消/截止状态后通过；存在报告时输入变化也优先。进程非正常终止/超限不是正常 254，不匹配；留证失败不进行该细分。

真实 original_type_author_version_formats_preserve_native_failures_and_repair 七轮回放通过（38.75 秒），无效原生正则的对话 reason、next_actions 与准备简报正则指引均断言通过；仍保留四轮作者/版本正反例和一次原配置修复复检，事实 open。

另以实际原生无效配置报告验证当前反馈/准备简报 schema，不从合成声明推导门禁。完整属性级错误诊断、任意本地化文本与其它 Checkstyle 版本仍未验收；未知错误保留原失败原因和调查方向。
