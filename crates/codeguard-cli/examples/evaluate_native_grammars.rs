//! 开发期原生差分入口；只有显式已安装工具，不安装工具或批准语言资格。
#[cfg(all(feature = "wasm-precheck", unix))]
fn main() -> std::process::ExitCode {
    use codeguard_cli::grammar_native_differential::replay_native_corpus;
    use codeguard_runtime::read_bounded_regular_file;
    use std::{
        collections::BTreeMap,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [binary, corpus, seconds, tools @ ..] = args.as_slice() else {
        eprintln!(
            "用法: evaluate_native_grammars ABS_CODEGUARD CORPUS_JSON TOTAL_SECONDS LANGUAGE=ABS_TOOL..."
        );
        return std::process::ExitCode::from(2);
    };
    let Some(seconds) = seconds
        .parse::<u64>()
        .ok()
        .filter(|s| (1..=3600).contains(s))
    else {
        return std::process::ExitCode::from(2);
    };
    let mut selected = BTreeMap::new();
    for tool in tools {
        let Some((language, path)) = tool.split_once('=') else {
            return std::process::ExitCode::from(2);
        };
        if selected
            .insert(language.to_owned(), PathBuf::from(path))
            .is_some()
        {
            return std::process::ExitCode::from(2);
        }
    }
    let result = read_bounded_regular_file(&PathBuf::from(corpus), 16 * 1024 * 1024)
        .map_err(|_| "native_grammar_corpus_unavailable".to_owned())
        .and_then(|bytes| {
            replay_native_corpus(
                &PathBuf::from(binary),
                &bytes,
                &selected,
                Instant::now() + Duration::from_secs(seconds),
                &AtomicBool::new(false),
            )
        });
    match result {
        Ok(report) => {
            println!("{report}");
            std::process::ExitCode::from(3)
        }
        Err(reason) => {
            eprintln!("原生差分未完成：{reason}");
            std::process::ExitCode::from(2)
        }
    }
}
#[cfg(not(all(feature = "wasm-precheck", unix)))]
fn main() -> std::process::ExitCode {
    eprintln!("开发回放需要 Unix 与 wasm-precheck 特性");
    std::process::ExitCode::from(3)
}
