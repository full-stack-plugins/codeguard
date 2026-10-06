# P3C 配置准备任务聚合协议验收

延续 introduce-rust-codeguard-cli 的2.x/9.x/12.x。Maven复检回归的实际check java输出暴露两个协议缺口：旧聚合0.38不容纳P3C配置准备简报；较新聚合schema未接受CLI既有Java选择原因码。完整实际报告失败证据保留。目标测试要求新版本0.58，先失败再修改产品。

新聚合0.58仅在选中java.maven.p3c/p3c_configuration_not_confirmed任务时使用，添加严格独立准备简报schema，保留历史schema字节。简报限定blocker、原检查器、原原因和review-project-policy动作、固定原检查参数，保留任务历史。缺配置不是源码违规，也不自行新增必需检查或签发通过。schema明确现有Java选择原因，不采用开放对象兜底。

四个受影响目标57通过、0失败、10条件忽略。测试实际捕获0.58聚合报告，schema验证完整报告及顶层next、native_results.java_p3c.next两处实际简报。检查器、finding类型、原因、源码修复动作、批准权威、allow六个篡改均被拒绝；替换为旧0.38版本仍被历史消费者拒绝。两份新schema元定义通过。CLI全目标严格Clippy、分层和OpenSpec strict通过；用户Erlang草稿摘要不变。

```bash
cargo test --offline --locked -p codeguard-cli --test check_all_java_p3c --test maven_javadoc_input_stability --test status_show_contract --test java_comments_cli
cargo clippy --offline --locked -p codeguard-cli --all-targets -- -D warnings
```

本批P3C配置缺失不执行原生Maven，Maven过程测试是受控夹具。完整P3C finding/其它阻塞协议、原生完整覆盖、策略审批、真实宿主、发行和整体计划仍开放。不借协议正确声称代码质量通过。
