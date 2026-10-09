# C++17 成员文档工作台局部验收

对应introduce-rust-codeguard-cli的8.29、9.9、15.3、15.6。公开CLI覆盖init→comments cpp→同一文件稳定结构任务→原Clang task verify→修复观察→问题再现。没有授予可信关闭或生产资格。

实际Apple Clang21定向测试默认/WASM各1通过、0忽略（最终1.93/2.53秒）。方法缺注释与构造参数缺描述形成一个稳定任务、两个源码位置，允许路径仅api.cpp；重复扫描及问题再现保持同一任务ID。每次复检显式核对原c++17标准、选定Clang路径、任务身份、输入稳定与当前源码SHA。修复后为candidate_absent_unverified_policy，事实仍open；再现为still_present，不能称为已验收的可信关闭/重开。

既有C/C++结构工作台实际回归1通过、0忽略（4.49秒），无进展预算与历史证据防篡改保持原断言。

[默认证据](evidence/cpp17-member-workbench-default.json)、[WASM证据](evidence/cpp17-member-workbench-wasm.json)绑定测试/CLI/前后源码摘要，保存实际首轮反馈及三次复检。CLI摘要对应采集时的开发构建，不代表发行产物或新计划嵌入后的二进制。测试临时项目已清理，原路径是采集时上下文。

[协议检查](evidence/cpp17-member-workbench-schema.json)：8个实际报告通过既有Schema，16种伪造覆盖/资格/交付输入被拒绝，509个Schema元定义有效；源码身份/观察顺序/标准/任务ID也核对。验证脚本为tests/cpp_documentation_workbench_schema.py，仅用于验收数据，不是运行时检测器。

类契约、析构/运算符/模板完整文档、语义准确性、异常/行为、项目构建配置、真实智能体宿主与生产门禁仍未完成。
