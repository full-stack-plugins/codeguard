# Maven Javadoc 运行后身份复核回归

对应 introduce-rust-codeguard-cli 的3.x/6.x/9.x：在已有私有 Maven Javadoc 探针接入工作台之前，补齐 JDK release 元数据的运行后身份复核，避免工具链身份变化后保存旧诊断。原观察协议、原规则、POM重放资格与门禁状态不变。

三个公开CLI测试使用受控Maven进程夹具与已有探针执行路径。原源码、原POM、JDK release分别在私有副本检查期间被改写：前两项通过 SourceSnapshot 已有两侧复核被拒绝；release原先只在启动前读取、运行后未复核，实际反例仍收到 findings_observed_untrusted 和原生候选。现重新有界读取release字节，与启动前捕获字节不同或不可读时返回 native_identity_changed_during_scan、零诊断及零观察文件数。

TDD初轮1通过/2失败。源码/POM反例的失败源于测试错误预期新原因名：SourceSnapshot 已复核原件并返回既有 native_inputs_changed_during_scan，修正测试预期，没有重写快照行为。release反例暴露真实产品缺陷；补齐运行后复核后通过。未改变完整性判断或抑制规则。

最终三目标47通过/0失败/10条件忽略：新增输入身份3项、既有check_all_java_p3c 26项、java_comments_cli 18项。条件原生用例未执行，不算真实Maven验收。本轮本机已有Maven与JDK，但此前隔离Javadoc离线仓库未定位到，未下载或安装制品。CLI全目标Clippy、分层、OpenSpec strict与diff检查通过。用户Erlang草稿不变，未执行或提交。

```bash
cargo test --offline --locked -p codeguard-cli --test maven_javadoc_input_stability --test check_all_java_p3c --test java_comments_cli
```

剩余：Maven诊断的源码摘要持久绑定、稳定任务同步、原任务复检以及完整项目覆盖、可信关闭、宿主验收。六项WASM差异保持开放。本次不是Maven工作台完成证明，也不代替真实Maven插件运行验收。
