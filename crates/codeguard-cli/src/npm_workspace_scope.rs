use crate::npm_audit_arguments::NpmAuditArguments;
use std::path::PathBuf;
/// npm项目目录及显式父工作区的物理归属；不搜索祖先或自授工作区身份。
pub(crate) struct NpmWorkspaceScope {
    /// 实际原生审计项目目录。
    pub(crate) project: PathBuf,
    /// 持久任务和预算默认值的工作区目录。
    pub(crate) workspace: PathBuf,
}
impl NpmWorkspaceScope {
    pub(crate) fn resolve(args: &NpmAuditArguments) -> Result<Self, &'static str> {
        let project = args
            .root
            .canonicalize()
            .ok()
            .filter(|p| p.is_dir())
            .ok_or("npm_project_directory_unavailable")?;
        let workspace = if let Some(value) = args.options.get("--workspace") {
            let path = PathBuf::from(value);
            if path.canonicalize().ok().as_ref() != Some(&path) || !path.is_dir() {
                return Err("npm_workspace_invalid");
            }
            path
        } else {
            project.clone()
        };
        if !project.starts_with(&workspace) {
            return Err("npm_scope_outside_workspace");
        }
        Ok(Self { project, workspace })
    }
}
