# 本地 HTTPS 契约证书

root.der 为测试 CA，cert.der 为含 localhost/127.0.0.1 SAN 的签发证书，key.der 为该叶证书的固定 PKCS8 测试私钥。仅 cfg(test) 服务及私有测试客户端使用；生产下载入口没有自定义根证书或关闭验证的参数，也不信任这个 CA。

证书有效期为 2026-09-26 至 2036-09-23，过期时须重新生成 CA 与叶证书，并复核 SAN、serverAuth、CA:FALSE、PKCS8 密钥和正反例。不能通过禁用 TLS 验证修复证书过期。服务使用独立线程及读写期限，退出时关闭并回收。
