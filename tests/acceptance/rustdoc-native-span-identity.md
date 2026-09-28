# Rustdoc 原生源码范围身份

对应OpenSpec native-tool-adapters原生证据Requirement、任务7.1/9.7的文档任务接线前置；不是完整持久任务验收。

旧行锚点加序号会在同一行crate/函数诊断消失一项后让另一项借用顺序身份。现保留原生byte_start/byte_end，必须有序且与真实捕获UTF8字节及原生行列一致。候选指纹绑定原生规则、相对源文件/目标和精确范围字节，不含绝对工作区路径、行号或诊断列表序号。源文件摘要另行绑定，行移动不会把新源码冒充旧证据。没有把规则文本/原始消息作为指令或以自写文档规则代替rustdoc。

同一原生记录可去重；同文件不同范围的相同声明产生身份碰撞时，保留全部观察，identity_status=ambiguous、finding_id=null，运行未完成。不能分配序号任务或关闭任一问题。尚无完整符号归属时这仍是unique_candidate，不能升级为批准身份或任务关闭。

反馈升级rustdoc-local-observation:0.2.0；旧0.1 schema按原字节保留为rustdoc-local-observation-v0.1.schema.json。新协议增加范围/摘要/身份状态及歧义ID约束；缺字段不能按新协议接受。尚无正式持久消费者，不能声称旧任务已经迁移。

TDD：新增身份API缺失时编译失败。重复声明CLI反例在旧实现中错误返回native_observed_unverified，修正范围身份和碰撞分流后通过。普通结果：3项身份契约（行移动、同一行crate/函数区分、越界/UTF8及位置错配）、6项CLI普通回归、7项机器流协议回归通过。默认CLI的原生用例仍忽略，独立运行结果另记；没有把忽略计为通过。

实际原生0.2反馈通过Draft202012；crate/函数候选ID不同，篡改为歧义却保留正式ID被schema拒绝，正确歧义调查形状可表达。独立Python仅验证协议，不属于产品运行路径。

完整符号/源码重构身份、稳定任务、task verify/关闭重开、配置/工具可信来源、全构建组合与宿主仍缺。源范围相同的不同符号保守地保留歧义，后续须绑定原生语义归属，不能靠宽泛豁免处理。最新原生及Clippy终态以OpenSpec verification.md记录为准。
