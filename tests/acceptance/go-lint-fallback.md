# Go统一lint的缺工具候选回退验收

日期：2026-10-05。沿用introduce-rust-codeguard-cli的14.5、14.7、14.8、14.19与Go生态父任务；本记录是局部实现证据，父任务尚未完整验收。

## 行为与边界

原生选择优先显式 --go-tool，其次调用方绝对PATH目录中的普通可执行Go；相对PATH目录不选择。显式无效工具和已选工具的版本/执行故障保留原报告，不用WASM替换故障。真正缺原生工具时复用有界项目发现、固定Go worker与完整文件结构规则，沿用共同deadline、64文件和两worker并发上限。未知/读取失败/范围预算不足保持未完成。

缺工具输出新的go_lint_fallback_feedback0.7，内含原go_lint_local_feedback0.6、工具选择、候选观察、任务同步、初检状态及原生工具准备要求。候选需要原生准备和确认；范围完整的零候选只推荐原生准备，仍不代表Go vet/类型/依赖/CVE或交付通过。无WASM构建保留wasm_feature_not_built，不伪造已执行。原0.6报告继续独立保存同步，候选以通用确认0.8导入；外层对话报告不是第二份原生执行收据。

## 实施与验证

新公开目标先RED：缺工具只输出旧原生未完成报告，无候选、无任务。实现后缺package结构、合法声明零候选、读取失败、重复lint/check稳定任务、显式坏工具及PATH已选坏版本六测试通过，后续相对PATH忽略与 --format=json 参数测试另通过；源文件保持原字节，补声明只消除候选，任务继续open。新Go工作台回归采用明确空PATH模拟缺工具，避免偶然调用本机SDK；保留原生报告断言，不以外层新协议改变原生事实。

闭合协议复用已有候选字段约束，同时允许合法无结构观察及未读取文件的language=null；null必须是candidate_unavailable，不能当成正常Go观察。要求推荐级反馈只对应非空、无遗漏、无恢复和结构候选的本轮观察。开发schema测试另核对错误语言、伪造原生完成、错误推荐和范围遗漏；记录最终运行结果于下。

```bash
codeguard lint go . --format json
codeguard lint go . --go-tool /absolute/sdk/bin/go --format json
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test go_syntax_fallback_candidate --test go_lint_cli --test go_work_sync
python3 tests/go_lint_fallback_schema.py
```

独立grammar资格、全量Go六类别义务、可信关闭、实际宿主自动触发、跨平台与npm发行仍未完成；不因为此入口可用勾选全语言或WASM父任务。

## 最终局部结果

WASM CLI单元77通过/3条件忽略；Go来源身份、lint、工作台、package公开链路及新回退集成回归通过，最新回退专项7通过，既有lint5通过/1条件忽略。默认构建Go lint/来源/工作台12通过/4条件忽略。显式既有Go1.23.4原生vet违规/干净/编译错误测试另1通过（21.08秒），没有安装工具或执行项目源码。新实际schema3测试通过，包括单独保存的无WASM默认构建；前轮Go package/任务协议5测试继续通过。默认/WASM两配置workspace/all-targets严格Clippy、定向格式、strict OpenSpec、分层及diff校验通过。

远端前序792fccc CI仍运行，未作为本轮完成证明。当前完整工作区/32grammar、真实宿主和本轮远端CI仍须独立确认；完整目标继续active，不以局部绿色验收关闭父任务。
