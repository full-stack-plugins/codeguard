# JavaScript 重复直接绑定的有界AST事实与原生对照

日期：2026-10-06。对应syntax-precheck / Duplicate binding facts SHALL preserve lexical scope and parser evidence，关联14.4、14.17、14.19，父任务保持未完成。

## 实现与原生依据

已知固定JavaScript grammar对`const x=1; const x=2;`没有ERROR/MISSING，Node24.18.0会拒绝该源码。新增Rust runtime扫描器读取调用方明确指定的根/声明/声明项/简单名称四种节点类型；当前验证配置为program/lexical_declaration/variable_declarator/identifier。仅比较该根的直接声明名称字节，返回第二次及后续声明的位置，不输出名称或伪造解析器恢复。

新测试实际RED为缺少scan_wasm_sibling_bindings API；实现后普通三项测试通过。显式已安装Node24.18.0的测试实际执行module和CommonJS各八例，共16次语法检查；版本探测与前后入口字节核对通过。三类重复let/const及Unicode/CRLF情形均被原生拒绝；嵌套块/函数、var重复及字符串/注释不形成重复直接绑定事实。`return 1`原生module拒绝而CommonJS接受，扫描器不据此猜测模块模式或产生重复绑定。显式四项测试4通过/0失败/0忽略，0.83秒。

源码和节点类型有界，访问与记录预算耗尽保持truncated并保留已获得事实；零事实不证明所有语法完整。解构、export包装、var/lexical冲突、转义归一化、其它作用域及版本/方言尚不在此简单事实范围。该扫描器是通用AST事实基础设施，语言策略必须由adapter/CLI单独绑定。

## 尚未接线

本批**没有**将该事实接入公开WASM worker、grammar probe、check/lint、Hook或修复任务。因此既有JavaScript原始及组合差分报告仍保留两项FN，不能声称用户命令已检出重复绑定；32语言资格仍0/32。下一步须完成独立规则包、版本化生产者/消费者与schema、稳定原生确认任务、真实Node差分及公开入口验收，保留原始parser分类，不借结构层改写旧报告。

复跑普通事实：`cargo test --offline -p codeguard-runtime --features wasm-precheck --test wasm_sibling_binding_scan`。实际工具须显式选择：`CODEGUARD_NODE_BIN=/absolute/node24.18.0 cargo test --offline -p codeguard-runtime --features wasm-precheck --test wasm_sibling_binding_scan -- --include-ignored`。未安装/升级工具、改变grammar字节或发布制品；Erlang草稿不执行、不提交。

最终四个runtime WASM目标18通过/0失败/1条件忽略（重复绑定、根子节点、空块、恢复扫描）；真实Node项已另行显式执行，不与重叠普通测试累加。默认/WASM runtime全目标严格Clippy、所改文件定向rustfmt、OpenSpec strict、crate分层与diff检查均通过。未执行完整workspace或358例重放，不用此基础设施声称公开路径漏检已修复。
