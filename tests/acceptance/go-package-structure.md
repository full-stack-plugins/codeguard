# Go完整文件package结构规则与公开链路验收

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

受影响运行时回归13测试通过（空block4、原始恢复5、根节点4）；crate边界7和依赖MSRV核查2通过。默认/WASM两配置workspace/all-targets严格Clippy通过，strict OpenSpec、分层与diff检查通过。新增文件定向格式通过；adapters/lib.rs已有格式差异与HEAD基线逐字核对相同，未扩大格式修改。没有执行本提交的完整default工作区测试、32grammar全量回放或发行验收；该底层验收时先前提交的远端CI仍在运行，当时未据此声称完整CI已通过。

## 公开接线及兼容性增量

本轮公开反例先RED（probe仍为0.1且无结构观察），再接入Go私有worker1.2、probe0.3、聚合check0.49、通用确认0.8和保存Hook反馈0.18/fast0.9。Python既有版本与历史报告不修改。文件级缺声明用0..0锚点，不冒充原生错误列；父进程拒绝旧版本冒充Go结构、跨语言规则、错误摘要、非零位置、重复Go结构及未知字段。工作台导入继续独立校验摘要/位置/数量，三份伪造报告拒绝且不增加任务。

实际无原生工具的已初始化项目中，probe、check go/all及保存Hook都产生独立Go候选，原始恢复仍为零。三次检查与保存复用同一任务；任务包含package依据、允许范围、原生确认步骤、复检及关闭条件。补声明后候选消失，但原任务保持open。现有Go环境任务仍可优先恢复原工具。协议验收发现Go vet修复简报原来误用仅支持Python的0.1版本；新Go预览0.13封闭校验Go checker及argv，聚合0.49接纳它，不放宽旧协议。

新Go20例[0.7报告](evidence/go-native-grammar-package-differential-2026-10-05.json)由实际现有SDK执行（10.26秒），原始5TP/0FP/2FN/12TN及1unknown保持不变，组合7TP/0FP/0FN/12TN及1unknown。旧0.6报告不覆盖。随后用相同[原八语言输入](evidence/native-differential-eight-language-go-input-2026-10-05.json)和八个现有SDK生成[新152例证据](evidence/native-differential-eight-language-go-package-2026-10-05.json)：原始39TP/0FP/16FN/91TN及6unknown，组合43TP/0FP/12FN/91TN及6unknown。所有原源码、标签、来源、原始grammar/原生分类及原始比较不变；其它七语言的结构观察和组合比较也不变。回放期间程序及所有所选工具稳定；这些是明确记录程序摘要的开发观察，不是最终发行制品或独立holdout验收。库存保留32语言，资格0。

本轮WASM单元75通过/3条件忽略；七个集成目标37通过/4条件忽略，包含Go项目原生lint、Python公开结构链路及新worker伪造反例。新Go实际协议3测试与Python旧probe/check协议2测试通过。前序底层提交240bc5b的[完整CI](https://github.com/full-stack-plugins/codeguard/actions/runs/37294343353)已success，不能代替本轮提交CI。

```bash
cargo test --locked -p codeguard-cli --features wasm-precheck --lib \
  --test check_go_package_structure --test check_python_structure \
  --test syntax_worker_candidate --test grammar_probe_cli \
  --test grammar_native_differential --test go_native_differential --test go_lint_cli
python3 tests/go_package_structure_schema.py
python3 tests/python_structure_probe_schema.py
python3 tests/check_python_structure_schema.py
```

## 未完成内容（当前）

公开候选及稳定任务已接通；Go确认任务的原生adapter/可信关闭、独立lint go的WASM回退、完整项目覆盖及独立误报验收尚未完成。真实SDK开发差分不等于该adapter已经接入task verify，也不证明类型/CVE/依赖、holdout、跨平台限制或发行资格；32份grammar资格仍为0。既有原生Go vet行为保留，公开npm0.1.4不含本轮变更。

本轮默认构建的Go来源身份、lint和工作台回归12测试通过/4条件忽略；显式Go1.23.4原生发现→重复同步→修复→复扫工作台测试另1项通过（6.55秒），保持任务不因局部零诊断关闭。默认/WASM两配置严格Clippy通过；首次Clippy发现聚合版本选择的相同分支，合并等价条件后通过，没有扩大原生语法能力或修改语料。定向格式、strict OpenSpec、分层及diff检查通过。未执行本轮完整default工作区或32grammar全量回放；本轮新提交的远端CI仍需独立确认。
