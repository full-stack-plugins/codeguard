use serde::Serialize;

use crate::SyntaxFileState;

/// 已选范围中一个文件的观察，path 为工作区相对路径。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SyntaxFileObservation {
    /// 文件的工作区相对路径。
    pub path: String,
    /// 该文件的实际解析状态。
    pub state: SyntaxFileState,
}
