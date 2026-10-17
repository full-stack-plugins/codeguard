# Javadoc 主源码归属与嵌套构建根隔离

日期：2026-10-06。对应唯一OpenSpec变更的native-tool-adapters / Javadoc source selection SHALL bind to the nearest build root，关联6.1、6.3、6.6；完整父任务仍未完成。

## 实际缺陷与修复

原JDK单文件路径按完整文件路径是否包含`/src/main/java/`判定源集，外层POM因而可能授权没有独立构建根的`vendor/src/main/java/`。原Maven多文件路径按前缀收集源码，在父主源码目录内存在嵌套POM时，同时把子构建根的主源码纳入父探针。

两项新增CLI反例实际RED：vendor文件返回native_probe_returned；父探针source_count为2，预期仅自己的1份。修复复用最近Javadoc构建根选择，并以该根为基准识别主源码目录。子根未配置仍遮蔽父配置，不借父配置启动扫描。文件仍留在报告和总源文件计数中，源集未确认保持incomplete；不将跳过解释成clean。

## 验证与边界

默认check_all_java_p3c、java_javadoc_cli、java_p3c_workbench三目标51通过/0失败/7条件忽略。显式已安装Microsoft JDK21.0.12.1的真实测试另行1通过/0失败/0忽略，1.85秒：合法主源集文件产生JavadocMissingComment；同项目vendor文件没有单文件探针结果；总2份源码仅1份已观察，local_probe_complete=false。

Maven分组测试使用缺失工具来核对执行前source_count及未配置子根遮蔽，未实际运行Maven插件，不伪称Maven原生规则验收。协议字段/版本、工具批准、完整项目覆盖和门禁保持不变。自定义sourceDirectory、动态effective模型和全部Java组合仍缺，6.1/6.3/6.6不勾选。没有改动grammar、发布包或用户Erlang草稿。

复跑：`cargo test --offline -p codeguard-cli --test check_all_java_p3c --test java_javadoc_cli --test java_p3c_workbench`。真实测试须显式指定已安装JDK：`CODEGUARD_JAVA_HOME=/absolute/jdk21 cargo test --offline -p codeguard-cli --test check_all_java_p3c real_jdk_check_java_reports_javadoc_comment_probe -- --ignored --exact`。

最终WASM构建的Javadoc筛选目标5通过/0失败/2条件忽略，与默认目标重叠不累加。默认/WASM CLI全目标严格Clippy、所改文件rustfmt、OpenSpec strict、crate分层和diff检查通过；未执行完整workspace、完整WASM语料或未提交Erlang草稿。
