# 白名单处置与原生 finding 关联（4.8/4.10 进行中）

RED 复现两种错误：完整观察身份与裁定一致，但 finding 的工具 ID 或主目标不同，旧纯领域门禁仍返回 AllowWithExceptions。另一个 RED 复现冲突处置：先处理的条目仍留在 whitelisted 集合，虽然整体门禁随后为 Incomplete。

DeliveryInput 新增默认空且为空时不序列化的 native_checker_bindings。NativeCheckerBinding 由受保护宿主冻结 checker_id/tool_id/category/obligation_id 映射；映射本身不具有批准权威。门禁只接受与该 finding 的检查器和具体义务对应的唯一映射，并核对工具、类别及合法引用。缺映射、重复/歧义映射、错类别/工具/义务或检查器均不应用处置，不按字符串后缀推断同一工具。

主定位采用 finding.locations 的第一处：源码或项目目标须路径完全一致；依赖目标须组件和显式版本完全一致。缺主定位、错类型或缺依赖版本均保持活动阻断与 Incomplete；次要位置不能覆盖不同的主目标。内容摘要、工具/适配器/规则包摘要、依赖图及 advisory 的真实归属仍须由宿主从原生报告和快照复核，领域模型不会凭路径相等认证这些来源。

门禁在应用任何处置前预先统计重复 finding 引用和重复批准决策 ID。重复 finding 即使使用不同决策 ID，也全部不应用；同一批准决策 ID 被复用于不同发现，也不保留先到的豁免。相关发现仍在原始及活动阻断集合，改变处置顺序不改变结果。

旧 DeliveryInput 可反序列化读取，但缺映射不能沿用白名单；普通无白名单的门禁不要求该新增集合。既有正例改为提供明确原生主定位及冻结映射，未用占位定位把缺定位用例改为通过。

delivery_gate_contract 26 项、check_session_contract 8 项、RunReport/带例外反馈/SARIF 14/7/3 项通过；core 自身回归通过，fmt 与全目标 Clippy 通过。一次专项误写不存在的 conversation_feedback_contract 等 target，未运行测试；已改为实际 target 重跑，不将失败命令计为通过。以上为合成领域与协议用例，不证明实际原生扫描或独立批准来源。完整工作区终态另记；4.8/4.10 与完整门禁接线保持未完成。

后补 RED 四复现：finding 归属与所在结果不符时，匹配的冻结映射仍可留下白名单状态。现在还要求源义务没有无效、未完成或覆盖失配标记，且 finding.obligation_id 与该结果 ID 一致。第 27 项包含错归属、未知义务与覆盖失配反例；这些调整另经目标回归，不追溯计入此前已启动的全量回归。

最终门禁 27 项、会话 8 项、RunReport 14 项、带例外反馈 7 项、SARIF 3 项与 core 自身回归通过；全目标 Clippy 和 fmt 退出 0。OpenSpec strict 与 diff check 通过。完整工作区终态另记，宿主授权与原生来源仍未认证。

完整工作区离线回归最终退出 0；后补源义务检查另经最终专项、core 回归、Clippy 与 fmt 验证。默认忽略的原生用例未计执行，完整计划与宿主门禁保持未完成。
