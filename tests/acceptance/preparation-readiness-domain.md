# 准备状态领域判定

范围：OpenSpec 9.25 的纯领域契约与判定，尚未接入可信宿主、CLI 输入或准备任务，整项不勾选。

ReadinessInput 由宿主提供已核验要求与必需观察、完整性、可信时钟、适用性以及策略/目标/工具/配置绑定。两个 verified 字段只是纯领域输入约定，不构成签名、公钥来源或本地 JSON 授权。CLI 不接受项目文件来置真，当前 init 仍 unknown。

必需前置的本轮有效 Missing/Incompatible/Conflict 优先 incomplete；Unprobed/Unresolved、过期、未来观察、无时钟、错绑定及缺失/重复观察为 unknown。来源未核验不能产生确认阻塞或 ready。集合未完整、无适用必需条件、未知适用性、重复要求、零/坏摘要不能 ready。可选条件的缺失、冲突、未探测或旧证据不影响必需汇总；已确认必需阻塞与未解析兄弟并存时保留各自 ID，状态仍 incomplete。结果仅含前置状态、阻塞/未知 ID 和诊断码，不产生质量 allow。

新契约用例先因类型/函数不存在编译失败。实现后 8 个目标用例覆盖上述行为，含联合原生交付门禁反例：准备 ready，但必需检查没有原生结果，交付仍 incomplete。证据是合成领域输入，不是工具探测或可信宿主来源证明。

受影响 CLI 契约 90 项（readiness 8、会话 8、门禁 32、init 39、边界 3）及 core 单元/契约 9/8/5/3 通过。workspace all-target Clippy（-D warnings）、格式、OpenSpec strict/diff 检查终态通过。没有运行完整 workspace/全语言原生检查、批准准备清单或宿主端到端；准备任务规划/持久化与来源接线仍缺，完整计划继续实施。
