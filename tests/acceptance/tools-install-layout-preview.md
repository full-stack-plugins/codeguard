# 安装预览 0.3 的完整布局阶段反馈

对应 OpenSpec 5.3/5.6，正式安装未完成。

当前 tools install 的 JSON/human 反馈升级为 0.3；0.2 schema 单独保留。发行清单静态绑定成功后，逐工具附 layout：declared_paths_bound_untrusted 表示完整树和路径声明与原锁关联；missing_full_tree_declaration 表示旧声明不足以形成完整布局；not_applicable_raw 表示 raw 不适用目录树布局。每类都有固定下一步，全部 content_verification=not_run。完整布局只显示树声明摘要、<摘要>.bundle 目录名及入口/可选 bundle 定位摘要，不回显包内私有路径或下载地址。

声明关联不读取下载包，不调用展开、内容关联或发布 API；dry-run 不写入，未获批准的 apply 保持 blocked_before_mutation、authority=unverified、readiness=unknown、gate_effect=none、退出 3。旧清单仍可观察但不猜目录。human 明示布局阶段和“包内容核验 not_run”，缺声明不伪造布局。

新增命令契约先因反馈仍为 0.2 失败；实现后 24 项命令、14 项清单、5 项包/布局、5 项 crate 边界，共 48 项通过。新增契约覆盖 dry-run/apply、私有包内路径脱敏、旧清单不足和 human 阶段反馈，既有 raw 用例补断言“不适用/not_run”。最终 schema 验证 8 份真实命令输出（无清单、完整声明、apply、旧声明、错路径、缺树、坏 bundle 映射、raw）；4 个伪造完成状态和 2 个格式/阶段矛盾被拒，旧 0.2 schema 拒绝 0.3。新错误码纳入枚举；schema 不证明清单来源可信。

all-target Clippy -D warnings、增强断言后的目标 Clippy 与格式检查通过。全 workspace、跨平台/宿主未运行。实际包读取/下载、批准来源、raw 定位、安装状态恢复及正式 apply 仍未接通，5.3/5.6 不勾选。
