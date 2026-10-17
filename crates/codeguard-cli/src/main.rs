//! CodeGuard CLI：默认保留完整运行时；无默认特性构建仅提供智能体接入。
mod agent_integration;
#[cfg(feature = "full-cli")]
mod domain_cli;

fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(code) = guard_install::dispatch(&agent_integration::tool(), &args) {
        return code;
    }
    #[cfg(feature = "full-cli")]
    {
        domain_cli::run()
    }
    #[cfg(not(feature = "full-cli"))]
    {
        if args.as_slice() == ["--version"] {
            println!(
                "codeguard {} (agent integration only)",
                env!("CARGO_PKG_VERSION")
            );
            return std::process::ExitCode::SUCCESS;
        }
        if args.is_empty() || args.as_slice() == ["--help"] || args.as_slice() == ["help"] {
            println!(
                "CodeGuard agent integration only: install, uninstall, context, instructions render|sync, doctor --integration, serve --mcp. Run install --help. Domain checks require the default full-cli build on a supported platform."
            );
            return std::process::ExitCode::SUCCESS;
        }
        eprintln!("This build provides agent integration only; domain command unavailable.");
        std::process::ExitCode::from(4)
    }
}
