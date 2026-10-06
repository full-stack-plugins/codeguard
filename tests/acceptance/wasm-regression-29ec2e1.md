# 当前WASM构建扩展回归收口

日期2026-10-06，CLI测试绑定29ec2e1；基础三个crate绑定8fcf0aa，两提交之间这些crate源码未变，逐路径核对。延续12.1/12.9/14.x，原失败日志保留在wasm-eslint-boundary-regression验收记录。此轮CLI从头重跑201个metadata登记集成目标，加lib/bins，不只运行此前失败后的余下目标。用户未提交erlang_native_differential明确排除，草稿摘要保持。

| 范围 | 组数 | 通过 | 失败 | 条件忽略 | 终态 |
|---|---:|---:|---:|---:|---|
| core/adapters/runtime WASM全目标 |80|420|0|10|退出0|
| CLI WASM lib/bins及201集成目标 |203|1459|0|167|退出0|
| 合计 |283|1879|0|177|两进程已结束|

独立解析日志Running tests路径，与Cargo metadata导出的预期目标排序逐项相等，确认无遗漏或重复、排除草稿。CLI examples未列入此执行范围，不能称包括所有可能构建产物；Clippy workspace/all-targets另覆盖编译检查。完整358例显式回放、真实原生条件测试、独立holdout及真实宿主没有在此轮执行，忽略不能当通过。

WASM全工作区/all-targets严格Clippy -D warnings、crate分层、OpenSpec strict及diff通过。398份schema元定义有效，实际grammar库存1.1通过精确schema，candidate_count32/released_count0/not_evaluated。初次验收脚本误搜索grammar-status文件名没有匹配库存schema，修正为grammar-inventory-v1.1后校验通过，不将脚本定位错误视作产品失败。

当前29ec2e1远端CI37427835909终态failure：MSRV成功，gate失败于Check out corpus evidence source，后续门禁未运行。本地回归不能替代远端成功。语言资格0/32、六项语料差异、177项条件用例、完整检测/关闭/宿主/平台/发行仍开放，父任务不勾选。文档更新不改变产品源码，日志和目标清单摘要见evidence/wasm-regression-29ec2e1-2026-10-06.json。
