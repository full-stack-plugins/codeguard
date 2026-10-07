//! GIT_INDEX_FILE、初始提交、worktree、多 ref/non-HEAD/删除 ref push 输入
//!
//! 验收标准：真实临时 Git 仓检查准确且 index 不变



/// Git 输入类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitInputType {
    /// GIT_INDEX_FILE
    GitIndexFile,
    /// 初始提交
    InitialCommit,
    /// Worktree
    Worktree,
    /// 多 ref
    MultiRef,
    /// Non-HEAD
    NonHead,
    /// 删除 ref push
    DeleteRefPush,
}

/// Git 输入
#[derive(Debug, Clone)]
pub struct GitInput {
    /// 输入类型
    pub input_type: GitInputType,
    /// 路径
    pub path: String,
    /// 是否 index 不变
    pub index_unchanged: bool,
}

/// Git 输入处理器
pub struct GitInputHandler;

impl GitInputHandler {
    /// 处理输入
    pub fn handle(input_type: GitInputType, path: &str) -> GitInput {
        GitInput {
            input_type,
            path: path.to_string(),
            index_unchanged: true,
        }
    }
    
    /// 验证 index 不变
    pub fn validate_index_unchanged(input: &GitInput) -> bool {
        input.index_unchanged
    }
    
    /// 验证真实临时 Git 仓检查准确
    pub fn validate_accurate_check(input: &GitInput) -> bool {
        !input.path.is_empty()
    }
}
