//! OWASP Dependency-Check JSON 1.1 报告的有界事实解析；不签发 CVE 清洁结论。

use serde::de::{Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;

/// 单条原生漏洞或原生抑制观察；不将路径或自由文本带入持久结果。
#[derive(Clone, Debug, PartialEq)]
pub struct OwaspAdvisoryObservation {
    /// 原生报告给出的漏洞来源。
    pub source: String,
    /// 原生漏洞 ID；不推断其一定是 CVE。
    pub advisory_id: String,
    /// 最高可解析的 CVSS 4/3/2 基础分；缺失时保持未知。
    pub score: Option<f64>,
    /// 原生报告在该依赖上列出的包标识，尚未与解析依赖图核对。
    pub package_ids: Vec<String>,
    /// 非虚拟文件的 SHA-256，可缺失；不由文件名推断组件身份。
    pub dependency_sha256: Option<String>,
    /// 原生工具已抑制的观察仍保留，不能当作无发现。
    pub suppressed_by_native_tool: bool,
}

/// 一份 OWASP JSON 报告的结构化局部事实；解析成功不证明扫描覆盖或数据库时效。
#[derive(Clone, Debug, PartialEq)]
pub struct OwaspDependencyCheckReport {
    /// 报告中声明的扫描引擎版本，尚未与受保护工具锁绑定。
    pub engine_version: String,
    /// 原生报告声明的项目名，供调用方核对本轮构建根归属。
    pub project_name: String,
    /// 报告生成时间原文，尚未与可信时钟核对。
    pub report_date: String,
    /// 原生数据源名与时间原文；空集合不得被当作数据库新鲜。
    pub data_sources: Vec<(String, String)>,
    /// 原生依赖数组长度；不证明所有项目依赖均被扫描。
    pub dependency_count: usize,
    /// 活动与原生抑制的全部漏洞观察。
    pub advisories: Vec<OwaspAdvisoryObservation>,
}

/// 解析本轮 OWASP Dependency-Check JSON 1.1 报告字节。
/// 缺必需结构、错误标志、分析异常和畸形漏洞均返回错误；调用方仍须验证本轮原生执行、库时效及依赖图归属。
pub fn parse_owasp_dependency_check_json(
    bytes: &[u8],
) -> Result<OwaspDependencyCheckReport, &'static str> {
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
        return Err("owasp_report_size_invalid");
    }
    let StrictValue(value): StrictValue =
        serde_json::from_slice(bytes).map_err(|_| "owasp_report_json_invalid")?;
    let root = object(&value)?;
    if root.contains_key("error") || root.contains_key("errors") {
        return Err("owasp_report_error_declared");
    }
    if root.get("reportSchema").and_then(Value::as_str) != Some("1.1") {
        return Err("owasp_report_schema_unsupported");
    }
    let scan = object(root.get("scanInfo").ok_or("owasp_scan_info_missing")?)?;
    let engine_version = text(scan, "engineVersion")?;
    if scan
        .get("analysisExceptions")
        .is_some_and(|value| value.as_array().is_none_or(|items| !items.is_empty()))
    {
        return Err("owasp_analysis_incomplete");
    }
    let sources = array(scan, "dataSource")?;
    if sources.len() > 1_000 {
        return Err("owasp_report_complexity_exceeded");
    }
    let mut data_sources = Vec::with_capacity(sources.len());
    for source in sources {
        let source = object(source)?;
        data_sources.push((text(source, "name")?, text(source, "timestamp")?));
    }
    let project = object(
        root.get("projectInfo")
            .ok_or("owasp_project_info_missing")?,
    )?;
    let project_name = text(project, "name")?;
    let report_date = text(project, "reportDate")?;
    let dependencies = array(root, "dependencies")?;
    if dependencies.len() > 100_000 {
        return Err("owasp_report_complexity_exceeded");
    }
    let mut advisories = Vec::new();
    for dependency in dependencies {
        let dependency = object(dependency)?;
        let _file_name = text(dependency, "fileName")?;
        let is_virtual = dependency
            .get("isVirtual")
            .and_then(Value::as_bool)
            .ok_or("owasp_dependency_identity_invalid")?;
        let dependency_sha256 = match dependency.get("sha256") {
            None => None,
            Some(value) => {
                let sha = value
                    .as_str()
                    .filter(|value| {
                        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                    .ok_or("owasp_dependency_hash_invalid")?;
                Some(sha.to_owned())
            }
        };
        if is_virtual && dependency_sha256.is_some() {
            return Err("owasp_dependency_identity_invalid");
        }
        let mut package_ids = Vec::new();
        if let Some(packages) = dependency.get("packages") {
            let packages = packages.as_array().ok_or("owasp_packages_invalid")?;
            if packages.len() > 32 {
                return Err("owasp_report_complexity_exceeded");
            }
            for package in packages {
                package_ids.push(text(object(package)?, "id")?);
            }
        }
        for (field, suppressed) in [
            ("vulnerabilities", false),
            ("suppressedVulnerabilities", true),
        ] {
            let Some(items) = dependency.get(field) else {
                continue;
            };
            let items = items.as_array().ok_or("owasp_advisories_invalid")?;
            if items.len() > 1_000 || advisories.len().saturating_add(items.len()) > 1_000 {
                return Err("owasp_report_complexity_exceeded");
            }
            for item in items {
                let item = object(item)?;
                advisories.push(OwaspAdvisoryObservation {
                    source: text(item, "source")?,
                    advisory_id: text(item, "name")?,
                    score: score(item)?,
                    package_ids: package_ids.clone(),
                    dependency_sha256: dependency_sha256.clone(),
                    suppressed_by_native_tool: suppressed,
                });
            }
        }
    }
    Ok(OwaspDependencyCheckReport {
        engine_version,
        project_name,
        report_date,
        data_sources,
        dependency_count: dependencies.len(),
        advisories,
    })
}

/// JSON 键重名会让后值覆盖前值；原生报告不能借此把漏洞数组覆盖为空。
struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrictVisitor;

        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("没有重复键的 JSON")
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(value)))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(Value::Number)
                    .map(StrictValue)
                    .ok_or_else(|| E::custom("非有限数字"))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value.to_owned())))
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value)))
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(StrictValue(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some((key, StrictValue(value))) =
                    map.next_entry::<String, StrictValue>()?
                {
                    if values.insert(key, value).is_some() {
                        return Err(A::Error::custom("重复 JSON 键"));
                    }
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }

        deserializer.deserialize_any(StrictVisitor)
    }
}

fn object(value: &Value) -> Result<&Map<String, Value>, &'static str> {
    value.as_object().ok_or("owasp_report_shape_invalid")
}

fn array<'a>(value: &'a Map<String, Value>, field: &str) -> Result<&'a Vec<Value>, &'static str> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or("owasp_report_array_missing")
}

fn text(value: &Map<String, Value>, field: &str) -> Result<String, &'static str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| {
            !value.trim().is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
        })
        .map(str::to_owned)
        .ok_or("owasp_report_identity_invalid")
}

fn score(value: &Map<String, Value>) -> Result<Option<f64>, &'static str> {
    let unscored = match value.get("unscored") {
        None => false,
        Some(Value::String(value)) if value == "true" => true,
        Some(_) => return Err("owasp_score_invalid"),
    };
    if unscored {
        return Ok(None);
    }
    for (field, score_field) in [
        ("cvssv4", "baseScore"),
        ("cvssv3", "baseScore"),
        ("cvssv2", "score"),
    ] {
        let Some(cvss) = value.get(field) else {
            continue;
        };
        let cvss = object(cvss)?;
        let Some(score) = cvss.get(score_field) else {
            continue;
        };
        let number = score.as_f64().ok_or("owasp_score_invalid")?;
        if !number.is_finite() || !(0.0..=10.0).contains(&number) {
            return Err("owasp_score_invalid");
        }
        return Ok(Some(number));
    }
    Ok(None)
}
