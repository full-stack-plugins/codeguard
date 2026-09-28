# 签名网络下载与受控缓存发布

对应 OpenSpec 5.3，正式宿主及 tools install 尚未完成。

新增 download_and_publish_signed_distribution，明确接收原始签名/清单/锁、精确工具、本机平台及独立宿主信任/范围/时钟/网络许可。请求前核验签名和全部制品、清单 1.2、目标存在与精确 authority 许可。网络许可为规范 host:port，runtime 静态校验同时拒绝通配；不从环境代理/项目配置推导许可。下载沿现有 TLS/长度/摘要/手动重定向/共享预算执行，随后调用原签名发布服务重核时间、原始身份和包内容。没有制品执行、运行时安装或门禁结论；CLI apply 未新增项目自批开关。

测试先因 API 缺失编译失败；另在加入 Tokio dev dependency 后以 offline 更新锁，未安装全局运行时。前置契约核对错误签名、外国平台、错工具、缺初始站点许可、通配许可分别返回预期固定诊断且缓存为空。共享测试签名/清单工厂提取后原发布契约继续验证。

显式运行真实公共 HTTPS 测试，输入为 https://raw.githubusercontent.com/rust-lang/rust/a22b02eaecd6ac937d752139c79d0159c725932d/LICENSE-MIT 。固定提交的许可证文本作为非执行 raw 测试包；首次独立取得其字节并冻结大小/摘要，专属 test seed 签发清单/锁，生产网络客户端采用公共根 TLS，再精确下载并临时发布，磁盘内容逐字节相同，1 项通过，用时 0.70 秒。没有信任测试 CA、没有运行下载内容，临时发布目录回收。测试时钟/密钥为测试宿主输入，不是真实发行者批准或生产信任根。

包长度：1068；SHA-256：b71bd43a069ca0641a9ecfe585ca7b3c53b5cc1608f8b68321168698e28b5ea1

真实宿主信任库/撤销/防回滚持久状态及网络许可来源、正式 tools install 参数/宿主接线、运行时/恢复/全平台验收仍缺。此公共非执行文件不是原生工具发行包验收，未运行全 workspace；5.3 不勾选。

最终回归：网络前置 1、签名 8、发布 5、tools 预览 25、runtime HTTPS 15，共 54 项普通测试通过；另 1 项显式真实公共 HTTPS 下载发布通过。CLI/runtime all-target Clippy -D warnings、格式、OpenSpec 严格校验与插件 diff 检查通过。
