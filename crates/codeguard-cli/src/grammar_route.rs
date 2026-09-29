//! 按源码后缀和明确的嵌入边界选择候选 grammar；不推断项目语言版本。

/// 一段可独立解析的候选源码；嵌入片段的位置相对于原文件的字节起点。
#[derive(Clone, Debug)]
pub struct GrammarRoute<'a> {
    /// 固定资产清单中的语言标识。
    pub language: &'static str,
    /// 交给隔离解析进程的原始字节。
    pub source: &'a [u8],
    /// 源码片段在文件中的字节偏移。
    pub byte_offset: usize,
    /// 整文件或显式 CFQuery 标签体。
    pub scope: &'static str,
}

/// 从普通源码路径选择固定 grammar；歧义后缀不作猜测。
/// 参数为相对路径及本轮源码字节；返回零个或多个待核对的候选片段。
pub fn route_source<'a>(relative_path: &str, source: &'a [u8]) -> Vec<GrammarRoute<'a>> {
    let extension = relative_path
        .rsplit_once('.')
        .map(|(_, extension)| extension);
    let language = match extension {
        Some("ets") => "arkts",
        Some("c") => "c",
        Some("cfm" | "cfc") => "cfml",
        Some("cfs") => "cfscript",
        Some("cbl" | "cob" | "cpy") => "cobol",
        Some("cpp" | "cc" | "hpp") => "cpp",
        Some("cs") => "csharp",
        Some("dart") => "dart",
        Some("erl" | "hrl") => "erlang",
        Some("go") => "go",
        Some("java") => "java",
        Some("js" | "jsx" | "mjs" | "cjs") => "javascript",
        Some("kt" | "kts") => "kotlin",
        Some("lua") => "lua",
        Some("luau") => "luau",
        Some("nix") => "nix",
        Some("m" | "mm") => "objc",
        Some("pas" | "dpr" | "dpk" | "lpr") => "pascal",
        Some("php") => "php",
        Some("py") => "python",
        Some("r") => "r",
        Some("rb") => "ruby",
        Some("rs") => "rust",
        Some("scala" | "sc") => "scala",
        Some("sol") => "solidity",
        Some("swift") => "swift",
        Some("tf" | "tfvars" | "tofu") => "terraform",
        Some("tsx") => "tsx",
        Some("ts") => "typescript",
        Some("vb") => "vbnet",
        Some("zig") => "zig",
        _ => return Vec::new(),
    };
    let mut routes = vec![GrammarRoute {
        language,
        source,
        byte_offset: 0,
        scope: "whole_file",
    }];
    if language == "cfml" {
        // 只取完整且有明确标签边界的查询体；普通 SQL 文件不属于 CFQuery。
        let lowercase = source.to_ascii_lowercase();
        let mut cursor = 0;
        while let Some(open_relative) = find_bytes(&lowercase[cursor..], b"<cfquery") {
            let open = cursor + open_relative;
            let after_name = open + b"<cfquery".len();
            if !lowercase
                .get(after_name)
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>')
            {
                cursor = after_name;
                continue;
            }
            let Some(tag_end_relative) = lowercase[after_name..]
                .iter()
                .position(|byte| *byte == b'>')
            else {
                break;
            };
            let body_start = after_name + tag_end_relative + 1;
            let Some(close_relative) = find_bytes(&lowercase[body_start..], b"</cfquery>") else {
                break;
            };
            let body_end = body_start + close_relative;
            if body_end > body_start {
                routes.push(GrammarRoute {
                    language: "cfquery",
                    source: &source[body_start..body_end],
                    byte_offset: body_start,
                    scope: "cfquery_body",
                });
            }
            cursor = body_end + b"</cfquery>".len();
        }
    }
    routes
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
