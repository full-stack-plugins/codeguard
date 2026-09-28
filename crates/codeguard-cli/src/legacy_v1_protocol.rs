//! 旧入口数字协议的显式映射；不得将兼容结果用于新交付认证。

/// 旧 CLI 或 Hook 的具名入口族，不能按退出码跨族推断含义。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyV1Entry {
    /// `bin/codeguard check`，含默认无子命令入口。
    Check,
    /// `bin/codeguard cve`。
    Cve,
    /// `bin/codeguard dockerfile`。
    Dockerfile,
    /// `bin/codeguard fix`。
    Fix,
    /// `bin/codeguard detect`。
    Detect,
    /// `bin/codeguard init`。
    Init,
    /// `bin/codeguard java-plan`。
    JavaPlan,
    /// `bin/codeguard` 未知子命令。
    UnknownCommand,
    /// 旧 `PreToolUse Bash` Git Hook。
    PreToolGitGuard,
    /// 旧 SessionStart、UserPromptSubmit、PostToolUse、Stop 观察 Hook。
    ObservingHook,
}

/// 旧入口已经归类的观察；不从原生进程数字自行制造问题。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyV1Signal {
    /// 旧检查报告为 PASS。
    Pass,
    /// 旧检查报告为 FAIL。
    Fail,
    /// 旧检查报告为 UNVERIFIED。
    Unverified,
    /// 旧检查报告为 SKIPPED。
    Skipped,
    /// 旧检查报告为 PLANNED。
    Planned,
    /// 旧格式化器成功，仅证明命令退出，未证实源码修复。
    FormatterSucceeded,
    /// 旧格式化器失败。
    FormatterFailed,
    /// 旧 fix 的显式 dry-run，未执行 formatter。
    DryRun,
    /// 旧 fix 在执行前没有识别到语言。
    NoLanguages,
    /// 旧 fix 发现 Git 改动集为空。
    NoChangedFiles,
    /// 有语言和改动文件，但没有适用该语言的 formatter 目标。
    NoApplicableFiles,
    /// 旧入口参数或项目配置错误。
    InvalidInput,
    /// 旧 Hook 报告宿主拦截，包括 Git 意图不可确认。
    HookBlocked,
    /// 旧 Hook 仅反馈或放行。
    HookAllowed,
}

/// 兼容结果仅描述旧出口，不包含新质量认证。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyV1Projection {
    /// 旧调用方实际接收的数字。
    pub legacy_exit_code: u8,
    /// 明确拒绝把旧成功映射为新交付 allow。
    pub new_delivery_decision: &'static str,
}

/// 按旧入口各自优先级计算兼容数字；参数为旧实现已归类的结果，不是原生退出码。
///
/// 不支持的入口/信号组合返回 `None`，避免猜测。返回值恒为 `not_evaluated`，
/// 后续 C35 入口仍需核对真实旧协议、输出与宿主行为后才能调用此映射。
#[must_use]
pub fn project_legacy_v1(
    entry: LegacyV1Entry,
    signals: &[LegacyV1Signal],
) -> Option<LegacyV1Projection> {
    use LegacyV1Entry as Entry;
    use LegacyV1Signal as Signal;

    if matches!(signals, [Signal::InvalidInput])
        && matches!(
            entry,
            Entry::Check | Entry::Cve | Entry::Dockerfile | Entry::Fix | Entry::JavaPlan
        )
    {
        return Some(LegacyV1Projection {
            legacy_exit_code: if entry == Entry::Cve { 3 } else { 2 },
            new_delivery_decision: "not_evaluated",
        });
    }
    if entry == Entry::Fix
        && matches!(
            signals,
            [Signal::NoLanguages | Signal::NoChangedFiles | Signal::NoApplicableFiles]
        )
    {
        return Some(LegacyV1Projection {
            legacy_exit_code: u8::from(signals[0] == Signal::NoLanguages),
            new_delivery_decision: "not_evaluated",
        });
    }

    let valid = match entry {
        Entry::Check => signals.iter().all(|signal| {
            matches!(
                signal,
                Signal::Pass
                    | Signal::Fail
                    | Signal::Unverified
                    | Signal::Skipped
                    | Signal::Planned
            )
        }),
        Entry::Cve | Entry::Dockerfile => signals
            .iter()
            .all(|signal| matches!(signal, Signal::Pass | Signal::Fail | Signal::Unverified)),
        Entry::Fix => {
            !signals.is_empty()
                && signals.iter().all(|signal| {
                    matches!(
                        signal,
                        Signal::FormatterSucceeded
                            | Signal::FormatterFailed
                            | Signal::DryRun
                            | Signal::Skipped
                            | Signal::Planned
                            | Signal::Unverified
                    )
                })
        }
        Entry::Detect => matches!(signals, [Signal::Pass] | [Signal::Unverified]),
        Entry::JavaPlan => {
            matches!(
                signals,
                [Signal::Planned] | [Signal::Skipped] | [Signal::Unverified]
            )
        }
        Entry::Init | Entry::UnknownCommand => signals.is_empty(),
        Entry::PreToolGitGuard => matches!(signals, [Signal::HookBlocked] | [Signal::HookAllowed]),
        Entry::ObservingHook => matches!(signals, [Signal::HookAllowed]),
    };
    if !valid {
        return None;
    }

    let code = match entry {
        Entry::Check | Entry::Cve => {
            if signals.contains(&Signal::Fail) {
                2
            } else if signals.is_empty()
                || signals.contains(&Signal::Unverified)
                || (entry == Entry::Check && signals.contains(&Signal::Planned))
            {
                1
            } else {
                0
            }
        }
        Entry::Dockerfile => {
            if signals.is_empty() || signals.contains(&Signal::Unverified) {
                1
            } else if signals.contains(&Signal::Fail) {
                2
            } else {
                0
            }
        }
        Entry::Fix => {
            if signals.contains(&Signal::FormatterFailed)
                || signals.contains(&Signal::Planned)
                || signals.contains(&Signal::Unverified)
            {
                1
            } else {
                0
            }
        }
        Entry::Detect | Entry::JavaPlan => u8::from(signals[0] == Signal::Unverified),
        Entry::Init | Entry::ObservingHook => 0,
        Entry::UnknownCommand => 1,
        Entry::PreToolGitGuard => {
            if signals[0] == Signal::HookBlocked {
                2
            } else {
                0
            }
        }
    };
    Some(LegacyV1Projection {
        legacy_exit_code: code,
        new_delivery_decision: "not_evaluated",
    })
}
