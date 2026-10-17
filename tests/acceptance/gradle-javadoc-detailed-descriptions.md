# Gradle Javadoc 空描述与用途说明验收

对应 `introduce-rust-codeguard-cli` 的 15.3 / 15.6；延续原生工具执行与公开原任务复检。此处补齐原生描述诊断，不授予 Java 或全语言文档生产资格。

## 发现的真实缺口

现有 JDK21 在 `-Xdoclint:missing` 下，分别对空注释、缺少主描述、裸参数/返回/异常标签发出 warning；退出码仍可为0。原 Gradle 适配器只映射了参数/返回空描述，另外三类被归为 `native_javadoc_output_unresolved`。结果没有假通过，但智能体得到环境准备任务，无法直接处理源码文档问题。

先运行原JDK五组样例，再让适配器单元测试与真实公开入口各自复现失败，随后补齐精确消息映射。固定英语JDK21消息、文件身份、源码行、caret及汇总数仍由原定位契约核验；未知消息或残缺块拒绝整组诊断，不猜测源码违规。原独立JDK/Maven解析器没有扩大，保留为明确待接入缺口。

| 已观察原生消息 | 原生规则身份 |
|---|---|
| `empty comment` | `JavadocEmptyComment` |
| `no main description` | `JavadocMissingMainDescription` |
| `no description for @param` | `JavadocEmptyParamDescription` |
| `no description for @return` | `JavadocEmptyReturnDescription` |
| `no description for @throws` | `JavadocEmptyThrowsDescription` |

## 实际执行路径

```mermaid
flowchart LR
    A[原Gradle官方Javadoc任务] --> B[固定格式的JDK21原生诊断]
    B --> C[规则身份、原源码行与定位核验]
    C --> D[0.2原生与工作台报告]
    D --> E[稳定源码任务与对话指引]
    E --> F[task verify 原范围原工具复检]
    F --> G[0.2复检与0.30任务预览]
    G --> H[保留开放事实和未受信观察]
```

原选定范围、Gradle/JDK、doclint/doclet/访问范围不被替换。添加原生消息的适配不会开启新原项目规则；它只把原任务已经报告的问题正确投影到源码任务。若原项目没有启用这些检查，零输出仍不能证明详细注释规则齐全。

## 已运行样例

已有 Gradle8.10.2、Microsoft JDK21.0.12；不安装工具。一个公开条件测试实际运行四次 `check java` 和四次原任务复检：

| 样例 | 首次诊断 | 原任务复检 | 修复后 |
|---|---:|---:|---|
| 类型、构造器、字段和方法均为空注释 | 4条空注释 | 仍有4条、事件已记录 | 0条，候选消失但事实仍open |
| 参数、返回、异常标签均无描述 | 3条、三种独立规则 | 仍有3条、事件已记录 | 本批未单独运行该任务修复 |
| 只有有内容的参数/返回标签，没有用途说明 | 1条缺主描述 | 仍有1条、事件已记录 | 本批未单独运行该任务修复 |
| 中文完整类型/构造器/字段/方法说明，适用标签含描述，合法`inheritDoc` | 0条 | 无源码任务需要复检 | 仍为未受信空输出 |

这不是独立误报语料，也不据四个受控样例声称全部Java文档覆盖。原JDK直接执行的五组观察与公开Gradle报告分别保存，不能把原JDK探测称为公开组件验收。见[原生证据](evidence/gradle-javadoc-detailed-descriptions-native-2026-10-06.json)，包含实际执行的CodeGuard二进制SHA-256及执行结束的一致性核验。

## 协议与兼容

新增原生/工作台/复检0.2、聚合0.67、异常反馈0.18、修复预览0.24和任务预览0.30。历史0.1及上一轮公开协议不修改。历史0.1 native报告不能携带新规则；新消费者仍消费已绑定的旧问题，并使用当前原工具复检。复检容器、内层工作台与native版本一致，不能伪装为旧协议。

默认CLI六目标48 passed、0 failed、7条工具条件测试忽略；WASM四目标24 passed、0 failed、7条条件忽略，两种构建重叠用例不相加。适配器三个目标12 passed、0 failed、2条Maven条件忽略，保留独立JDK/Maven旧行为回归。真实公开条件测试另1 passed，含上述八次质量检查；未执行的条件测试不算通过。异常反馈两项单元通过；[新版兄弟失败反馈](evidence/gradle-javadoc-detailed-descriptions-aborted-2026-10-06.json)是构造协议反例，不声称执行原生工具。默认/WASM严格Clippy、分层、OpenSpec strict、定向格式与差异检查通过。

430份协议元定义、28份实际公开/原生/任务报告与1份构造异常反馈（合计29）、8种伪造及4个旧消费者拒绝验证见[校验记录](evidence/gradle-javadoc-detailed-descriptions-schema-2026-10-06.json)；保留全部423份旧schema字节。未知规则、版本、checker、定位、完整覆盖、权威升级和附加字段反例必须被拒绝。首次schema验证还发现嵌套修复指引版本未升级，修正新0.24契约后重跑，不把第一次失败计为通过。

## 仍待完成

独立JDK/Maven新增规则接入、Checkstyle描述/摘要模块、所有源集/模块/doclet和JDK格式、适用行为契约与业务语义充分性、可信关闭和复发重开，以及逐语言文档规范与独立精度/平台/宿主/性能/发布验收仍未完成。静态检查不能凭文字长度证明业务契约真实充分。父任务不勾选，语法资格保持0/32。
