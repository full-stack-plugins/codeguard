//! Rust 语料校验与显式原生工具回放；不是产品扫描入口。

use codeguard_adapters::{RuffParseState, parse_ruff_json};
use codeguard_cli::corpus::{digest_path, validate_corpus};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

fn run(binary: &Path, args: &[&str], cwd: Option<&Path>) -> Result<std::process::Output, String> {
    let mut command = Command::new(binary);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .output()
        .map_err(|error| format!("启动 {} 失败：{error}", binary.display()))
}

fn digest_executable(path: &Path) -> Result<String, String> {
    let resolved = fs::canonicalize(path)
        .map_err(|error| format!("解析工具路径 {} 失败：{error}", path.display()))?;
    digest_path(&resolved)
}

fn verify_ruff(cases: &[Value], binary: &Path, corpus: &Path) -> Result<(), String> {
    let version = run(binary, &["--version"], None)?;
    if !version.status.success() {
        return Err("Ruff 版本探测失败".into());
    }
    let version = String::from_utf8_lossy(&version.stdout).trim().to_string();
    let binary_digest = digest_executable(binary)?;
    let mut found = false;
    for case in cases {
        if case["adjudication"] != "accepted" || case["tool"]["id"] != "ruff" {
            continue;
        }
        found = true;
        let id = case["id"].as_str().ok_or("缺样本 ID")?;
        if version
            != format!(
                "ruff {}",
                case["tool"]["version"].as_str().ok_or("缺 Ruff 版本")?
            )
            || binary_digest != case["tool"]["binary_sha256"]
        {
            return Err(format!("{id}: Ruff 制品身份不匹配"));
        }
        let path = case["input"]["path"].as_str().ok_or("缺语料路径")?;
        let output = run(
            binary,
            &[
                "check",
                "--isolated",
                "--select",
                "F401",
                "--output-format",
                "json",
                path,
            ],
            Some(corpus),
        )?;
        let exit = output.status.code().ok_or("Ruff 异常终止")?;
        let parsed = parse_ruff_json(exit, &output.stdout);
        if parsed.state != RuffParseState::Valid {
            return Err(format!("{id}: Ruff 原生结果未完成：{:?}", parsed.reason));
        }
        let mut actual: Vec<&str> = parsed
            .diagnostics
            .iter()
            .map(|item| item.code.as_str())
            .collect();
        actual.sort_unstable();
        let mut expected: Vec<&str> = case["expected"]["finding_rule_ids"]
            .as_array()
            .ok_or("缺预期规则列表")?
            .iter()
            .map(|value| value.as_str().ok_or("非法规则 ID"))
            .collect::<Result<_, _>>()?;
        expected.sort_unstable();
        if actual != expected {
            return Err(format!("{id}: Ruff 发现与 oracle 不符"));
        }
    }
    if !found {
        return Err("没有可回放的 Ruff 已接受样本".into());
    }
    Ok(())
}

struct TempProject(PathBuf);

impl TempProject {
    fn create(source: &Path) -> Result<Self, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("codeguard-corpus-{}-{now}", std::process::id()));
        fs::create_dir(&root).map_err(|error| format!("隔离目录创建失败：{error}"))?;
        if let Err(error) = copy_tree(source, &root) {
            let _ = fs::remove_dir_all(&root);
            return Err(error);
        }
        Ok(Self(root))
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("语料树中不允许符号链接".into());
        }
        if metadata.is_dir() {
            fs::create_dir(&target_path).map_err(|error| error.to_string())?;
            copy_tree(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path).map_err(|error| error.to_string())?;
        } else {
            return Err("语料树中有非法文件类型".into());
        }
    }
    Ok(())
}

fn verify_maven(cases: &[Value], binary: &Path, corpus: &Path) -> Result<(), String> {
    let binary_digest = digest_executable(binary)?;
    let version_output = run(binary, &["-version"], None)?;
    if !version_output.status.success() {
        return Err("Maven 版本探测失败".into());
    }
    let version = String::from_utf8_lossy(&version_output.stdout);
    let runtime_home = version
        .lines()
        .find(|line| line.starts_with("Java version: "))
        .and_then(|line| line.split_once("runtime: "))
        .map(|(_, home)| home)
        .ok_or("Maven 未报告运行 JDK 路径")?;
    let runtime_digest = digest_executable(&Path::new(runtime_home).join("bin/java"))?;
    let mut found = false;
    for case in cases {
        if case["adjudication"] != "accepted" || case["tool"]["id"] != "maven" {
            continue;
        }
        found = true;
        let id = case["id"].as_str().ok_or("缺样本 ID")?;
        let maven_version = case["tool"]["version"].as_str().ok_or("缺 Maven 版本")?;
        let jdk_version = case["tool"]["runtime_version"]
            .as_str()
            .ok_or("缺 JDK 版本")?;
        if binary_digest != case["tool"]["binary_sha256"]
            || runtime_digest != case["tool"]["runtime_binary_sha256"]
            || !version.contains(&format!("Apache Maven {maven_version}"))
            || !version.contains(&format!("Java version: {jdk_version}"))
        {
            return Err(format!("{id}: Maven/JDK 身份不匹配"));
        }
        let input = case["input"]["path"].as_str().ok_or("缺语料路径")?;
        let project = TempProject::create(&corpus.join(input))?;
        let pom = project.0.join("pom.xml");
        let pom = pom.to_str().ok_or("隔离路径不是 UTF-8")?;
        let goal = if id == "maven-pom-4-1-environment-failure" {
            "validate"
        } else {
            "verify"
        };
        let output = run(binary, &["-o", "-B", "-f", pom, "-DskipTests", goal], None)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if id == "maven-pom-4-1-environment-failure" {
            if output.status.success() || !stdout.contains("'modelVersion' of '4.1.0' is newer") {
                return Err("Maven 4.1 模型错误未真实复现".into());
            }
            continue;
        }
        if !output.status.success() || !stdout.contains("BUILD SUCCESS") {
            return Err("Maven 离线 verify 没有完成".into());
        }
        let lower = stdout.to_lowercase();
        if ["pmd:", "checkstyle:", "javadoc:"]
            .iter()
            .any(|needle| lower.contains(needle))
        {
            return Err("质量插件实际执行；样本不再证明遗漏义务".into());
        }
        if case["expected"]["unproven_obligations"]
            .as_array()
            .is_none_or(Vec::is_empty)
        {
            return Err("假通过样本缺待证明义务".into());
        }
    }
    if !found {
        return Err("没有可回放的 Maven 已接受样本".into());
    }
    Ok(())
}

fn main() -> ExitCode {
    let mut ruff = None;
    let mut maven = None;
    let mut args = std::env::args().skip(1);
    while let Some(option) = args.next() {
        let slot = match option.as_str() {
            "--verify-ruff" => &mut ruff,
            "--verify-maven" => &mut maven,
            _ => {
                eprintln!("未知参数：{option}");
                return ExitCode::from(2);
            }
        };
        let Some(path) = args.next() else {
            eprintln!("{option} 缺二进制路径");
            return ExitCode::from(2);
        };
        if slot.replace(PathBuf::from(path)).is_some() {
            eprintln!("{option} 不得重复");
            return ExitCode::from(2);
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = root.join("tests/fixtures/corpus");
    let index = corpus.join("oracle.json");
    let plugin = root.join("../codeguard-plugin");
    let result = (|| -> Result<(), String> {
        let counts = validate_corpus(&index, &plugin)?;
        let document: Value =
            serde_json::from_slice(&fs::read(index).map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?;
        let cases = document["cases"].as_array().ok_or("语料列表损坏")?;
        if let Some(binary) = ruff.as_deref() {
            verify_ruff(cases, binary, &corpus)?;
        }
        if let Some(binary) = maven.as_deref() {
            verify_maven(cases, binary, &corpus)?;
        }
        println!(
            "{}",
            serde_json::json!({"accepted":counts.accepted,"pending_reproduction":counts.pending_reproduction,"disputed":counts.disputed})
        );
        Ok(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("语料验收未完成：{error}");
            ExitCode::from(3)
        }
    }
}
