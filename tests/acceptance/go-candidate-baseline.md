# Go 六类别候选档案（OpenSpec 8.1）

`rulepacks/go_static_candidate_v1.json` 是版本化研究与适配计划，不是可执行的发行能力声明。`schemas/go-static-candidate.schema.json` 描述文件形状，`codeguard-adapters::parse_go_candidate_profile` 进一步核对六类别不重不漏、五个候选平台、类别与原生命令对应、版本状态、来源与缺口。任何类别都固定 `applicable/gap`；旧语言清单的 Go `stable` 不得迁移为新能力 `implemented`。

| 类别 | 候选版本与原生入口 | 一手依据 | 未完成的关键证据 |
|---|---|---|---|
| lint | 项目 Go 工具链，`go vet ./...` | [Go 命令文档](https://pkg.go.dev/cmd/go) | 原生退出/诊断契约、build tags/workspace、其它 lint 规则 |
| comments | Staticcheck 2026.1，显式 ST1000/ST1020/ST1021/ST1022 | [Go 注释规范](https://go.dev/doc/comment)、[Staticcheck 规则](https://staticcheck.dev/docs/checks) | 这些规则非默认且不覆盖全部导出符号缺注释；`gofmt` 不算注释检测 |
| dependencies | 项目 Go 工具链，`go list -m -json all` | [Go Modules 规范](https://go.dev/ref/mod) | `go.work`、replace、离线解析、许可证/SBOM/来源 |
| cve | govulncheck v1.1.4，`-json ./...` | [Go 漏洞管理](https://go.dev/doc/security/vuln/)、[发行记录](https://github.com/golang/vuln/releases) | 漏洞库来源及时效、依赖图/advisory 归属、流式 JSON/退出语义 |
| security | gosec v2.28.0，`-fmt=json ./...` | [gosec 项目](https://github.com/securego/gosec)、[发行记录](https://github.com/securego/gosec/releases) | Go 1.25+ 工具要求、报告/退出语义；不覆盖 secrets/IaC/container |
| build | 项目 Go 工具链，`go build ./...` | [Go 命令文档](https://pkg.go.dev/cmd/go) | build tags/目标平台/可复现性；`go test` 是独立义务 |

候选版本只标识需要实验的制品，尚未锁定二进制摘要或在五个平台跑真实正反例。Go Modules 是本档案的方言；GOPATH 与多工作区需另行建模。原生工具缺失属于环境缺口，不能变为 `not_applicable`。govulncheck 命中可提供可达性证据，但不能替代独立的依赖治理或漏洞库新鲜度证明。

测试：`cargo test -p codeguard-adapters --test go_candidate_contract` 验证完整六槽与旧矩阵 gap，并拒绝缺槽、重复、虚报 implemented/not_applicable、格式化器冒充注释检查、CVE 漏新鲜度及类别工具替换。真实原生正反例、坏配置/版本/报告和平台验收属于 8.2/8.3，未在此证明。
