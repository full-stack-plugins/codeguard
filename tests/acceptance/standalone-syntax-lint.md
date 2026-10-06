# 注册表语言的独立候选 lint 入口

沿用 `introduce-rust-codeguard-cli` 2.1/2.7、8.x、14.6/14.9/14.10；不关闭完整父任务。现有十一类原生/共享生态入口保持原分派，其余注册表规范ID进入统一单文件服务，未登记别名仍拒绝。没有专用原生适配不等于工具未安装，配置状态固定unknown，不直接执行旧注册表的lint数组，也不自动下载/安装。

启用WASM时，仅对明确匹配的路由复用项目有界扫描。19个单文件请求实际调用剩余20份grammar：CFML请求分别覆盖CFML+CFQuery和CFScript；其他为ArkTS/C/C++/C#/Dart/COBOL/Lua/Luau/Nix/Objective-C/Pascal/PHP/R/Scala/Solidity/Terraform/VB.NET。已有原生入口覆盖的grammar不被此服务替换。此证据证明单文件候选执行，不证明任何新增语言原生lint已接入或精度合格。

Dart疑似样本验证重复扫描和限定项目检查复用同一任务；后续合法候选仍保留原生确认及同ID。删除任务投影反馈未完成，不因零候选隐藏记录。历史恢复及有界终端位置显示从JavaScript入口提取为共享函数，最多显示四条观察，各条恢复及结构位置分别最多八项，不输出源码片段。

未知ID、混写重复选项、无关工具参数、坏格式在扫描/持久化前拒绝。语言与文件不匹配、歧义头文件、工作区外文件及符号链接不强选grammar，不建立伪任务。无WASM构建仍返回封闭版本化未完成报告及明确编译能力缺口。原生确认始终required，因为适配准备尚未确定；不会以未验收grammar的零候选取代原生lint。

报告为独立 `syntax_lint_feedback` 0.1，复用现有syntax_candidates及syntax_tasks协议，不改历史schema字节。失败保留原因；未初始化工作区提供init原工作区的明确动作，记录损坏要求恢复受管身份及历史，不要求修改无关源码。所有示例来自本轮实际CLI报告，见evidence/standalone-syntax-lint-2026-10-06.json。

WASM四组受影响测试31 passed/0 failed/0 ignored；新增项目身份、工作区外及链接反例后，本目标最终5 passed（与31项重叠，不累加）。默认四目标14 passed/0 failed/4 ignored；被忽略的真实工具条件不作为本轮原生验收。全32候选项目路由目标包含在31项中，未执行358语料全量精度评测。真实宿主自动触发、可信原生关闭、完整项目lint/语言资格与发行仍未完成。受保护Erlang草稿保持原字节，未执行/提交。

28份实际反馈通过封闭schema；20种新增候选语言集合明确核对，6份伪造通过/配置缺失/降级推荐变体拒绝。367份schema元定义、默认/WASM CLI全目标Clippy -D warnings、OpenSpec strict、分层和diff检查通过。help语言清单与57项注册表逐项对照，保留partial而非原生全覆盖声明。
