/// Cargo 包 edition 的静态声明；来源：Cargo manifest/workspace 继承契约。
/// 不证明源码归属、工作区成员关系或完整 Cargo 清单有效性。
#[derive(Clone, Debug)]
pub struct CargoEditionDeclaration {
    /// 直接声明或 Cargo 缺字段默认的 edition；继承时为空。
    pub edition: Option<&'static str>,
    /// 是否明确声明 edition.workspace=true。
    pub inherits_workspace: bool,
    /// package.workspace 的原始定位声明，由上层核对路径和成员身份。
    pub workspace_locator: Option<String>,
}

impl CargoEditionDeclaration {
    /// 解析至多256KiB的UTF-8 TOML声明；参数为原始清单字节，返回edition上下文。
    pub fn observe(bytes: &[u8]) -> Result<Self, &'static str> {
        let manifest = parse(bytes)?;
        let package = manifest
            .get("package")
            .and_then(toml::Value::as_table)
            .ok_or("rust_package_edition_unresolved")?;
        let workspace_locator = match package.get("workspace") {
            None => None,
            Some(value) => {
                let locator = value
                    .as_str()
                    .filter(|value| !value.is_empty() && value.len() <= 4096)
                    .ok_or("rust_workspace_locator_invalid")?;
                if manifest.get("workspace").is_some() {
                    return Err("rust_workspace_locator_conflict");
                }
                Some(locator.to_owned())
            }
        };
        let (edition, inherits_workspace) = match package.get("edition") {
            None => (Some("2015"), false),
            Some(toml::Value::String(value)) => (Some(valid_edition(value)?), false),
            Some(toml::Value::Table(table))
                if table.len() == 1
                    && table.get("workspace").and_then(toml::Value::as_bool) == Some(true) =>
            {
                (None, true)
            }
            _ => return Err("rust_package_edition_invalid"),
        };
        Ok(Self {
            edition,
            inherits_workspace,
            workspace_locator,
        })
    }

    /// 从已选择的工作区清单解析显式继承；参数为可选工作区字节，返回有效edition。
    /// 直接声明不受提供者影响；调用方仍需核对工作区定位、成员及输入连续性。
    pub fn resolve(&self, workspace: Option<&[u8]>) -> Result<&'static str, &'static str> {
        if !self.inherits_workspace {
            return self.edition.ok_or("rust_package_edition_unresolved");
        }
        let provider = parse(workspace.ok_or("rust_workspace_edition_unresolved")?)?;
        let edition = provider
            .get("workspace")
            .and_then(|value| value.get("package"))
            .and_then(|value| value.get("edition"))
            .and_then(toml::Value::as_str)
            .ok_or("rust_workspace_edition_unresolved")?;
        valid_edition(edition)
    }

    /// 识别最近工作区的edition提供者；参数为清单字节，非工作区返回None。
    /// 工作区存在但未明确提供有效edition时返回错误，禁止越过它借用更外层声明。
    pub fn workspace_edition(bytes: &[u8]) -> Result<Option<&'static str>, &'static str> {
        let manifest = parse(bytes)?;
        let Some(workspace) = manifest.get("workspace") else {
            return Ok(None);
        };
        let value = workspace
            .get("package")
            .and_then(|value| value.get("edition"))
            .and_then(toml::Value::as_str)
            .ok_or("rust_workspace_edition_unresolved")?;
        valid_edition(value).map(Some)
    }
}

fn parse(bytes: &[u8]) -> Result<toml::Value, &'static str> {
    if bytes.len() > 256 * 1024 {
        return Err("rust_edition_manifest_limit_exceeded");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "rust_edition_manifest_invalid_utf8")?;
    text.parse()
        .map_err(|_| "rust_edition_manifest_invalid_toml")
}

fn valid_edition(value: &str) -> Result<&'static str, &'static str> {
    match value {
        "2015" => Ok("2015"),
        "2018" => Ok("2018"),
        "2021" => Ok("2021"),
        "2024" => Ok("2024"),
        _ => Err("rust_edition_unsupported"),
    }
}
