# 受管工具字节的原子缓存发布

对应 introduce-rust-codeguard-cli/execution-kernel、5.3/5.6 安装基础层。不是 tools install CLI、可信发行来源、下载或完整安装验收。

## 实现契约

runtime `publish_tool_bytes` 接收调用方冻结的非空字节（最多 128 MiB）、非零预期 SHA-256、剩余截止时间、取消标记及已存在的绝对缓存根。此 API 不授予安装授权；正式服务必须先核验批准锁、发行来源、平台与包格式。

- 缓存根以 O_DIRECTORY/O_NOFOLLOW 打开，归当前用户且不可被组/其它用户写入。
- 暂存使用目录 FD 内 openat 的 O_EXCL/O_NOFOLLOW；名字冲突有上限重试，不覆盖崩溃遗留文件。
- 分块写入并检查取消/预算，设置 0700，fsync 后重读完整字节核对摘要、长度、类型、UID、权限和单链接。
- 同目录 linkat 原子新增 `<sha256>.bin`，不替换已有名字；根目录同步，最终文件再次核验。
- 复用须重新核对实际普通文件；拒绝损坏内容、链接、FIFO、错误权限或所有权。正常返回及失败路径清理本次暂存，旧遗留不自动删除。
- InstalledArtifact 只有相对定位、新发布/复用标记和字节数，不含批准、启动、准备或质量结论。
- 取消/预算检查不强制中断不可中断的文件系统调用。发布后的同步、复核或预算失败可能保留完整最终制品，返回失败而不冒充完整安装；后续按同一身份重新核验恢复。
- 同用户恶意进程及特权进程可修改本地缓存，不能声称不可绕过。正式使用仍核验可信来源和当前字节，缓存位置自身不是授权。

## 验证

五项目标用例最初因接口缺失编译失败，实现后通过；进一步加入硬链接/空输入、遗留暂存与多制品、并发重试、超限/零摘要、FIFO，共 10 项普通发布测试通过。

fresh_report 10 项、private_log 6 项、crate_boundaries 4 项、tool_identity 8 项、tools_verify CLI 15 项也通过；本轮普通相关用例合计 53 项。新增 CLI 集成把真实发布收据作为 managed_cache 锁候选核验，结果仅 matched_untrusted，不执行脚本、不授予 ready。

显式 `CODEGUARD_TEST_RUFF=/opt/anaconda3/bin/ruff cargo test -p codeguard-runtime --test tool_cache_install real_ruff --offline -q -- --ignored`：1 项通过。将本机 Ruff 原字节发布到临时缓存后，统一原生版本诊断核对前后 SHA-256 及精确 Ruff 0.16.8 输出。没有把默认 ignored 计为运行；也没有把本地原生字节来源当受批准发行证明。

all-target Clippy（-D warnings）、格式、OpenSpec strict/diff 检查通过。未增加产品依赖、下载安装工具或改动全局 PATH。全 workspace 测试、Windows 发布、网络下载、压缩包展开/目录闭包及宿主端到端未运行或尚未实现。5.3/5.6 保持未完成。
