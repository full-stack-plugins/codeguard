/// 已通过归档结构核验的内存文件；未落盘，不代表安装或批准。
#[derive(Debug)]
pub struct UnpackedFile {
    /// 规范相对路径，不含点段、绝对路径或平台歧义。
    pub relative_path: String,
    /// 完整冻结内容；后续发布仍须验证归属与权限。
    pub bytes: Vec<u8>,
}
