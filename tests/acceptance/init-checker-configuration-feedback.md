# init 原生检查配置反馈

范围：OpenSpec 9.24/9.25 的只读配置反馈切片，不代表完整准备任务或 readiness 完成。

init_plan 0.5 的 profile_summary.checkers 使用既有静态发现结果，按 checker/构建根反馈 configured/missing/invalid/unknown、来源、原因与下一步。每条 execution=not_run、required_by_policy=null、gate_effect=none，清单明确 partial。human 从相同结构投影并沿用不可信文本转义。缺配置不产生源码 finding，声明配置不证明工具可用、原生执行或批准义务已绑定；未知继承不猜测为 missing，不自动启用/忽略检查器。历史 init_plan 0.4 schema 单独保留。

新增配置反馈用例先因旧输出 0.4 无检查配置反馈而失败。实现后真实临时项目覆盖四个 Maven 构建根：直接 Javadoc 配置、无配置、坏 XML、外部父继承；JSON/human 同状态/原因/下一步，均未执行。Python 无 Ruff 配置保持 missing/not_run，源码原字节不变且没有 finding，dry-run 无新增 codeguard/ 或 target。

真实 CLI 四状态输出通过 JSON Schema，6 个伪造配置通过/执行完成/必需策略/门禁效果/完整清单/缺下一步反例拒绝，历史 0.4 正例有效。schema 验证辅助不是 Python 产品实现。

受影响 CLI 契约 77 项（init 39、detect 20、check-plan/config 各 6、边界/plan 各 3）与 CLI 库 18 项通过。workspace all-target Clippy（-D warnings）、格式、OpenSpec strict/diff 检查终态通过。未运行完整 workspace/全语言原生检查/宿主端到端，9.24/9.25 保持未完成。
