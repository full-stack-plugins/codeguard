# Python3.14模板字符串原生反证与next纠错指引

对应既有OpenSpec 9.10、14.11、14.12、14.17；父任务未完成。

实际最小样本为`message = t"hello"\n`。模板字符串适用于Python3.14，语言依据为[PEP750](https://peps.python.org/pep-0750/)。固定Python grammar v0.23.6（SHA a7fdc587e77bd729b9f5b783c659be23c896e305a2c374472bed7114d9e01fac）在字节10–11产生ERROR；同字节Ruff0.16.8在明确py314下零语法诊断，在py313下报告invalid-syntax。这里只用实际Ruff语法结果作原生对照，未执行CPython3.14、完整lint、项目构建或安全检查。

RED一：两类首次WASM报告进入SDK后均记录false_positive_review_required且源码未改，但next仍为actionable；Python确认任务未接生命周期指引。修复只将该类确认任务接入既有历史读取，不从本地历史批准关闭。首次报告来源按native_evidence对象区分，不能将Python结构0.2版本误当原生首次。

RED二：误报记录之后改为缺函数体源码，next正确标记source_input_changed_or_unavailable，却被历史WASM反证指引覆盖。修复保留当前输入失效优先级；同时验证配置目标改为py313时configuration_input_changed_or_unavailable仍要求重新复检。

执行终态补于下。原生反证不自行白名单放行，不删除首次疑似，也不把未修复grammar升级资格或计为已解决误报。历史358例回放及其混淆计数保持原字节，不把新增Python3.14样本偷偷加入旧分母。


最终证据：真实Ruff两类来源误报调查/next与源码、配置变化保护1通过，54.99秒；披露grammar限制后再显式执行1通过，57.20秒。库存2、next7、Python兜底10、新SDK签名负例1共20通过/0失败/4条件忽略；两项真实Python SDK测试在普通回归中保持条件忽略，不虚报执行。既有Zig原生反证和环境失败不关闭回归1通过38.88秒。默认/WASM全目标严格Clippy、OpenSpec strict、diff检查通过。库存新增Python限制的RED实际失败，添加已知限制后通过；资产WASM摘要、release_status、公开npm和插件锁均不改。Erlang/VB.NET重建仍缺获准生成器，不将本轮next修复记成grammar修复。
