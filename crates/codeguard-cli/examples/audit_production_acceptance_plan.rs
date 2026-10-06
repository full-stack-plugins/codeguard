use codeguard_adapters::parse_production_acceptance_plan;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let raw = std::fs::read(root.join("rulepacks/production_acceptance_plan_v1.json"))?;
    let doc = parse_production_acceptance_plan(&raw)?;
    let hashes = doc["source_hashes"].as_object().ok_or("缺摘要")?;
    for (reference, expected) in hashes {
        let candidate = root.join(reference);
        let metadata = std::fs::symlink_metadata(&candidate)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 8 * 1024 * 1024
            || !candidate.canonicalize()?.starts_with(&root)
        {
            return Err(format!("来源越界或超出预算：{reference}").into());
        }
        let digest = format!("{:x}", Sha256::digest(std::fs::read(candidate)?));
        if expected.as_str() != Some(digest.as_str()) {
            return Err(format!("来源已漂移：{reference}").into());
        }
    }
    let tasks = std::fs::read_to_string(
        root.join("openspec/changes/introduce-rust-codeguard-cli/tasks.md"),
    )?;
    let mut task_count = 0;
    for row in doc["languages"].as_array().ok_or("缺语言")? {
        for cell in row["capabilities"].as_object().ok_or("缺能力")?.values() {
            for task in cell["task_refs"].as_array().ok_or("缺任务")? {
                let id = task.as_str().ok_or("无效任务")?;
                if !tasks.lines().any(|line| {
                    line.starts_with(&format!("- [ ] {id} "))
                        || line.starts_with(&format!("- [x] {id} "))
                }) {
                    return Err(format!("任务不存在：{id}").into());
                }
                task_count += 1;
            }
        }
    }
    println!(
        "{}",
        json!({"status":"mapping_complete_qualification_blocked","languages":57,"core_obligations":228,"source_hashes_checked":hashes.len(),"task_references_checked":task_count,"qualification":"not_granted"})
    );
    Ok(())
}
