/// 按固定入口提供已接线的参数示例；占位路径须由调用者替换并由命令本身核对。
pub(crate) fn examples(command: &str) -> &'static [&'static str] {
    match command {
        "lint" => &[
            "codeguard lint rust . --cargo-tool /absolute/cargo --format json",
            "codeguard lint go . --go-tool /absolute/sdk/bin/go --format json",
            "codeguard lint erlang app.erl --erl-tool /absolute/erl --format json",
            "codeguard lint swift app.swift --swift-tool /absolute/swift --format json",
            "codeguard lint kotlin app.kt --kotlinc-tool /absolute/kotlinc --format json",
            "codeguard lint zig app.zig --zig-tool /absolute/zig --format json",
            "codeguard lint python . --ruff-tool /absolute/ruff --format json",
            "codeguard lint java File.java --maven-tool /absolute/mvn --java-home /absolute/jdk --maven-repo /absolute/repository --repo-sha256 SHA256 --format json",
            "codeguard lint typescript file.ts --node-tool /absolute/node --eslint-entry /absolute/eslint/bin/eslint.js --eslint-version VERSION --config /absolute/eslint.config.js --cwd /absolute/project --format json",
        ],
        "task verify" => &[
            "codeguard task verify TASK_ID . --go-tool /absolute/sdk/bin/go --format json",
            "codeguard task verify TASK_ID . --erl-tool /absolute/erl --format json",
            "codeguard task verify TASK_ID . --swift-tool /absolute/swift --format json",
            "codeguard task verify TASK_ID . --kotlinc-tool /absolute/kotlinc --format json",
            "codeguard task verify TASK_ID . --zig-tool /absolute/zig --format json",
            "codeguard task verify TASK_ID . --ruff-tool /absolute/ruff --format json",
        ],
        "check" => &[
            "codeguard check all . --timeout 60s --jobs 1 --format json",
            "codeguard check go . --go-tool /absolute/sdk/bin/go --format json",
            "codeguard check swift . --swift-tool /absolute/swift --format json",
            "codeguard check kotlin . --kotlinc-tool /absolute/kotlinc --format json",
        ],
        "cve" => &[
            "codeguard cve rust . --cargo-audit-tool /absolute/cargo-audit --db /absolute/rustsec-db --format json",
            "codeguard cve python . --pip-audit-tool /absolute/pip-audit --pip-audit-version VERSION --format json",
            "codeguard cve typescript . --node-tool /absolute/node --npm-entry /absolute/npm/bin/npm-cli.js --npm-version VERSION --userconfig /absolute/user.npmrc --globalconfig /absolute/global.npmrc --format json",
        ],
        _ => &[],
    }
}
