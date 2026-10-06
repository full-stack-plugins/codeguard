//! Rust 迁移核对工具；核对57项全部字段，只生成审计，不执行旧命令。
use codeguard_adapters::{parse_legacy_registry, parse_unique_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::process::ExitCode;

fn audit() -> Result<Value, String> {
    let plugin = include_bytes!("../../../tests/fixtures/legacy_languages_plugin_2026_10_06.json");
    let rust = include_bytes!("../../../rulepacks/legacy_languages.json");
    for bytes in [plugin.as_slice(), rust.as_slice()] {
        parse_legacy_registry(std::str::from_utf8(bytes).map_err(|e| e.to_string())?)?;
    }
    let a = parse_unique_json(plugin).map_err(str::to_owned)?;
    let b = parse_unique_json(rust).map_err(str::to_owned)?;
    let indexed = |doc: &Value| -> BTreeMap<String, Value> {
        doc["languages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["id"].as_str().unwrap().to_string(), r.clone()))
            .collect()
    };
    let old = indexed(&a);
    let new = indexed(&b);
    if old.keys().ne(new.keys()) {
        return Err("旧与新注册表语言身份不一致".into());
    }
    let mut rows = Vec::new();
    let mut count = 0;
    for (id, left) in old {
        let right = &new[&id];
        let mut keys: Vec<&String> = left
            .as_object()
            .unwrap()
            .keys()
            .chain(right.as_object().unwrap().keys())
            .collect();
        keys.sort();
        keys.dedup();
        let mut deltas = Vec::new();
        let mut preserved = Vec::new();
        for key in keys {
            if left.get(key) == right.get(key) {
                preserved.push(key);
                continue;
            }
            let (disposition, fixture) = match (id.as_str(), key.as_str()) {
                ("cpp", "extensions") | ("r", "extensions") => (
                    "preserve_corrected_coverage",
                    "crates/codeguard-cli/tests/detect_cli.rs#detect_includes_r_and_cpp_explicit_suffixes_without_guessing_shared_headers",
                ),
                ("typescript", "extensions") => (
                    "preserve_corrected_coverage",
                    "crates/codeguard-cli/tests/detect_cli.rs#detect_includes_typescript_module_sources_without_package_manifest",
                ),
                ("cpp", "gate") => (
                    "legacy_compatibility_only",
                    "crates/codeguard-cli/tests/legacy_v1_protocol_contract.rs",
                ),
                _ => return Err(format!("未分类注册表差异：{id}/{key}")),
            };
            deltas.push(json!({"field":key,"legacy":left.get(key),"rust_snapshot":right.get(key),"disposition":disposition,"spec_ref":"openspec/changes/introduce-rust-codeguard-cli/specs/native-tool-adapters/spec.md#requirement-migration-shall-account-for-every-legacy-language-entry","fixture_ref":fixture}));
            count += 1;
        }
        rows.push(json!({"language":id,"legacy_status":left["status"],"rust_status":right["status"],"preserved_fields":preserved,"differences":deltas,"legacy_commands_disposition":"legacy_compatibility_only","capability_disposition":"not_qualified_by_legacy_registration"}));
    }
    Ok(
        json!({"schema_version":"0.1.0","report_type":"legacy_registry_migration_audit","plugin_source_commit":"dec5f9d1361eefff493e120b4278939a243814a9","plugin_registry_sha256":format!("{:x}",Sha256::digest(plugin)),"rust_registry_sha256":format!("{:x}",Sha256::digest(rust)),"language_count":57,"stable_count":54,"planned_count":3,"field_difference_count":count,"delivery_decision":"not_evaluated","rows":rows}),
    )
}
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !matches!(args.as_slice(), [] | [_, _])
        || (!args.is_empty() && !matches!(args[0].as_str(), "--write" | "--check"))
    {
        eprintln!("用法：audit_legacy_registry [--write|--check FILE]");
        return ExitCode::from(2);
    }
    let report = match audit() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(3);
        }
    };
    let bytes = serde_json::to_vec_pretty(&report).unwrap();
    if args.is_empty() {
        println!("{}", String::from_utf8(bytes).unwrap());
        return ExitCode::SUCCESS;
    }
    if args[0] == "--check" {
        let current = match fs::read(&args[1]) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(3);
            }
        };
        if parse_unique_json(&current).ok().as_ref() != Some(&report) {
            eprintln!("迁移审计已过期或损坏");
            return ExitCode::from(3);
        }
        return ExitCode::SUCCESS;
    }
    let mut bytes = bytes;
    bytes.push(b'\n');
    match fs::write(&args[1], bytes) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(3)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::audit;
    #[test]
    fn audit_accounts_for_every_language_and_each_changed_field() {
        let report = audit().unwrap();
        assert_eq!(report["field_difference_count"], 4);
        let rows = report["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 57);
        assert_eq!(
            rows.iter()
                .filter(|r| r["legacy_status"] == "stable")
                .count(),
            54
        );
        assert_eq!(
            rows.iter()
                .filter(|r| r["legacy_status"] == "planned")
                .count(),
            3
        );
        for row in rows {
            assert_eq!(row["legacy_status"], row["rust_status"]);
            assert!(!row["preserved_fields"].as_array().unwrap().is_empty());
            assert_eq!(
                row["capability_disposition"],
                "not_qualified_by_legacy_registration"
            );
            for change in row["differences"].as_array().unwrap() {
                assert!(
                    change["spec_ref"]
                        .as_str()
                        .unwrap()
                        .contains("native-tool-adapters")
                );
                assert!(change["fixture_ref"].as_str().unwrap().contains("tests/"));
            }
        }
        assert_eq!(report["delivery_decision"], "not_evaluated");
    }
}
