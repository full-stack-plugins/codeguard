# Erlang 直接函数 form 终止符候选

对应现有 introduce-rust-codeguard-cli S12.11、S14.4/14.17/14.19。固定 grammar 字节不变，不重新编译或提升语言资格。

实测固定 WASM 将缺句点的 `f() -> ok` 解析为无ERROR/MISSING的 `fun_decl`。合法多子句的中间form同样是独立fun_decl，但末尾直接token为分号。新增Rust有界AST扫描仅观察直接函数form的直接token：句点结束；分号在后续同类form前保留为可能的续接，EOF或其它form前为候选；没有直接终止token则锚定form末尾。字符串、字符、浮点数、quoted atom与注释的内容不参与标点匹配。

本规则只观察终止符，未判定续接子句的函数名/参数个数等语义，也不代替OTP的语法或项目检查。遍历根子节点、所有直接token均计费；超限不能将未访问的下一子句当EOF，记录超限保留incomplete。最多128条锚点和200000次访问。候选与原始ERROR/MISSING分开存储，并由父进程核对语言、规则摘要、源码位置和固定grammar身份。

实际首次测试因临时路径的系统symlink被拒绝；改为canonical临时根后，公开RED准确落在 missing_period_eof 零候选。新规则接入后同一测试通过：24个原有样例，13合法均无终止符候选，9种终止符非法各产生候选，另2个bare form仍保留原始解析错误。没有执行或改变未提交的erlang_native_differential.rs草稿；当前测试来源为独立已提交的 erlang_source_forms.json。

接线范围：显式 `grammar probe erlang FILE --format=json` 和私有 `--form-terminators`。新增worker1.6/probe0.7仅在出现结构观察时输出，旧普通零结构协议及已有消费者保持原行为。固定rulepack引用见rulepacks/erlang/form_terminator.json。24份实际反馈、六种矛盾变体与369份schema元定义已校验；证据见evidence/erlang-form-candidates-2026-10-06.json。

项目/check all、编辑Hook、稳定任务导入及原生差分的组合指标尚未接线，旧358例raw parser回放及十个Erlang漏检的历史记录不修改。不得以显式probe候选宣布全量漏检已消除、原生工具执行、任务关闭、宿主或发行完成。grammar qualification仍为0/32；父任务继续开放。

终态验证：runtime两目标6 passed/0 failed/0 ignored，受影响CLI五目标40 passed/0 failed/1 ignored（该Go原生条件未执行，不计为本轮原生证据），显式Erlang样例目标另1 passed。WASM三个crate全目标Clippy -D warnings通过；所改文件格式、分层和OpenSpec strict通过。未执行完整默认workspace或358例全量回放。

默认三个crate全目标Clippy -D warnings也已终态通过。公共测试的bare表达式恢复另以明确断言保护；与此前同一Erlang样例目标重叠，不重复累计。
