# 插件默认Rust生命周期当前回归（2026-10-05）

对相邻codeguard-plugin本地`dec5f9d1361eefff493e120b4278939a243814a9`进行只读验收，未修改或发布插件。插件采用独立OpenSpec变更`2026-10-03-rust-wasm-runtime-candidate`和`2026-10-04-rust-lifecycle-default`；CLI不创建第二份任务状态。

Framework Python3.13现有Ruff/MCP环境：完整unittest655通过/0失败/0跳过（123.748秒），集成144通过/0失败/0跳过。技能vendor离线/在线、Ruff、架构、57语言/11规则、便携manifest/镜像以及两个OpenSpec strict通过，工作区仍干净。

锁定公开npm0.1.4 tarball从注册表只读获取，摘要严格匹配a0d605f4a102e78d42766afaef61c6bd002556ccc5d16881c7553a91d6e20c80。仅安装在测试的临时缓存，不修改全局工具。实际Node回归8通过/0失败/0跳过，包含真实Zig0.16.0复检、默认五生命周期、稳定任务、失败编辑和活动制品篡改拒绝；执行时间8.353秒。直接默认入口编辑测量403.96/333.92/334.44毫秒，不当作真实安装宿主或p95证据。缺tarball首轮6通过/2跳过，未借用其作为完整包验收。

远端main仍为f09c074e2ecc2bfcccbba2ef132d20391c98bc53；本地三笔待推送提交涉及候选运行时和默认生命周期0.20.0。GitHub主分支规则要求PR与三Python版本检查；已请求用户按工作区Git约束批准创建送检分支，不能以旧远端CI批准新本地提交。对应插件2.4真实宿主、3.2完整远端CI和3.3发行仍开放；本轮不更改任务勾选或插件版本，不替代Rust当前源码的CI。

## 日志摘要

- `/private/tmp/codeguard-plugin-020-unittest-20261005.log`：SHA-256 `dbc81b50de65cb68cd2104861c4a1db320ec83184d65bb2858ed77d37bca0f50`。
- `/private/tmp/codeguard-plugin-020-integration-20261005.log`：SHA-256 `bcf4ebed94bd35b068ce77065009454774a2af3605450c595726d91a901add5b`。
- `/private/tmp/codeguard-plugin-020-real-runtime-20261005.log`：SHA-256 `4fe29ecaacd3f5679bb63739cd59797f7961824d8d9a9a7c906ac0d21a4d093e`。
