# CFQuery嵌入区域结束边界修复

规格：`introduce-rust-codeguard-cli` SP06 / 14.4、14.6、14.19。只修正区域识别，不改变grammar字节、任务批准或原生义务。

## 问题与依据

原实现直接寻找首个`</cfquery>`，会把普通/嵌套服务器CFML注释中的文本当成结束标签；未闭合注释也会创建截短候选。路由三反例先RED（7通过/3失败），真实check all反事实恢复旧实现后也RED（多出一个不完整片段，3个查询而非2个）。最初公开测试使用未支持的`--timeout=60s`在参数校验失败，该失败未计作产品反例；改成现有`--timeout 60s`后才执行真实worker反事实。

依据为[Adobe官方CFML注释规范](https://helpx.adobe.com/coldfusion/developing-applications/the-cfml-programming-language/elements-of-cfml/comments.html)：服务器注释内容忽略且允许嵌套。实现只用已有嵌套注释边界跳过其中的伪结束标签，不删除注释，不遮盖/替换SQL原始字节，也不引入新的解析器。

## 验证

修复后直接路由10通过。真实公开check all在隔离WASM worker中处理两个完整查询；对中文/CRLF前缀逐片段核对源码SHA及原文件byte_offset，对恢复节点核对原范围。未闭合注释只保留整文件CFML候选。实际报告见[evidence](evidence/cfquery-comment-boundary-2026-10-05.json)，schema0.38；交付incomplete、grammar未获资格、原生六类别均保留gap。公开目标1通过/0忽略，11.63秒。

整文件CFML对嵌套注释样本仍产生两个恢复节点；未闭合注释的整文件grammar反而返回零恢复。两者原样保留，不作为正确性oracle或语法通过；此次只证明选择完整原始区域，未声称CFML/CFQuery原生精度已验收。独立版本/方言oracle、原生对照、完整级联归并、资源及发行仍缺，14.4/14.19不勾选。完整受影响回归及Clippy结果在后续追加。

最终受影响两目标回归：15通过/0忽略，117.97秒；10通过/0忽略，0.00秒。包含全部32候选在四个项目、同一混合项目以及逐文件保存Hook实际执行；未重复358例完整精度语料，不提升grammar资格。实际公开报告通过0.38封闭schema；OpenSpec strict、分层与定向格式通过。

最终WASM workspace/all-targets严格Clippy通过。未修改受保护Erlang差分草稿；新远端CI需要按当前提交独立验证。
