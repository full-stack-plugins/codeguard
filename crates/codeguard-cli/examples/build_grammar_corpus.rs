//! Rust 开发期语料导入；读取随仓上游 corpus，不运行编译器或改变批准状态。

use codeguard_adapters::{parse_grammar_test_corpus, parse_unique_json};
use codeguard_cli::grammar_evaluation::validate_corpus;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn build(base: &Path, additional: &Path) -> Result<Value, String> {
    let bytes =
        read_bounded_regular_file(base, 16 * 1024 * 1024).map_err(|_| "corpus_base_unavailable")?;
    validate_corpus(&bytes)?;
    let mut document = parse_unique_json(&bytes).map_err(str::to_owned)?;
    document["schema_version"] = json!("0.2.0");
    let cases = document["cases"]
        .as_array_mut()
        .ok_or("corpus_cases_invalid")?;
    for case in cases.iter_mut() {
        case["cohort"] = json!(if case["label"] == "pending" {
            "provisional_syntax"
        } else {
            "repository_regression"
        });
    }
    let additional = read_bounded_regular_file(additional, 16 * 1024 * 1024)
        .map_err(|_| "corpus_additional_unavailable")?;
    let additional = parse_unique_json(&additional).map_err(str::to_owned)?;
    cases.extend(
        additional
            .as_array()
            .ok_or("corpus_additional_invalid")?
            .iter()
            .cloned(),
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammars/dart/corpus");
    let mut paths = std::fs::read_dir(&root)
        .map_err(|_| "dart_corpus_directory_unavailable")?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "dart_corpus_entry_unavailable")?;
    paths.sort();
    let mut total = 0;
    for path in paths {
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let bytes = read_bounded_regular_file(&path, 16 * 1024 * 1024)
            .map_err(|_| "dart_corpus_file_unavailable")?;
        let source = std::str::from_utf8(&bytes).map_err(|_| "dart_corpus_encoding_invalid")?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("dart_corpus_path_invalid")?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("dart_corpus_path_invalid")?;
        let origin_sha = format!("{:x}", Sha256::digest(&bytes));
        for (index, sample) in parse_grammar_test_corpus(source)?.into_iter().enumerate() {
            total += 1;
            cases.push(json!({"id":format!("dart-upstream-{stem}-{index}"),"language":"dart",
                "source_sha256":format!("{:x}",Sha256::digest(sample.source.as_bytes())),"source":sample.source,
                "expected_valid":!sample.expected_error,"label":"regression","cohort":"upstream_grammar_regression",
                "origin":format!("grammars/dart/corpus/{name}#sha256={origin_sha}:{}",sample.name)}));
        }
    }
    if total != 150 {
        return Err("dart_corpus_frozen_count_changed".into());
    }
    validate_corpus(&serde_json::to_vec(&document).map_err(|_| "corpus_serialization_failed")?)?;
    Ok(document)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [base, additional] = args.as_slice() else {
        eprintln!("用法: build_grammar_corpus BASE_JSON ADDITIONAL_JSON");
        return ExitCode::from(2);
    };
    match build(&PathBuf::from(base), &PathBuf::from(additional)) {
        Ok(document) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&document).expect("JSON value 可序列化")
            );
            ExitCode::SUCCESS
        }
        Err(reason) => {
            eprintln!("语料导入拒绝：{reason}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn importer_reproduces_every_checked_in_source_and_origin_byte() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let document = super::build(
            &root.join("tests/fixtures/grammar_regression.json"),
            &root.join("tests/fixtures/grammar_additional_regressions.json"),
        )
        .unwrap();
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&document).unwrap()),
            include_str!("../../../tests/fixtures/grammar_regression_v0_2.json")
        );
    }
}
