# 版本探测期间的源码副本与输出连续性

日期：2026-10-05；对应OpenSpec 3.7、5.4、12.11、14.17、14.19的局部验收。

## 两个已复现的问题

Kotlin版本动作在私有cwd改写Sample.kt后，观察器只检查launcher，继续把已变化副本交给编译器，直到编译结束才撤回结果。Zig版本退出0且stdout符合0.16.0时，没有拒绝stderr，继续执行AST检查。两个实际调用标记反例先RED，证明不是仅报告格式差异。

Kotlin现在于编译前后使用同一个有界普通文件读取检查，核对私有副本与冻结原始源码字节。Zig要求版本动作stderr为空，异常版本输出使用已有version_unverified/incomplete结果。二者均在后继调用前停止，不将工具/输入问题变成源码finding，不授予修复或关闭权威。真实工具版本与正常输出的兼容性另行重跑，不套默认工具或安装新工具。

两项新反例GREEN；WASM CLI单元68通过、0失败、3条件忽略，4.75秒。六语言共享取消及入口变化反例仍通过。临时反事实代码不在工作树；受保护Erlang草稿保持原摘要且不提交。

## 规格归属校正

前几轮新增的三条原生差分取消/版本入口场景误附在Specific grammar limitations要求末尾；本轮仅移动到既有原生差分场景区域，场景内容与验收不变。两项新输入/输出场景归属于Native evidence per tool contract，继续使用同一个OpenSpec change，不增加平行计划或修改核心范围。

## 真实与受控证据

对固定同字节语料中Zig12例、Kotlin14例执行当前二进制和显式已安装工具对照，独立保存26例报告，不覆盖此前六工具114例。两例Kotlin grammar未知须保留，不算通过；此范围不覆盖Erlang、Python、JavaScript的14处历史原始漏检，也不授予32 grammar资格。[26例真实报告](evidence/native-version-input-zig-kotlin-2026-10-05.json)为0.1协议（没有选择Python/JavaScript层）；Zig4TP/0FP/0FN/8TN，Kotlin4TP/0FP/0FN/8TN、2例WASM未知，26样本仅24例可比较。程序与工具身份稳定，fixture/native未产生分歧；与114例报告中的同一26例源码、grammar、原生/解析器分类和比较逐项一致。原114例报告字节保留，不从这个子集消除其它语种的14处原始漏检。协议1项通过，核对库存、分母、身份、语料摘要与旧报告的相同样本。

```bash
cargo test --locked -p codeguard-cli --features wasm-precheck --lib
cargo build --locked -p codeguard-cli --features wasm-precheck \
  --bin codeguard --example evaluate_native_grammars
target/debug/examples/evaluate_native_grammars \
  "$PWD/target/debug/codeguard" \
  tests/acceptance/evidence/native-differential-six-language-entry-input-2026-10-05.json \
  300 zig=/opt/homebrew/bin/zig kotlin=/opt/homebrew/bin/kotlinc
```

此前cbac625的CI37278807557已success，两个job均完成；它只证明此前提交，不完成本轮新代码或任何仍缺的全语言、源集、工具链闭包、TOCTOU、独立holdout和发布验收。

最终六个受影响lint/任务服务/聚合入口目标40通过、0失败、3条件忽略；忽略项不算实际工具验收。默认/WASM全目标严格Clippy、定向rustfmt、OpenSpec strict、分层与diff检查通过。未重跑完整默认工作区或358语料，父任务保持开放。报告SHA-256 `2af7ebc38dfaf61ba5019b0031ca6aa14f38a8b13700cd43f3862be929537e79`。
