# 有界 HTTPS 包传输验收

对应 OpenSpec 5.3，正式安装仍未完成。

runtime 的 download_verified_package 采用 Rust 异步 HTTPS 客户端，验证公共根证书，不采用环境代理、自动重试或自动内容解压。冻结请求限定 HTTPS、初始无查询/凭据/片段、1..128 MiB、非零 SHA-256；最多五次手动重定向，精确 host:port 授权，禁止降级/凭据/歧义位置。CDN 重定向查询不进入固定诊断。整轮连接、响应及流读取继承绝对期限和取消信号；响应须为 200、未编码或 identity 内容，仅支持单个 chunked 传输框架，拒绝 Content-Length 混用和其它传输编码。返回前精确核对实际包长度/摘要，错误无半成品、不写缓存，也不授予发行或安装权威。

先前缺 API 的测试编译失败；实现后本地 TLS 正例暴露 macOS 接受连接继承非阻塞模式的测试服务问题，服务显式切为阻塞并带读写期限后通过。加强反例，要求请求确实到达且诊断与预期一致。新增 gzip, chunked 反例先意外得到 tool 成功字节，再增加传输编码校验使其拒绝。

11 项实际本地 TLS 用例覆盖精确正常字节、chunked/关闭分隔、URL/预算前拒绝、非信任证书、状态/大小/摘要/编码/截断、跨 authority 拒绝与允许、降级/凭据/歧义 Location、五跳限制、阻塞等待取消/到期，以及带查询的已允许重定向目标不能重置期限。测试使用专属测试 CA 和 cfg(test) 私有客户端；生产入口没有根证书替换或关闭验证选项。不是远端真实发行源验收。

相关回归：runtime 基础 4、HTTPS 11、归档 19、包流 9、目录发布 6、raw 缓存 10；CLI 包/布局 5、清单 14、raw 4、tools 预览 25，共 107 个不同测试通过。两个真实 Ruff 用例本轮 ignored，不计已执行；未运行全 workspace。新增网络依赖锁定；本机缓存的 154 个 registry manifest 无声明高于 Rust 1.85 的 rust-version，14 个非当前平台/wasm 包未缓存、未核验，实际 1.85 编译与跨平台运行未执行。

来源批准、受保护下载授权、清单→传输→展开/发布正式接线、运行时安装、恢复、宿主及其它平台仍缺，5.3 保持未完成。普通 check/plan/doctor 没有调用此下载接口，install apply 仍写前阻塞。未来调用方必须独立验证可信发行源和网络许可，不能以请求摘要相符推导授权。

最终 runtime/CLI all-target Clippy -D warnings、cargo fmt --check、OpenSpec 严格校验与插件 git diff --check 均通过。
