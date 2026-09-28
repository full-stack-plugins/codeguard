use serde::Serialize;

/// 只读发现的项目原生工具候选，不证明可启动、受批准或检查通过。
#[derive(Debug, Serialize)]
pub struct NativeToolCandidate {
    /// 候选所属的项目构建根，相对工作区。
    pub build_root: String,
    /// 对应检查器的稳定身份。
    pub checker_id: String,
    /// 候选或阻塞的精确分类。
    pub state: String,
    /// 从本地包或 Wrapper 配置观察的版本。
    pub observed_version: Option<String>,
    /// 项目依赖声明是否可见；不输出可能包含秘密的原始声明。
    pub declaration_observed: bool,
    /// 下一步原生核验或环境修复动作。
    pub next_action: String,
}
