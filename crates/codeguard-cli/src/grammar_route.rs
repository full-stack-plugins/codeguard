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
        while let Some(open) = next_cfquery_open(&lowercase, cursor) {
            let after_name = open + b"<cfquery".len();
            if !lowercase
                .get(after_name)
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>')
            {
                cursor = after_name;
                continue;
            }
            let Some(tag_end) = find_tag_end(&lowercase, after_name) else {
                break;
            };
            let body_start = tag_end + 1;
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

// 只跳过由服务器移除的 CFML 注释；普通 HTML 注释中的 CFML 标签仍会执行。
fn next_cfquery_open(source: &[u8], mut cursor: usize) -> Option<usize> {
    while cursor < source.len() {
        let open = cursor + find_bytes(&source[cursor..], b"<")?;
        if source[open..].starts_with(b"<!---") {
            cursor = skip_nested_cfml_comment(source, open)?;
            continue;
        }
        if source[open..].starts_with(b"<!--") {
            cursor = open + 4;
            continue;
        }
        let after_name = open + b"<cfquery".len();
        if source[open..].starts_with(b"<cfquery")
            && source
                .get(after_name)
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>')
        {
            return Some(open);
        }
        let after_script = open + b"<cfscript".len();
        if source[open..].starts_with(b"<cfscript")
            && source
                .get(after_script)
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>')
        {
            let body_start = find_tag_end(source, after_script)? + 1;
            cursor = skip_closed_span(source, body_start, b"</cfscript>")?;
            continue;
        }
        if source
            .get(open + 1)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'/' | b'!' | b'?'))
        {
            cursor = find_tag_end(source, open + 1)? + 1;
        } else {
            cursor = open + 1;
        }
    }
    None
}

fn skip_closed_span(source: &[u8], start: usize, close: &[u8]) -> Option<usize> {
    Some(start + find_bytes(&source[start..], close)? + close.len())
}

fn skip_nested_cfml_comment(source: &[u8], open: usize) -> Option<usize> {
    let mut cursor = open + b"<!---".len();
    let mut depth = 1;
    loop {
        let nested = find_bytes(&source[cursor..], b"<!---").map(|offset| cursor + offset);
        let close = cursor + find_bytes(&source[cursor..], b"--->")?;
        if let Some(nested) = nested.filter(|nested| *nested < close) {
            depth += 1;
            cursor = nested + b"<!---".len();
        } else {
            depth -= 1;
            cursor = close + b"--->".len();
            if depth == 0 {
                return Some(cursor);
            }
        }
    }
}

fn find_tag_end(source: &[u8], start: usize) -> Option<usize> {
    let mut quote = None;
    let mut index = start;
    while index < source.len() {
        let byte = source[index];
        if quote == Some(byte) {
            quote = None;
        } else if quote.is_none() {
            if source[index..].starts_with(b"<!---") {
                index = skip_nested_cfml_comment(source, index)?;
                continue;
            }
            if matches!(byte, b'\'' | b'"') {
                quote = Some(byte);
            } else if byte == b'>' {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}
