//! 已核对的旧注册表规范身份；同数量的任意替换不能充当迁移基线。

/// 核对语言ID与stable/planned归属；参数是候选条目，返回是否匹配57项固定身份。
pub(crate) fn matches(id: &str, status: &str) -> bool {
    matches!(
        (id, status),
        ("arkts" | "cobol" | "metal", "planned")
            | (
                "ansible"
                    | "astro"
                    | "c"
                    | "cfml"
                    | "clojure"
                    | "cpp"
                    | "crystal"
                    | "csharp"
                    | "css"
                    | "cuda"
                    | "dart"
                    | "dockerfile"
                    | "elixir"
                    | "elm"
                    | "erlang"
                    | "fsharp"
                    | "go"
                    | "graphql"
                    | "groovy"
                    | "haskell"
                    | "html"
                    | "java"
                    | "julia"
                    | "kotlin"
                    | "liquid"
                    | "lua"
                    | "luau"
                    | "markdown"
                    | "nim"
                    | "nix"
                    | "objc"
                    | "ocaml"
                    | "pascal"
                    | "perl"
                    | "php"
                    | "powershell"
                    | "protobuf"
                    | "python"
                    | "r"
                    | "ruby"
                    | "rust"
                    | "scala"
                    | "shell"
                    | "solidity"
                    | "sql"
                    | "svelte"
                    | "swift"
                    | "terraform"
                    | "toml"
                    | "typescript"
                    | "vbnet"
                    | "vue"
                    | "yaml"
                    | "zig",
                "stable",
            )
    )
}
