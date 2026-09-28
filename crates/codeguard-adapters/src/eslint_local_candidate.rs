/// 项目本地 ESLint 的只读准备候选；不能单独授权执行或证明 lint 覆盖。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EslintLocalCandidate {
    /// 结构化候选状态，绝不把 PATH 不可见等同工具缺失。
    pub state: &'static str,
    /// 根 package.json 中的原始 ESLint 版本声明（如有）。
    pub declared_spec: Option<String>,
    /// 项目本地 ESLint package.json 中的具体版本（如有效）。
    pub observed_version: Option<String>,
    /// 供智能体继续核验的具体动作，不是自动安装指令。
    pub next_action: &'static str,
}
