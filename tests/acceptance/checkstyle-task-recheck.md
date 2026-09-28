# Checkstyle 任务原工具复检

现支持 task verify <CG-id> <workspace> --java-tool <绝对路径> --checkstyle-jar <绝对路径> --config <绝对路径>。三个执行输入必须显式提供，不从可编辑 Markdown 或本地观察自动选择可执行程序。复用任务租约、统一预算/取消、保存报告、同步和 verification_observed 事件，状态始终 open。

四种基本观察：同一稳定 finding 仍存在为 still_present；原规则仍绑定同一检查类且原工具字节相同、未再检出为 candidate_absent_unverified_policy；移除规则、同 ID 换检查类或同规则不同锚点为 rule_coverage_requires_review；缺输入、原工具失败或工具身份变化为 incomplete。零诊断不能自己关闭任务；原生 @Override/@Generated 等默认例外、全部源集和批准覆盖仍需独立核验。

工具比较固定在任务首次摘要绑定的报告，后续局部报告不能滚动重新认可新 Java/JAR 身份。真实回放使用可正常运行但追加测试尾部字节的 JAR，连续两次均为 incomplete，证明不因新报告覆盖身份而逃逸。原输入在保存前重新核对；源码逃出项目边界不启动原生检查。完整 JDK 制品闭包仍未验证。

next 与尝试历史识别 Checkstyle 复检报告和 run 序号，反馈最新观察及具体失败原因；复检后的源码或配置再次变化要求复扫。暂未检出仅提示核验覆盖、策略和批准，失败观察指引修复环境而非修改无关源码。取消/超时边界、跨进程竞态、所有配置和宿主批准仍需完整验收；不宣称正式关闭或全语言完成。

TDD：入口先返回 2；实施后真实回放发现旧 next/尝试读者不识别复检容器和 Checkstyle 序号，分别返回 checkstyle_report_invalid 与 verification_event_invalid。补齐摘要绑定读取及协议分支后通过，未删除失败证据或弱化原任务状态。

验证：34 项普通回归通过；真实复检六轮一项通过（27.38 秒），另工作台真实回归一项通过（9.99 秒）。覆盖仍存在、补正文档后缺失、同 ID 换检查类、缺 JAR 和连续两次工具字节变化，均保存事件并保持任务开放。

补充协议验收：两轮真实 still_present/缺前置复检及对应 next 产物通过三个专用 schema；错误 coverage_proven=true 被拒。首次 next schema 仍沿用 Ruff 的 lint-复检 ID 和缺少 incomplete 枚举而拒绝真实输出，已改为 Checkstyle 专用 run ID 与明确未完成值；未放宽授权字段。CLI 全目标及后续局部变更 Clippy、格式、OpenSpec 严格校验和差异空白检查均通过。

## 旧 still_present 不得覆盖当前输入失效

真实六轮回放新增第一轮 still_present 后的配置和源码改动查询。配置原字节增加注释即失效，不以规则看似相同猜测语义等价；源码增加空行也必须重新观察当前输入，不能使用旧行号修复方向。恢复原字节后继续已有文档修复、规则身份变化、缺 JAR 和两轮改变 JAR 的回归。

原配置反例在修复前明确暴露：checkstyle_guidance.configuration_current=false，但旧 still_present 分支将 disposition 覆盖为 actionable。修正为当前工具 → 配置 → 已绑定源码失效检查优先于旧复检结果分类；三个失效原因独立于历史 verification_reason，旧 verification_observation 不进入当前简报。准备和源码任务共享该优先级，仍保留历史、原工具复检方向及开放任务，不签发批准或覆盖证明。

本轮验证：33 项普通受影响回归、扩展六轮真实源码复检（57.00 秒）通过；实际工具/配置/源码失效简报及状态 schema 通过，覆盖和任务关闭伪造变体被拒。CLI 全目标 Clippy、格式、OpenSpec 严格校验与差异空白检查通过。
