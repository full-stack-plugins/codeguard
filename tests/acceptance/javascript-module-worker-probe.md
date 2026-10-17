# JavaScript 显式 module worker/probe 与原生差分

日期2026-10-06，基础6cf2253，延续12.11/14.9/14.17/14.19，父任务保持未完成。新规则包固定root/return节点及六种函数边界，SHA a87fb74d19d215148c8be5c1785f7e872873f38fcf5e35a53631351a85455295。复用前批有界AST事实，必须显式module才能激活。

```bash
codeguard grammar probe javascript MODULE_FILE --module --format=json
```

请求绑定worker1.7的javascript_mode=module及probe0.8；未指定模式继续用旧协议，不推断CommonJS或项目配置。原始恢复、直接重复绑定和函数外return分别保留；共同输出上限128，扫描截断不解释成clean。函数/生成器/箭头/方法内return不报该候选，注释和字符串不当语法。零结构候选仍输出mode与空结构数组，读取失败保留mode与reason；均退出3/incomplete/native not_run。语言错配、未知/矛盾参数在解析前返回2。

父进程拒绝老版本、CommonJS、缺失/null模式、错误规则摘要/节点类型、未知字段及旧请求消费module报告。公开probe0.8封闭schema允许明确失败或完整候选两种形状，按固定grammar和规则摘要约束；旧schema文件不修改。help usage/examples已同步。

开发差分的Node原生观察固定--input-type=module，现采用相同显式module worker；报告0.10逐JavaScript行记录mode，其他语言禁止该字段。先修改真实Node测试，确认module_return仍false_negative的RED，再接线并复跑GREEN：18例原始5TP/11TN/0FP/2FN，组合7TP/11TN/0FP/0FN；duplicate_binding和module_return只补到组合指标，不改写raw。此为额外module语料，与固定358例及独立holdout分开。

最终八个相关WASM目标51通过/0失败/2条件忽略；真实Node24.18.0条件目标另显式1通过；默认help5通过。首次新增API测试因函数缺失E0425失败；API补齐后明确两个module请求返回2的行为失败；实现后一次报告目录创建竞态导致测试写文件失败，已确保各导出入口独立创建目录、捕获名称绑定本次唯一序号，再复跑全相关目标通过。不把测试夹具问题当源码语法问题，也不覆盖失败日志。

实际14份module probe报告、实际0.10差分报告、401份schema元定义通过；probe四种篡改、native五种篡改及旧0.6/0.9消费者拒绝通过。严格WASM CLI/adapters/all-targets Clippy、分层/OpenSpec strict/diff通过。新名归档输入、报告及摘要见[证据](evidence/javascript-module-worker-probe-2026-10-06.json)。

独立holdout=false、正式资格0/32。常规lint/check/Hook尚不推断项目module模式，未启用该规则；自动项目模式观察、任务/关闭/宿主、完整原生检查及发行仍开放。本批未发布npm/插件/市场，最新已确认远端CI40cc624的gate仍失败于固定插件证据源码checkout，不能用本地结果替代远端门禁。
