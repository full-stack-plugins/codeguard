# Grammar语料的历史清单与当前执行身份

对应12.11、14.17、14.19。GitHub CI [37257302720](https://github.com/full-stack-plugins/codeguard/actions/runs/37257302720) 在5个语料/历史回放测试上因grammar_evaluation_corpus_identity_invalid失败。1c55670为Python新增已知限制，改变清单摘要；原语料固定摘要86547d2e5c038140a392f670a820335e3e7a7d251d75e9f958c30a9932fc1a4d仍正确绑定旧清单，不应改写旧报告或直接放宽当前执行。

将1c55670父提交的完整清单原字节保存至tests/fixtures/grammar_manifests/manifest_2026_10_04.json，SHA256与两份历史语料一致。validate_corpus_against_manifest只核对调用方提供的有界清单及语料，不授予来源信任、资格或执行许可。manifest重复字段、重复语言、超长、空范围及摘要失配拒绝；validate_corpus与replay_corpus仍只接受当前固定清单。

历史测试使用原清单，当前回放夹具先验证旧语料，再显式生成只更换清单身份的新语料字节；全部case源码、来源、标签保留，当前报告使用新语料摘要。冻结历史JSON不变。旧语料即便历史校验成功，直接调用当前回放仍在进程启动前拒绝。

首轮修复普通WASM目标11通过/0失败/2忽略，55.00秒；新增边界后13通过/0失败/2忽略，53.63秒。WASM严格Clippy、OpenSpec strict、定向fmt及diff检查通过；完整358例回放显式执行1通过/0失败，444.20秒：32语种/35来源组全部实际attempted，程序稳定；73 TP / 1 FP / 10 FN / 269 TN、3 unknown、2 pending保持。新输入摘要c4b548a38ce5b169f3f106a6ef219a19ad2b17c4c2498b933604e1242a87ad9b绑定当前清单a992c0975b49dc6ee811888be599a819deb5d106b762722417cfefec4d84c76e。新输入、清单快照及[原始报告](evidence/grammar-current-manifest-2026-10-05.json)独立保存，报告通过Draft202012且新输入摘要独立重算匹配。报告SHA256 df648d8af989068348d8ec2f7ca638cff10d0e9fe727a5a6d3a369a840064048。默认9通过、默认/WASM严格Clippy均通过；新增归档契约1通过（0.06秒），新语料也通过Draft202012；新增契约后默认/WASM严格Clippy再次通过。CI仍需当前提交终态，不将旧commit结果借用。历史清单是核对材料，不表示这些候选已发行或通过语言精度。
