# 固定grammar资产的进程内复用

对应14.13/14.19的不可变资产复用边界，不完成源码结果缓存父任务。

## RED与核验范围

实际内置Java资产首次及重复八次选择的核验计数先RED：9而非1；缓存成功核验项后为1。并发八个调用者选择第二个JavaScript资产也只增加一次核验，元数据/字节身份一致且Java/JavaScript不会串用。生产缓存只保存固定清单中成功核验的资产，未知/大小写/路径穿越标识不插入缓存。Mutex保护首次核验和发布，损坏锁返回明确错误，不猜测资产。

元数据来自include_bytes固定字节，通过OnceLock解析后返回独立克隆，修改调用方副本不污染共享清单。verify_grammar_asset仅复用该不可变元数据，不缓存调用方字节核验；缓存热身后篡改WASM、许可证或资产SHA仍拒绝。内置二进制更新或进程重启清空这层缓存，不触碰tracked事实、任务或决策。

Adapters lib与资产契约19 passed/0 failed；目标先RED后GREEN，并发增强后再通过。首轮严格Clippy拒绝复杂静态类型，改为明确内部类型别名，不关闭lint规则。CLI四目标45 passed / 0 failed / 2 ignored：全32份项目及编辑Hook回归12通过（81.71秒）、历史/当前语料边界14通过、库存2通过、实际worker资源/恢复17通过。与资产层合计64通过，不把ignored的两份全量回放计作本轮执行；当前缓存仅资产核验，未改解析算法或任何WASM字节，已有358例证据保留。OpenSpec strict、分层与diff检查通过；默认与WASM两种构建的adapters/CLI all-targets严格Clippy最终均通过；定向fmt、OpenSpec strict、分层及diff检查通过。Erlang预先草稿SHA256保持2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6。

不保存源码解析结论，不跳过当前输入/配置/原生工具核对，不将任何未验收候选标成clean。没有宣称冷暖启动性能预算、跨命令命中率、正式grammar资格或14.13完成。旧358例报告及制品资产原字节不变。
