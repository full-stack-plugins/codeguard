# Javadoc 原任务复检局部验收

延续 introduce-rust-codeguard-cli C15及9.x。新增原任务JDK21复检、摘要绑定原配置/工具、输入记录前重核、租约和尝试绑定、next读取与失效分流。局部零诊断仍保持open，不授予可信关闭或白名单权限。

TDD初始两项新增行为测试失败（没有正确复检、未拒绝其它检查器参数）；实现后通过。七个受影响目标共81通过/0失败/28条件忽略。覆盖：原问题still_present、缺工具incomplete、工具字节变化incomplete、原任务事件保存、错参启动前拒绝、连续两次无进展转needs_decision，以及既有租约、跨类别和工作台回归。

真实已有JDK21新验收单独1通过/0失败/0忽略（是上述条件忽略之一，不重复计数）。原问题仍在→补齐类及构造函数文档→局部零诊断候选→只改变POM→规则覆盖需复核，均保存原任务事件并保持open。首次验收测试误读finding_id而不是已有事实协议id，修正测试字段后通过，没有修改事实协议。无安装、下载或发布。

```bash
CODEGUARD_TEST_JAVA_HOME=/absolute/existing/jdk21 cargo test --offline --locked -p codeguard-cli --test java_comments_cli actual_jdk_task_verify_absence_and_configuration_change_keep_open -- --ignored --exact
```

4份新schema元定义通过。真实CLI当前项目包装/next，以及still_present、消失候选、配置变更、缺工具四轮公开复检和原生容器均通过对应schema；伪造coverage拒绝。默认CLI全目标Clippy -D warnings、分层、OpenSpec strict通过。用户Erlang草稿字节保持不变，未执行。

未完成：可信关闭、复发闭环、原完整项目检查归因、Maven多文件和文件工作台，以及真实宿主与跨平台验收。9.x/6.x父任务仍未完成。旧协议和证据归档保留，不用局部新测试覆盖历史全量验收。
