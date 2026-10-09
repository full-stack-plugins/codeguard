# Shell 7.4 Shell/Dockerfile 验收

> 日期：2026-10-08；OpenSpec 7.4

## 验收标准覆盖

| 标准 | 测试 | 状态 |
|---|---|---|
| zsh 不静默丢弃 | zsh_not_silently_dropped | ✅ |
| zsh/bash 方言区分 | zsh_and_bash_dialects_distinguished | ✅ |
| Hadolint 与配置安全不互相掩盖 | hadolint_and_config_security_not_masking | ✅ |
| Dockerfile 独立检查 | dockerfile_check_separate_from_shell | ✅ |
| IaC 基础检查 | iac_basic_check | ✅ |
| 配置安全报告独立 | config_security_report_independent | ✅ |

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| zsh_not_silently_dropped | zsh 不静默丢弃 | ✅ |
| zsh_and_bash_dialects_distinguished | zsh/bash 方言区分 | ✅ |
| hadolint_and_config_security_not_masking | Hadolint/配置安全不掩盖 | ✅ |
| dockerfile_check_separate_from_shell | Dockerfile 独立检查 | ✅ |
| iac_basic_check | IaC 基础检查 | ✅ |
| config_security_report_independent | 配置安全报告独立 | ✅ |
| **总计** | | **6/6** |

## 验证明细

### zsh 不静默丢弃 ✅
- dialect: "zsh"
- silently_dropped: false
- status: "supported_boundary"

### zsh/bash 方言区分 ✅
- zsh: shebang "#!/bin/zsh"
- bash: shebang "#!/bin/bash"
- 方言独立识别

### Hadolint 与配置安全不互相掩盖 ✅
- Hadolint 故障（tool_not_found）不掩盖配置安全发现
- 配置安全发现不掩盖 Hadolint 故障

### Dockerfile 独立检查 ✅
- Shell 和 Dockerfile 独立类别
- 独立状态报告

### IaC 基础检查 ✅
- terraform_validate / cfn_lint 工具

### 配置安全报告独立 ✅
- exposed_secret / weak_crypto 规则
- 独立 findings 记录

## 结论

7.4 Shell/Dockerfile 验收标准全部满足。
zsh 不静默丢弃，Hadolint 与配置安全报告不互相掩盖故障。
