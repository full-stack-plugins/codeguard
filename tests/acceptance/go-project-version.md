# Go 固定语法 SDK 的项目版本边界

日期：2026-10-06。对应既有 OpenSpec 5.4、9.3/9.9、11.17、14.6/14.7/14.10 和 hook-protocol；不新建 change，不提前关闭全生态父任务。

## 问题与实现

版本不匹配的公开反例先失败：go.mod 要求 Go1.24，编辑事件仍调用固定1.23.4。新增共享 go_project_version 静态观察，编辑扫描和原工具 task verify 在执行 SDK 前检查源码最近 go.mod 与最近 go.work；更近声明遮蔽对应类型的祖先声明，两种文件各自独立寻找。

go 声明按最低版本比较，toolchain 是建议工具链；旧版本最低要求不因不等于固定SDK而阻塞，toolchain default 不要求下载。比1.23.4新的最低版本或建议工具链分别返回环境原因。重复/损坏/预发行或不支持注释形式不猜测，链接和超预算声明拒绝。读取每份文件上限256KiB、祖先64层，不执行 go env/mod、脚本、构建或安装。语义依据 [Go 官方模块参考](https://go.dev/doc/modules/gomod-ref) 和 [工具链参考](https://go.dev/doc/toolchain)，查阅于2026-10-06。

检查前后复核声明字节和更近缺项，新增或变化会撤回原生诊断；保存后的声明变得不适用，next撤回旧位置。选定SDK不适用不降级WASM，任务仍开放；无声明仍只是局部未批准语法观察，不证明项目通过。历史报告与schema不改，既有封闭reason协议承载新原因。

## 验证

新增四项公开回归覆盖不调用错误版本SDK的执行标记、嵌套模块/工作区遮蔽、重复/预发行/链接声明、运行中声明变化与保存后next及repair_ready纠偏。最终默认编辑九项通过；WASM六个受影响目标48通过/0失败/2条件忽略，版本解析单元两构建各一项通过。默认与WASM workspace/all-targets严格Clippy、最终WASM二进制构建通过。显式本机Go1.23.4四份实际报告分别验证可兼容最低要求、新最低要求、新建议工具链及重复声明；封闭schema及反例由 tests/go_hook_feedback_schema.py 核对，原实际报告不改。新证据为 [实际项目版本报告](evidence/go-project-version-2026-10-06.json)。

本机日志为 `/private/tmp/codeguard-go-project-version-{red,green,unit,wasm-unit,wasm,clippy-default,clippy-wasm,build}.log`。red记录原不调用约束失败；补充测试时曾缺少必需timeout和误读Hook摘要，修正测试调用及改由next检查具体原因后，最终九项通过。没有重跑全量workspace、32grammar完整精度回放或真实宿主会话，本批不借旧1461通过作当前全量证明。

## 未完成

此边界仅保护固定1.23.4语法候选，未实现通用工具链自动选择、GOWORK/GOENV外部配置、版本限定语言语义、build tags、项目vet调度或完整Go能力资格。开发差分仍直接观察固定冻结样本，不用项目声明改变历史oracle；SDK关闭服务的原反例与项目声明绑定验收继续独立。Rust编辑原生快检尚未接线，项目Clippy不能直接等同无构建的单文件编辑检查。
