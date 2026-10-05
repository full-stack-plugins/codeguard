# Go限定任务原生关闭与复发验收

规格：`introduce-rust-codeguard-cli` syntax-precheck 的Go双制品关闭要求，涉及9.10/14.10/14.12；保持同一规格事实源。

## 实现和边界

Unix宿主SDK新增 `verify_go_task_resolution` 与 `GoTaskResolutionRequest`，复用既有验签、限定任务、租约、失败尝试、原反例/当前源码对照、追加父链和重开。Go签名策略1.6.0必须同时绑定Go1.23.4、同SDK gofmt SHA以及规范路径联合SHA；旧策略不能接受新字段。证据0.7.0独立保留，旧Python0.6和其它语言协议不改。

调用前辅助制品失配拒绝执行；执行中变化保存 inputs_stale；原样本零诊断进入误报调查，不得用候选消失关闭。当前完整零诊断、原始正向诊断、独立批准和所有输入稳定才关闭限定任务。普通 `task verify --go-tool` 可记录已关闭任务复发；自写历史即使重算文件名和事件SHA也不能替换已批准辅助制品绑定。CLI不提供项目自批入口，所有收据 delivery_decision=not_evaluated。

## 证据

新增公开API缺失时编译RED，恢复后受控SDK回归覆盖关闭/幂等/重开/再次关闭、辅助身份失配、字段缺失、执行中辅助变化、错误宿主/时钟/回滚/过期、原生反证、丢失证据、旧策略拒绝扩展和重算摘要的篡改。替身是执行边界测试，不作为语法oracle。

显式执行已安装 `/usr/local/go/bin/go` 的条件测试，真实Go/gofmt对原 `func f() {}` 与补package源码产生诊断/零诊断，限定关闭和复发链路1通过/0忽略，5.59秒。宿主批准密钥仍为测试夹具；产物位于 `evidence/go1234-resolution-2026-10-05-{resolved,reopened}-{policy,evidence,receipt}.json`。闭合schema及实际双制品绑定开发校验2通过。后续相关回归与Clippy结果追加在此文件，不借用旧提交CI。

## 未完成

生产宿主信任根、默认插件可信上下文、全语言/全平台关闭、独立holdout、32 grammar精度资格、全项目义务门禁与发行仍未完成。此切片不关闭9.10/14.10/14.12父任务。

相关八目标回归40通过/0失败/11条件忽略，含Go关闭7通过/1条件忽略；显式实际工具目标另1通过/0忽略。WASM严格Clippy通过10.18秒，开发协议2通过；OpenSpec strict、分层和diff检查通过。默认配置检查与最终CI独立记录。

默认严格Clippy通过7.94秒；受保护Erlang差分草稿的字节摘要仍为 `2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6`，未修改或作为本轮oracle执行。

WASM CLI单元77通过/0失败/3条件忽略；没有重跑32份grammar全量语料或声明其资格完成。当前提交的Linux/MSRV完整CI须独立验收。

默认CLI单元63通过/0失败/3条件忽略，默认功能不因新增SDK入口要求WASM制品。
