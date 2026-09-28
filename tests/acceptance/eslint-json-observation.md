# ESLint 10 JSON 局部观察契约

本轮为纯 Rust 报告解析，不安装/执行 ESLint，也不执行项目 JS 配置。公开入口 parse_eslint_json 的具体预期版本与观察版本必须相同、属于稳定 10.x；这些值是调用方输入，不自证工具身份。当前原生范围/版本兼容尚未验收，不能把解析器存在当作语言能力已完成。

依据官方 formatter JSON 与 CLI exit/max-warnings 契约：
- https://eslint.org/docs/latest/use/formatters/#json
- https://eslint.org/docs/latest/use/command-line-interface#exit-codes

## 契约夹具与 TDD

API/类型缺失先编译失败，补齐后六项测试通过。新增 dot 路径反例因 Rust Path 自动折叠内部 ./ 失败，修正为原始段与组件双重核对。

七项目标用例覆盖：退出 0 的 warning 保留；max-warnings=0 使 warning 退出 1 的正确解释；普通 error 与退出 1；退出 2/非正常退出不能成为普通 lint 结果；fatal 或缺 ruleId 的消息保持调查未完成，保留同报告有效规则；suppressedMessages 保留数量、要求覆盖核查，不自成白名单或干净结果；冻结版本/文件集合、计数、重复字段/文件、缺/额外文件、空范围、损坏 JSON 与路径跳转均拒绝。

报告限制 16 MiB、显式范围最多 10,000 个文件、活动与抑制消息合计最多 100,000 项。消息与规则 ID 有长度限制；派生反序列化拒绝语义字段重复，原生 fix/suggestions/source/metadata 不转化为可执行指令。模型按对象拆文件，生产注释中文。

## 未完成边界

CLI 原生调用、工具/Node 包闭包、flat config 与 plugins/tsconfig 的有效范围、真实 JS/TS 正反例、monorepo、依赖审计、持久任务及受批准门禁未接通。没有本轮原生执行证据，不关闭 OpenSpec 7.3。未验证其它 ESLint major，公开消息呈现前仍须脱敏。
