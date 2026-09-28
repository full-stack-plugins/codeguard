# MissingJavadocMethod 原生 token 与构造器验收

使用显式 Java 21 和 Checkstyle 10.21.4 制品，执行 java_checkstyle_workbench 的 native_method_tokens_select_constructors_and_annotation_members 忽略测试。

同一源文件包含普通方法、普通构造器、record 紧凑构造器及注解成员。原 XML 中分别配置四类全部 token、METHOD_DEF、CTOR_DEF、ANNOTATION_FIELD_DEF、COMPACT_CTOR_DEF；原生诊断行分别为 [2,3,5,8]、[3]、[2]、[8]、[5]。Rust 不改写 token，也不补充非选中类别的发现。自定义规则 ID methodDocs 保留，首次同步生成四张稳定任务，后续扫描不重复生成。

保持最后的紧凑构造器配置：第一次 task verify 为 still_present；在紧凑构造器前新增独立行 Javadoc 后，第二次为 candidate_absent_unverified_policy。两次事件落盘，四张任务的事实仍为 open，coverage_proven=false。

TDD：静态绑定测试先因不识别方法 token 失败；适配后绑定七项及静态配置四项通过。原生测试七轮（五次扫描、两次复检）通过，耗时 51.11 秒。此验收只覆盖本版指定语法、显式静态配置和局部工作台；不证明完整 Maven/Gradle 配置、全部构造器语法、批准规则覆盖或正式关闭门禁。

配置连续性修正后，该矩阵的首轮配置为全 token，最后配置为仅 COMPACT_CTOR_DEF。因此紧凑构造器真实补文档后的观察现为 rule_coverage_requires_review，不能凭切换 token 的零诊断签发修复候选；同配置实际修复的候选正例另见 checkstyle-recheck-config-continuity.md。上述首轮通过时间保留为历史，最终代码复跑结果另记。

最终配置连续性分类下，该七轮原生矩阵重跑通过（42.08 秒）；不将切换 token 后的缺失观察当成修复候选。
