//! 固定 Checkstyle 原生失败的脱敏分类，不解释源码规则或授予覆盖权威。

/// 识别 10.21.4 原生配置正则异常，返回固定原因码；未知输出返回空。
/// 参数为正常退出码及预算内 stderr，不返回属性值、路径或原始堆栈。
#[must_use]
pub fn checkstyle_native_failure_reason(
    exit_code: Option<i32>,
    stderr: &[u8],
) -> Option<&'static str> {
    if exit_code != Some(254) || stderr.len() > 1024 * 1024 {
        return None;
    }
    let text = std::str::from_utf8(stderr).ok()?;
    let property_failure = text.lines().any(|line| {
        line.starts_with(
            "Caused by: com.puppycrawl.tools.checkstyle.api.CheckstyleException: illegal value ",
        ) && [
            "authorFormat",
            "versionFormat",
            "ignoreMethodNamesRegex",
            "ignoreNamePattern",
        ]
        .iter()
        .any(|name| line.ends_with(&format!("for property '{name}'")))
    });
    let regex_failure = text
        .lines()
        .any(|line| line.starts_with("Caused by: java.util.regex.PatternSyntaxException: "));
    (property_failure && regex_failure).then_some("checkstyle_configuration_regex_invalid")
}
