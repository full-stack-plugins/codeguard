/// 按固定入口提供已接线的参数示例；占位路径须由调用者替换并由命令本身核对。
pub(crate) fn examples(command: &str) -> &'static [&'static str] {
    match command {
        "format list" => &["codeguard format list"],
        "format check" => &[
            "codeguard format check all . --format json",
            "codeguard format check rust . --tool rustfmt=/absolute/rustfmt --format json",
            "codeguard format check go . --tool gofmt=/absolute/sdk/bin/gofmt --format json",
            "codeguard format check cpp . --tool clang-format=/usr/bin/clang-format --format json",
        ],
        "format apply" => &[
            "codeguard format apply all .",
            "codeguard format apply rust . --tool rustfmt=/absolute/rustfmt",
        ],
        "grammar probe" => &[
            "codeguard grammar probe javascript FILE --format=json",
            "codeguard grammar probe javascript MODULE_FILE --module --format=json",
        ],
        "lint" => &[
            "codeguard lint all . --jobs 2 --timeout 30m --format json",
            "codeguard lint shell app.sh --dialect bash --shellcheck-tool /absolute/shellcheck --format json",
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
        "comments" => &[
            "codeguard comments c api.c --clang-tool /absolute/clang --standard c11 --format json",
            "codeguard comments c /absolute/project/api.c --workspace /absolute/project --clang-tool /absolute/clang --standard c11 --format json",
            "codeguard comments cpp api.cpp --clang-tool /absolute/clang --standard c++17 --format json",
            "codeguard comments java File.java --java-home /absolute/jdk21 --format json",
            "codeguard comments java File.java --workspace . --java-home /absolute/jdk21 --format json",
            "codeguard comments java . --java-home /absolute/jdk21 --maven-tool /absolute/mvn --maven-repo /absolute/repository --repo-sha256 SHA256 --format json",
            "codeguard comments rust . --cargo-tool /absolute/cargo --format json",
            "codeguard comments python . --ruff-tool /absolute/ruff --format json",
        ],
        "task verify" => &[
            "codeguard task verify TASK_ID . --java-home /absolute/jdk21 --format json",
            "codeguard task verify TASK_ID . --rustfmt-tool /absolute/toolchain/bin/rustfmt --format json",
            "codeguard task verify TASK_ID . --go-tool /absolute/sdk/bin/go --format json",
            "codeguard task verify TASK_ID . --erl-tool /absolute/erl --format json",
            "codeguard task verify TASK_ID . --swift-tool /absolute/swift --format json",
            "codeguard task verify TASK_ID . --kotlinc-tool /absolute/kotlinc --format json",
            "codeguard task verify TASK_ID . --zig-tool /absolute/zig --format json",
            "codeguard task verify TASK_ID . --ruff-tool /absolute/ruff --format json",
        ],
        "hook execute" => &[
            "codeguard hook execute . --rustfmt-tool /absolute/toolchain/bin/rustfmt --timeout 30s --format json",
        ],
        "check" => &[
            "codeguard check all . --timeout 60s --jobs 1 --format json",
            "codeguard check java . --gradle-bundle /absolute/gradle --java-home /absolute/jdk --gradle-project-file settings.gradle --gradle-project-file build.gradle --format json",
            "codeguard check java . --gradle-javadoc --gradle-bundle /absolute/gradle --java-home /absolute/jdk21 --gradle-project-file settings.gradle --gradle-project-file build.gradle --gradle-project-file src/main/java/Example.java --format json",
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
