//! 开发期固定 grammar 回放入口；不是新的产品检查命令。

#[cfg(all(feature = "wasm-precheck", unix))]
fn main() -> std::process::ExitCode {
    use codeguard_cli::grammar_evaluation::replay_corpus;
    use codeguard_runtime::read_bounded_regular_file;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [binary, corpus, seconds] = args.as_slice() else {
        eprintln!("用法: evaluate_grammars ABS_CODEGUARD CORPUS_JSON TOTAL_SECONDS");
        return std::process::ExitCode::from(2);
    };
    let Some(seconds) = seconds
        .parse::<u64>()
        .ok()
        .filter(|s| (1..=3600).contains(s))
    else {
        eprintln!("回放总预算须为 1–3600 秒");
        return std::process::ExitCode::from(2);
    };
    let result = read_bounded_regular_file(&PathBuf::from(corpus), 16 * 1024 * 1024)
        .map_err(|_| "grammar_evaluation_corpus_unavailable".into())
        .and_then(|bytes| {
            replay_corpus(
                &PathBuf::from(binary),
                &bytes,
                Instant::now() + Duration::from_secs(seconds),
                &AtomicBool::new(false),
            )
        });
    match result {
        Ok(report) => {
            println!("{report}");
            // 任何开发回归结果都不能充当完整语法或发布验收成功。
            std::process::ExitCode::from(3)
        }
        Err(reason) => {
            eprintln!("开发回放未完成：{reason}");
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(not(all(feature = "wasm-precheck", unix)))]
fn main() -> std::process::ExitCode {
    eprintln!("开发回放需要 Unix 与 wasm-precheck 特性");
    std::process::ExitCode::from(3)
}
