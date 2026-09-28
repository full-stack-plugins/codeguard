use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Component, PathBuf},
};
/// 显式npm离线锁文件审计计划；本地观察不证明数据库新鲜度或完整覆盖。
pub struct NpmAuditCommand {
    /// 调用者核验的Node绝对入口。
    pub node: PathBuf,
    /// 调用者核验的npm JS入口。
    pub entry: PathBuf,
    /// 私有缓存目录，不改项目安装树。
    pub cache: PathBuf,
    /// 调用者显式选择的用户配置文件，不能隐式读取宿主凭据。
    pub user_config: PathBuf,
    /// 调用者显式选择的全局配置文件。
    pub global_config: PathBuf,
}
impl NpmAuditCommand {
    /// 使用显式无凭据审计源联网观察；调用者另行批准来源、冻结配置与网络边界。
    /// HTTPS根地址可用，HTTP只接受127.0.0.1验收服务；不声明数据库可信或新鲜。
    pub fn args_for_registry(&self, registry: &str) -> Result<Vec<OsString>, &'static str> {
        let (authority, loopback) = if let Some(rest) = registry.strip_prefix("https://") {
            (rest, false)
        } else if let Some(rest) = registry.strip_prefix("http://") {
            (rest, true)
        } else {
            return Err("npm_audit_registry_invalid");
        };
        let authority = authority.strip_suffix('/').unwrap_or(authority);
        if authority.is_empty() || authority.len() > 255 || authority.contains('/') {
            return Err("npm_audit_registry_invalid");
        }
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        if (loopback && host != "127.0.0.1")
            || host.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
            || port.is_some_and(|p| {
                p.is_empty()
                    || !p.bytes().all(|b| b.is_ascii_digit())
                    || p.parse::<u16>().map_or(true, |v| v == 0)
            })
        {
            return Err("npm_audit_registry_invalid");
        }
        let mut args = self.args()?;
        args.retain(|arg| arg != "--offline");
        args.extend([
            "--offline=false".into(),
            "--prefer-offline=false".into(),
            "--prefer-online=true".into(),
            format!("--registry={registry}").into(),
            format!("--audit-registry={registry}").into(),
            "--fetch-retries=0".into(),
            "--fetch-timeout=10000".into(),
        ]);
        Ok(args)
    }

    /// 返回固定字面argv；只执行audit，不执行fix/安装/生命周期脚本或联网。
    /// 原项目cwd、package-lock及npmrc身份由执行层另行冻结；此计划无门禁权威。
    pub fn args(&self) -> Result<Vec<OsString>, &'static str> {
        let mut paths = BTreeSet::new();
        for path in [
            &self.node,
            &self.entry,
            &self.cache,
            &self.user_config,
            &self.global_config,
        ] {
            let s = path.to_str().ok_or("npm_audit_path_invalid")?;
            if !path.is_absolute()
                || s.len() > 4096
                || s.chars().any(char::is_control)
                || s.split('/').any(|p| p == "." || p == "..")
                || path
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
                || !paths.insert(path)
            {
                return Err("npm_audit_path_invalid");
            }
        }
        Ok(vec![
            "--".into(),
            self.entry.as_os_str().to_owned(),
            "audit".into(),
            "--json".into(),
            "--package-lock-only".into(),
            "--ignore-scripts".into(),
            "--offline".into(),
            "--audit-level=info".into(),
            "--loglevel=silent".into(),
            "--no-fund".into(),
            "--no-update-notifier".into(),
            "--cache".into(),
            self.cache.as_os_str().to_owned(),
            "--userconfig".into(),
            self.user_config.as_os_str().to_owned(),
            "--globalconfig".into(),
            self.global_config.as_os_str().to_owned(),
        ])
    }
}
