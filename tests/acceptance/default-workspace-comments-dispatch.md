# 当前默认工作区回归与 Java 注释入口兼容

日期：2026-10-06；测试源码基于4445f06，附本批rust_comments_cli回归修正。覆盖完整默认 `cargo test --workspace --all-targets --offline --locked`，不启用wasm-precheck。

第一轮完整回归在rust_comments_cli的invalid_arguments_stop_before_tool_execution失败，返回3而测试预期2。原用例把comments java作为非法命令；后续Java入口已实现，合法请求应是局部未完成，不再是用法错误。保留首轮失败日志，不能把首轮部分通过计作完整通过。

将该非法用例改为unknown-language，额外配置PATH中的哨兵Cargo，断言所有真正非法参数均在启动工具前被拒绝。新增合法comments java在没有Java源码的Rust工作区返回3/incomplete/not_evaluated，并核对Java报告身份。没有放宽产品参数或退出码。目标16通过、0失败、1条件忽略。

修正后重新执行完整默认工作区及所有目标，终态290组、1565通过、0失败、142条件忽略，进程退出0。默认构建中WASM专用目标是零测试，不把它们算作WASM验收；条件原生测试未执行，不借普通fixture通过签发工具准确率/真实宿主/交付通过。用户Erlang草稿在本轮不参与编译后的源码执行，其摘要保持2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6。

397份schema元定义均通过；实际grammar库存1.1和plan lint py的计划报告通过对应schema，库存candidate_count=32/released_count=0，别名返回canonical python。元定义和形状校验不能替代所有实际报告验收。提案首页与实现基线纠正“WASM全部待实施”过期状态，保留未完成资格和交付边界。

远端4445f06对应CI37423582241：MSRV成功，gate失败于Check out corpus evidence source，因插件固定提交dec5f9d远端不可达，后续门禁未运行。未改变CI锁或撤掉检查。该CI证据不用于新提交的通过证明。

```bash
CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace --all-targets --offline --locked
CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --workspace --all-targets --offline --locked -- -D warnings
```

全工作区严格Clippy、分层、diff与OpenSpec strict通过。详细源/日志/报告/CI摘要见 evidence/default-workspace-comments-dispatch-2026-10-06.json。完整WASM、142项条件测试、跨平台、真实宿主、源码审计远端前置和完整OpenSpec计划仍未完成。
