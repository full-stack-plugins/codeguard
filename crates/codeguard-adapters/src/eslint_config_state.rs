/// ESLint 原配置的只读观察状态；静态存在不等于原生配置有效。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EslintConfigState {
    /// 已观察到配置文件，动态加载和规则仍待原生核验。
    Observed,
    /// 配置文件或其声明已确定无效。
    Invalid,
    /// 配置尚未可判定。
    Unknown,
}
