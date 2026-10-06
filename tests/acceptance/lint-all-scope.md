# lint all 多语言类别范围验收

对应 introduce-rust-codeguard-cli 2.1/2.3/7.x。原公开 lint all 被路由到 Python 专用入口，混合项目不能复用多语言调度，--jobs 被拒绝；目标测试先 RED。现在复用项目检查引擎，仅调度 lint 候选与原生节点，不创建独立构建、注释、依赖或 CVE 任务。保留共享预算、源码复核、WASM 有界候选和稳定任务。正常反馈为 check_feedback 0.59.0，requested_categories 固定 lint；历史非 lint 简报不作为本次 next，历史事实不删除。

七个默认回归目标86通过、0失败、15条件忽略；WASM 构建下 lint_all_scope 四项通过，属于相同验收行为，不重复累计。混合 Python/Rust 缺工具、SQL 能力缺口、专用 CVE 参数拒绝、受控 Cargo 调用范围均有公开 CLI 断言。Cargo 夹具记录只出现版本探测和 Clippy；初版夹具缺 Cargo.lock 导致原生前置阻塞，补齐夹具后通过，产品前置没有放宽。受控工具不是实际原生准确率验收。

实际混合报告通过新 schema；四种伪造类别、任务 ID 与交付状态被拒绝。CLI 全目标严格 Clippy、crate 分层、OpenSpec strict、diff 检查通过。旧 schema 保留。中英文 README、架构和技术文档与帮助增加统一入口及执行路径，示例明确为字段节选。

完整义务账本、全语言原生覆盖、真实宿主、跨平台及发行仍未完成，父任务保持开放。内部异常仍沿用 check_aborted，本批未独立验收 lint 模式取消/内部异常。用户 Erlang 草稿没有执行或修改。最新已提交 a68916d 的远端 CI 主门禁仍失败于插件证据提交 checkout；不能将该结果当本批源码通过。
