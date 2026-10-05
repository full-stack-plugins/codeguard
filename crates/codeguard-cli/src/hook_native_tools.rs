use std::path::Path;

/// 编辑快检的显式原生入口；来源：OpenSpec 限定范围与调用方工具选择契约。
pub(crate) struct HookNativeTools<'a> {
    pub(crate) ruff: Option<&'a Path>,
    pub(crate) node: Option<&'a Path>,
    pub(crate) kotlinc: Option<&'a Path>,
    pub(crate) swift: Option<&'a Path>,
    pub(crate) ruby: Option<&'a Path>,
    pub(crate) zig: Option<&'a Path>,
}
