use crate::{UnpackedFile, package_archive::safe_path};
use std::collections::BTreeSet;
/// 归档展开后的完整普通树，保留空目录；仍未批准或发布。
#[derive(Debug)]
pub struct UnpackedArchive {
    /// 完整冻结普通文件内容。
    pub files: Vec<UnpackedFile>,
    /// 显式空目录及文件所需的全部父目录，不包含树根。
    pub directories: Vec<String>,
}
impl UnpackedArchive {
    /// 补全父目录并验证形状；输入是解析器的冻结文件和显式目录。
    pub(crate) fn from_parts(
        mut files: Vec<UnpackedFile>,
        directories: BTreeSet<String>,
    ) -> Result<Self, &'static str> {
        let mut directories = directories;
        for path in files
            .iter()
            .map(|file| file.relative_path.as_str())
            .chain(directories.clone().iter().map(String::as_str))
        {
            if !safe_path(path) {
                return Err("bundle_path_invalid");
            }
            let mut next = path;
            while let Some((parent, _)) = next.rsplit_once('/') {
                directories.insert(parent.to_owned());
                if directories.len() + files.len() > 100000 {
                    return Err("bundle_entry_limit");
                }
                next = parent;
            }
        }
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        let tree = Self {
            files,
            directories: directories.into_iter().collect(),
        };
        tree.validate_shape()?;
        Ok(tree)
    }
    /// 重新校验可写结构的路径、父目录、冲突和总量；不信任外部构造的对象。
    pub(crate) fn validate_shape(&self) -> Result<(), &'static str> {
        if self.files.len().saturating_add(self.directories.len()) > 100000 {
            return Err("bundle_entry_limit");
        }
        let mut names = BTreeSet::new();
        let mut dirs = BTreeSet::new();
        for path in &self.directories {
            if !safe_path(path) {
                return Err("bundle_path_invalid");
            }
            if !names.insert(path.to_lowercase()) || !dirs.insert(path.as_str()) {
                return Err("bundle_path_conflict");
            }
        }
        let mut bytes = 0u64;
        for file in &self.files {
            if !safe_path(&file.relative_path) {
                return Err("bundle_path_invalid");
            }
            if !names.insert(file.relative_path.to_lowercase()) {
                return Err("bundle_path_conflict");
            }
            bytes = bytes
                .checked_add(file.bytes.len() as u64)
                .ok_or("bundle_byte_limit")?;
            if bytes > 512 * 1024 * 1024 {
                return Err("bundle_byte_limit");
            }
        }
        for path in self
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .chain(self.directories.iter().map(String::as_str))
        {
            if path
                .rsplit_once('/')
                .is_some_and(|(parent, _)| !dirs.contains(parent))
            {
                return Err("bundle_parent_missing");
            }
        }
        Ok(())
    }
}
