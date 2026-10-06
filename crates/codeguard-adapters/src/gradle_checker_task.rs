//! Gradle原生任务事实，不根据任务名猜测漏洞引擎。
use serde::{Deserialize, Serialize};
/// 原生模型观察的任务身份、实现类及启用状态。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GradleCheckerTask {
    /// 当前子项目中的任务名，调用方须以字面参数传递。
    pub name: String,
    /// 原生反射观察的检查实现基类；未识别任务使用实际类名。
    pub implementation: String,
    /// Gradle任务的当前enabled值，不表示已执行或覆盖完整。
    pub enabled: bool,
}
impl GradleCheckerTask {
    /// 返回是否为已启用的官方OWASP分析或聚合任务；名称相似的普通任务不成立。
    pub fn is_dependency_check(&self) -> bool {
        self.enabled
            && matches!(
                self.implementation.as_str(),
                "org.owasp.dependencycheck.gradle.tasks.Analyze"
                    | "org.owasp.dependencycheck.gradle.tasks.Aggregate"
            )
    }
}
