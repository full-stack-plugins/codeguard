# Rust独立lint原生优先与工作台验收

对应现有OpenSpec `introduce-rust-codeguard-cli` 2.1/2.7/7.3/9.9/14.9–14.11/14.19，不新增第二套任务状态。

## 缺口、规格和RED

原有Clippy实现只能由check all/check rust触发，lint rust被Python入口拒绝。新增独立原生、显式无效和缺工具反例，默认首轮1通过/3失败，三项均因入口返回2而未取得Rust反馈。先补充原生工具和无Cargo两个规格场景，再接线。

## 最终行为

Rust参数独立解析，唯一绝对Cargo入口、human/json、共享timeout优先级；坏参数在观察项目/启动工具前拒绝。只调Clippy `--locked --offline --all-targets --message-format=json`，不调用rustdoc、build、CVE；沿用Cargo代理名、已观察源码/清单/锁/配置复核和私有target目录，不隐式安装。

独立与聚合检查共用Clippy持久化服务，保持原0.2局部观察。已初始化项目自动同步同一任务并提供next；原工具task verify记录still_present，重复扫描不新增问题。显式无效/原生失败不会改选工具或运行WASM。空Rust源码范围不启动所选Cargo。

未显式选Cargo且绝对PATH找不到入口时，WASM构建复用统一候选worker、同轮预算和语法确认任务。候选/未完成要求准备原生，完整有界范围零候选推荐准备；默认构建明确wasm_feature_not_built。独立feedback 0.1始终incomplete/not_evaluated，取消130，其余局部执行3。

help升级0.2列出Rust和参数示例，历史0.1 schema与实际报告保持原件。专用Rust修复简报schema覆盖原0.1简报的Clippy finding/blocker、六项原生复检argv、尝试/验证历史，实际报告驱动封闭验证，不放宽任意对象。

## 原生工具实际验收

显式本机 `/opt/homebrew/opt/rustup/bin/cargo`；依赖为空的edition2021项目、原锁不改，经原生Clippy发现needless_return并生成任务。源码去掉return后task verify同工具普通扫描和force-warn对照均local_scan_complete=true、零诊断，结果为candidate_absent_unverified_policy，事实仍open。首次测试误期望resolved_candidate导致失败；对照现有政策契约修正测试预期，没有修改产品结果或降低关闭要求。最终1项显式原生测试通过，2.57秒；实际原生/复检报告保留独立捕获。

## 限制

all-targets/default-features是局部探针，不代表全部workspace、features、目标平台、获批准规则和配置闭包。WASM候选不等于lint，Clippy局部零诊断不等于关闭或正式交付。未修改公开npm发行和插件锁。7.3、14.9–14.11及其它父任务保持未完成；当前提交的CI需独立终态。


## 最终本地验证

默认完整workspace实际结束退出0：250目标累计1375 passed、0 failed、124 ignored。随后Clippy要求使用等价unwrap_or_default，修正后最终默认四目标33 passed/0 failed/4 ignored；最终WASM七目标52 passed/0 failed/5 ignored。独立原生Clippy实际发现/修改/复检最终1 passed（2.13秒），忽略用例不计入原生验收。默认/WASM workspace all-targets严格Clippy分别7.96/9.89秒通过，改动Rust格式、分层、OpenSpec strict、diff检查通过。

新feedback及五份实际捕获（原生、复检、候选、零候选、help）按封闭schema验证；开发回归在WASM和默认二进制分别4通过，拒绝伪造执行许可、批准、工具available和未知版本。历史help0.1的两项回归仍通过；help0.2与原0.1按新旧协议分别消费。完整WASM suite、全语言精度、真实宿主及发布没有借用这批局部测试作为证明。
