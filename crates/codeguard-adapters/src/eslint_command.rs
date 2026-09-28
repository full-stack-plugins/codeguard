use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// ESLint 原生 JS 入口的字面参数计划；不执行 Node、读取配置或安装工具。
pub struct EslintCommand {
    /// 显式 Node 二进制绝对路径；执行上下文必须核验身份及环境。
    pub node: PathBuf,
    /// 原生 ESLint 包的 JS 入口绝对路径；包与插件闭包须独立核验。
    pub entry: PathBuf,
    /// 原 flat config 绝对路径，不自动选择或替换配置。
    pub config: PathBuf,
    /// 本轮显式完整源码范围；调用方须冻结源集及物理输入身份。
    pub sources: Vec<PathBuf>,
    /// 本轮独立 JSON 报告绝对槽位；新鲜性由 runtime 管理。
    pub report: PathBuf,
    /// 显式警告数量上限；为空保留原生缺省阈值，结果解析须使用同一值。
    pub max_warnings: Option<u32>,
}

impl EslintCommand {
    /// 返回单文件原生print-config参数；沿用原入口/配置，不加入忽略、修复或安装选项。
    /// 输入路径与参数预算沿用扫描校验；调用者仍需冻结身份、工作目录与截止时间。
    pub fn effective_config_args(&self) -> Result<Vec<OsString>, &'static str> {
        if self.sources.len() != 1 {
            return Err("eslint_effective_scope_invalid");
        }
        self.args()?;
        Ok(vec![
            "--".into(),
            self.entry.as_os_str().to_owned(),
            "--no-config-lookup".into(),
            "--config".into(),
            self.config.as_os_str().to_owned(),
            "--print-config".into(),
            self.sources[0].as_os_str().to_owned(),
        ])
    }
    /// 返回传给已核验 Node 的固定 argv；路径/范围/预算无效则拒绝规划。
    /// 不核验符号链接、文件类型、插件执行安全或报告新鲜性，不授予检查完成权威。
    pub fn args(&self) -> Result<Vec<OsString>, &'static str> {
        if self.sources.is_empty() || self.sources.len() > 10_000 {
            return Err("eslint_source_scope_invalid");
        }
        let mut inputs = BTreeSet::new();
        for path in [&self.node, &self.entry, &self.config, &self.report] {
            if !safe_absolute(path) {
                return Err("eslint_path_not_safe_absolute");
            }
            if !inputs.insert(path) {
                return Err("eslint_input_output_collision");
            }
        }
        let mut sources = BTreeSet::new();
        for path in &self.sources {
            if !safe_absolute(path) {
                return Err("eslint_path_not_safe_absolute");
            }
            // 配置文件也是可检查源码；只允许与原配置共享只读输入身份。
            if !sources.insert(path) || (inputs.contains(path) && path != &self.config) {
                return Err("eslint_input_output_collision");
            }
        }
        let mut args: Vec<OsString> = vec![
            "--".into(),
            self.entry.as_os_str().to_owned(),
            "--no-config-lookup".into(),
            "--config".into(),
            self.config.as_os_str().to_owned(),
            "--format".into(),
            "json".into(),
            "--output-file".into(),
            self.report.as_os_str().to_owned(),
        ];
        if let Some(max) = self.max_warnings {
            args.extend(["--max-warnings".into(), max.to_string().into()]);
        }
        args.push("--".into());
        args.extend(self.sources.iter().map(|p| p.as_os_str().to_owned()));
        let total = args.iter().try_fold(0_usize, |sum, arg| {
            sum.checked_add(arg.as_encoded_bytes().len() + 1)
        });
        if total.is_none_or(|size| size > 128 * 1024) {
            return Err("eslint_argument_budget_exceeded");
        }
        Ok(args)
    }
}

fn safe_absolute(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    path.is_absolute()
        && text.len() <= 4096
        && !text.chars().any(char::is_control)
        && !text
            .split(|c| c == '/' || (cfg!(windows) && c == '\\'))
            .any(|part| matches!(part, "." | ".."))
        && path.components().all(|c| {
            matches!(
                c,
                Component::RootDir | Component::Prefix(_) | Component::Normal(_)
            )
        })
}
