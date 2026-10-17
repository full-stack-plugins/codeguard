# Go 编辑原生优先与原 SDK 修复任务局部验收

日期：2026-10-06。对应 OpenSpec 9.3/9.9、11.17、14.6/14.7/14.9/14.10、hook-protocol；完整父任务保持开放。

## 实现和执行边界

三个公开反例因缺少Go编辑接线/首次任务而RED。确认编辑现按实际选中文件调用既有Go1.23.4整文件探针，同SDK gofmt只读冻结stdin。SDK版本、两个制品、辅助入口绑定和源码字节均复核；不运行源码、项目vet、依赖安装或构建。选定工具失败/辅助缺失不改走WASM；缺工具保留候选，疑似或未完成要求原生确认，完整零恢复只推荐工具。`//line`重映射不猜物理位置。

原生首次观察0.10复用语法候选的稳定工作区/路径/语言身份；已有任务的干净扫描保存观察，不关闭。`repair_ready --go-tool`通过现有原SDK任务复检记录事件；未批准零诊断为candidate_absent_unverified_policy。next和Claude格式反馈仅使用当前规则、UTF-8字节列、真实任务与复检指引，不输出源码/自由文本。Hook外层0.22、内层0.12，历史schema留存；0.22允许无Shell扫描的空字段，不修改0.21。

首次原生观察的next曾误用仅允许复检引用的0.14，协议测试明确失败后，新增 `repair-brief-preview` 0.18接受Go首次观察的 `syntax-confirm-` 引用；原实际任务复检仍使用0.14及 `syntax-native-` 引用，历史schema不修改。

## 验证证据

受控 `go_native_hook` 五项通过：原生优先/重复任务/修复前后开放状态、选定失败、缺工具回退、Claude安全简报、辅助工具和逻辑位置阻塞。最后协议修正后，WASM受影响十目标78通过、0失败、5条件忽略；默认三个目标15通过、0失败、0忽略。此前首次Go目标三项GREEN后增加后两项，并非把重复运行相加。

本机 `/usr/local/go/bin/go` 实测Go1.23.4 darwin/arm64；同目录gofmt实际产生两个物理字节位置，修复后零诊断。源码和辅助身份分别保存在原始报告，未执行用户项目源码。九份实际记录位于 `evidence/go-hook-2026-10-06-*.json`，涵盖编辑、重复、修复前后、干净编辑、缺工具、选定失败、原生首次报告和Claude形状。实际Hook报告、Go扫描及原生首次报告通过封闭schema；Python协议回归3项通过并拒绝unknown version、allow、批准字段、错误语言/规则/列单位。

默认/WASM严格workspace/all-targets Clippy通过。此前默认全workspace/all-targets265目标1461通过、0失败、127忽略，是末次next协议修正之前的基线；末次修正以以上受影响回归为当前源码证据，不冒称重新完成全量验证。最终WASM二进制构建后，离线npm安装后工作流1通过、0失败、0跳过，约22.29秒，覆盖next、编辑、Claude、原SDK复检、再次出现和失败写入；九份安装后响应保存在 `evidence/go-npm-repair-2026-10-06.json`，八份结构化响应通过现有/新增schema，Claude格式另核对安全摘要。最终定向日志为 `/private/tmp/codeguard-go-hook-next-final-{default,wasm}.log`；Clippy、构建及npm日志为 `/private/tmp/codeguard-go-hook-{clippy-default-final,clippy-wasm-final,build-final,npm-final}.log`。不借前一源码的1452通过作当前证明。

## 未完成范围

这不是项目级lint、所有Go版本/平台/build tags资格、可信关闭/复发、真实已安装宿主验收或公开发行。固定SDK只证明该版本整文件语法，项目声明版本仍需核对。完整Go vet编辑调度和Rust编辑原生优先继续开放；32份grammar清单/字节/资格与既有精度指标没有修改。用户Erlang草稿不修改、不执行、不提交，散列仍为 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`。
