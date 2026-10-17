# Java comments 统一入口局部验收

对应 introduce-rust-codeguard-cli 的 C15、6.x 与9.x。本次补齐公开入口，不将父任务或完整 Java 闭环标为完成。TDD 初始新增四项测试为1通过/3失败，失败原因是入口缺失（退出2）；实现后三个目标合计15通过/0失败/2忽略。

覆盖：文件原生诊断与源码不变；项目配置和仅 Java 注释选择；缺配置不启动工具；重复/非法参数执行前拒绝；显式 Maven 不回退；显式预算及不可用路径。旧 Javadoc 和帮助契约回归保留。

真实 JDK21 单独执行新验收目标：1通过/0失败/0忽略。初始“仅给类补注释”的样例仍收到公共默认构造函数缺注释诊断；修正样例，显式构造函数及类均有文档后为局部零诊断。产品未屏蔽该规则。前后源码身份不同，Javadoc 工具身份相同；没有创建 `.codeguard` 或伪造任务。

```bash
CODEGUARD_TEST_JAVA_HOME=/absolute/existing/jdk21 cargo test --offline --locked -p codeguard-cli --test java_comments_cli actual_jdk_comments_change_from_missing_to_documented -- --ignored --exact
```

实际本机使用已有 `ms-21.0.12.1` JDK，无安装、下载或发布。新的 wrapper schema 校验文件、项目及不可用路径实际输出，拒绝伪造覆盖；验证器需登记同目录 schema 引用，与已有报告 schema 方式一致。

限制：原工具复检作为重新调用可用，但 Javadoc 持久工作台及任务 verify 适配未接通；报告显式 not_integrated。配置归因及局部零诊断不代表项目交付。未运行完整 WASM 回归或实际宿主测试。父任务保持待验收。
