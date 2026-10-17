use crate::gradle_model_probe_request::Request as NativeRequest;

/// 显式Gradle OWASP原任务请求；已安装工具、选定输入和期限复用原生模型契约。
pub struct Request {
    /// 显式已有caches/modules-2目录；可缺失，仅复制依赖缓存，不读取用户配置。
    pub module_cache: Option<std::path::PathBuf>,
    /// 原生工具与不可变选定项目输入；不代表完整依赖范围。
    pub native: NativeRequest,
    /// 必须显式指定完整任务路径，不根据任务名猜测引擎。
    pub task_paths: Vec<String>,
}
