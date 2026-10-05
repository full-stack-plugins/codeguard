# Go完整文件package结构规则底层验收

日期：2026-10-05。对应现有OpenSpec 12.11、14.17、14.19；父任务未完成。

## 问题、实现与权限边界

同一固定grammar将`x := 1`及`func f() {}`按片段接纳，没有ERROR/MISSING；原生整文件检查拒绝。新增`scan_wasm_root_child`运行时事实接口，以游标线性遍历根的直接命名子节点，不搜索源码文本、不深入嵌套节点。记录root/child种类、present和truncated。预算1至200000，非法参数拒绝；预算耗尽即使已经看到声明也保留truncated，不能声称遍历完整。

适配层`missing_go_package_candidate`核对固定规则语言go、whole_file范围、source_file根、package_clause子节点及非截断状态。相同source_file根的合法Rust不会误套Go规则。规则版本1.0.0，摘要`d8af5cba71012b59615fbda509d88e7f2503ed51bd3611bb8a9498467f562bd3`，资格candidate_unqualified。Some(false)只表示这条规则没有候选，None表示未知；两者均不授予源码合法、原生lint、任务关闭或门禁权威。

## 本地证据

新API测试先因接口不存在RED，再实现最小有界观察及适配器。运行时4测试通过，规则解释1测试通过；CLI组合回归验证原20例源码不改、原始漏检仍为2、独立结构层可补获2。CLI目标6测试通过/1条件忽略，其中5项复用已有Go观察器测试；不将它们计为6个新功能。真实SDK测试需显式选择已安装工具，不安装或下载任何工具。

```bash
cargo test --locked -p codeguard-runtime --features wasm-precheck --test wasm_root_child_scan
cargo test --locked -p codeguard-adapters --test go_package_rule
cargo test --locked -p codeguard-cli --features wasm-precheck --test go_package_structure
CODEGUARD_GO_BIN=/usr/local/go/bin/go cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test go_package_structure \
  pinned_go_structure_matches_fresh_native_whole_file_observations \
  -- --ignored --exact --nocapture
```

显式真实测试于本轮执行通过（31次原生源码观察，部分专项回归与原样本重复，4.42秒）：复用[原20例冻结源码](evidence/go-native-grammar-input-2026-10-05.json)，组合7TP/0FP/0FN/12TN，逻辑line位置重映射1例仍unknown，未删除样本；原始grammar5TP/0FP/2FN/12TN不变。另11例专门复核缺声明语句/函数、注释/字符串中的package、只有注释、空文件、仅package、build标签、Unicode包名及CRLF，全部与原生结果一致。测试只读stdin，原生工具不解析导入或执行用户源码。测试最初误将diagnostics_observed当作未完成；核对既有原生契约后更正测试分类，生产观察器未改。

原生版本go1.23.4，主制品摘要`5900a8e0942f88b7bacf2b44b397b4950c8dc9f850a16ae1249e1636860dd6ef`，同SDK辅助绑定摘要`19436fb5bd7825f4450058b0704166c843979e04dddada5174d14710174fe7e4`；各完整原生观察及批次结束继续核对该绑定。Go grammar摘要`4eda5d91c99ca981e88bc7d3d33f0db166b4bab0a84d0021a9abf39b364c78ef`，ABI14。现有清单和历史原生报告均不改。

受影响运行时回归13测试通过（空block4、原始恢复5、根节点4）；crate边界7和依赖MSRV核查2通过。默认/WASM两配置workspace/all-targets严格Clippy通过，strict OpenSpec、分层与diff检查通过。新增文件定向格式通过；adapters/lib.rs已有格式差异与HEAD基线逐字核对相同，未扩大格式修改。没有执行本提交的完整default工作区测试、32grammar全量回放或发行验收；先前提交的远端CI仍在运行，不能据此声称本提交完整CI已通过。

## 未完成内容

此底层模块未接入私有语法worker和公开grammar probe、check all、任务确认、智能体反馈或差分协议。不能以本测试宣称公开Go漏检已修复。接线必须保留完整文件范围、规则身份、未知预算、原始/组合统计分离，版本化受影响报告，并完成公开反馈、任务复检及独立误报验收。完整项目lint、类型/CVE/依赖检查、holdout、跨平台限制和发行资格均未由此证明；32份grammar资格仍为0。
