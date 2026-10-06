/// 显式选择的Gradle OWASP原任务；仅保存原生模型身份，不签发漏洞扫描资格。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GradleDependencyCheckTaskPlan {
    /// 所属项目完整路径。
    pub project_path: String,
    /// 本轮显式请求的完整任务路径。
    pub task_path: String,
    /// 原生模型观察到的官方分析/聚合基类。
    pub implementation: String,
}
