//! C11/C++17独立Clang入口的预处理上下文防护，不替代编译器词法/语法诊断。

/// 判断有界冻结源码中是否存在可能影响外部上下文的行首预处理指令。
/// 参数为最多1MiB源码及是否C++17；返回true时原生入口须保持上下文未解析。
/// 字符串/字符/注释中的井号不是指令；C++原始字符串保留其原始换行与反斜杠。
#[must_use]
pub fn has_c_family_preprocessor_directive(source: &[u8], cpp17: bool) -> bool {
    if source.len() > 1024 * 1024 {
        return true;
    }
    let (bytes, offsets) = logical_bytes(source, !cpp17);
    let mut i = 0;
    let mut line_start = true;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' | b'\r' => {
                line_start = true;
                i += 1;
            }
            b' ' | b'\t' | b'\x0b' | b'\x0c' => {
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < bytes.len() {
                    if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                        i += 2;
                        break;
                    }
                    if matches!(bytes[i], b'\n' | b'\r') {
                        line_start = true;
                    }
                    i += 1;
                }
            }
            b'#' if line_start => {
                return true;
            }
            b'%' if line_start && bytes.get(i + 1) == Some(&b':') => {
                return true;
            }
            b'R' if cpp17 && bytes.get(i + 1) == Some(&b'"') => {
                // 阶段2的拼接在原始字符串双引号内撤销：从原字节识别分隔符和真正闭合点。
                let Some(end) = raw_string_end(source, offsets[i + 1] + 1) else {
                    return true;
                };
                while i < offsets.len() && offsets[i] < end {
                    i += 1;
                }
                line_start = false;
            }
            b'"' | b'\'' => {
                let quote = bytes[i];
                line_start = false;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(bytes.len());
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else if matches!(bytes[i], b'\n' | b'\r') {
                        break;
                    } else {
                        i += 1;
                    }
                }
            }
            byte if line_start && !byte.is_ascii() => {
                // 固定Clang可将Unicode空白或非法起始字符恢复为空白；不据此执行未知指令。
                // 仅保留行首不确定性，字符串/注释与已有ASCII token仍由各自分支处理。
                i += 1;
            }
            _ => {
                line_start = false;
                i += 1;
            }
        }
    }
    false
}

fn logical_bytes(source: &[u8], trigraphs: bool) -> (Vec<u8>, Vec<usize>) {
    let mut bytes = Vec::with_capacity(source.len());
    let mut offsets = Vec::with_capacity(source.len());
    let mut i = if source.starts_with(&[0xef, 0xbb, 0xbf]) {
        3
    } else {
        0
    };
    while i < source.len() {
        let (byte, width) = if trigraphs && source.get(i..i + 2) == Some(b"??") {
            match source.get(i + 2) {
                Some(b'=') => (b'#', 3),
                Some(b'/') => (b'\\', 3),
                Some(b'\'') => (b'^', 3),
                Some(b'(') => (b'[', 3),
                Some(b')') => (b']', 3),
                Some(b'!') => (b'|', 3),
                Some(b'<') => (b'{', 3),
                Some(b'>') => (b'}', 3),
                Some(b'-') => (b'~', 3),
                _ => (source[i], 1),
            }
        } else {
            (source[i], 1)
        };
        if byte == b'\\' {
            let mut after = i + width;
            // Apple Clang21接受反斜杠与换行之间的水平空白扩展；警告交给原工具。
            while matches!(source.get(after), Some(b' ' | b'\t' | b'\x0b' | b'\x0c')) {
                after += 1;
            }
            let splice = match source.get(after) {
                Some(b'\n') if source.get(after + 1) == Some(&b'\r') => 2,
                Some(b'\r') if source.get(after + 1) == Some(&b'\n') => 2,
                Some(b'\n' | b'\r') => 1,
                _ => 0,
            };
            if splice != 0 {
                i = after + splice;
                continue;
            }
        }
        bytes.push(byte);
        offsets.push(i);
        i += width;
    }
    (bytes, offsets)
}

fn raw_string_end(source: &[u8], start: usize) -> Option<usize> {
    let mut open = start;
    while open < source.len() && source[open] != b'(' {
        if open - start >= 16
            || !source[open].is_ascii()
            || matches!(
                source[open],
                b' ' | b'\t' | b'\x0b' | b'\x0c' | b'\n' | b'\r' | b'\\' | b')'
            )
        {
            return None;
        }
        open += 1;
    }
    if source.get(open) != Some(&b'(') {
        return None;
    }
    let delimiter = &source[start..open];
    let mut i = open + 1;
    while i < source.len() {
        if source[i] == b')'
            && source.get(i + 1..i + 1 + delimiter.len()) == Some(delimiter)
            && source.get(i + 1 + delimiter.len()) == Some(&b'"')
        {
            return Some(i + 2 + delimiter.len());
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::has_c_family_preprocessor_directive;
    #[test]
    fn literal_hashes_and_comment_directives_are_not_context_dependencies() {
        for source in [
            "const char *s=\"#include \\\"x.h\\\"\";\n",
            "int x='#'; /* #include */\n",
            "/*\n#include \"x.h\"\n*/ int x;\n",
            "// literal hash #include\nint x;\n",
            "// continued \\\n#include \"x.h\"\nint x;\n",
        ] {
            for cpp in [false, true] {
                assert!(
                    !has_c_family_preprocessor_directive(source.as_bytes(), cpp),
                    "{source}"
                );
            }
        }
    }
    #[test]
    fn line_start_directives_preserve_comments_splices_and_alternate_tokens() {
        for source in [
            "#include \"x.h\"\n",
            "/* context */ #define X 1\n",
            "int x; /*\n*/ #include \"x.h\"\n",
            "%:include \"x.h\"\n",
            "%\\\n:include \"x.h\"\n",
            "\\\r\n#include \"x.h\"\n",
        ] {
            for cpp in [false, true] {
                assert!(
                    has_c_family_preprocessor_directive(source.as_bytes(), cpp),
                    "{source}"
                );
            }
        }
        assert!(has_c_family_preprocessor_directive(
            b"??=include \"x.h\"\n",
            false
        ));
        assert!(!has_c_family_preprocessor_directive(
            b"??=include \"x.h\"\n",
            true
        ));
        assert!(!has_c_family_preprocessor_directive(
            b"// continued ??/\n#include \"x.h\"\n",
            false
        ));
        assert!(has_c_family_preprocessor_directive(
            b"// not continued ??/\n#include \"x.h\"\n",
            true
        ));
    }
    #[test]
    fn cpp_raw_strings_use_original_bytes_and_resume_directive_detection_after_end() {
        for source in [
            "const char *s=R\"cg(\n#include \"x.h\"\n)cg\";\n",
            "const char *s=u8R\"cg(\n)cg\\\n\"\n#include \"x.h\"\n)cg\";\n",
            "const char *s=R\\\n\"cg(\n#include \"x.h\"\n)cg\";\n",
        ] {
            assert!(
                !has_c_family_preprocessor_directive(source.as_bytes(), true),
                "{source}"
            );
            assert!(has_c_family_preprocessor_directive(
                format!("{source}#include \"real.h\"\n").as_bytes(),
                true
            ));
        }
        assert!(has_c_family_preprocessor_directive(
            b"R\"invalid\\delimiter(foo)invalid\"",
            true
        ));
    }
    #[test]
    fn clang_bom_unicode_recovery_and_space_splices_keep_directives_unresolved() {
        for source in [
            "\u{feff}#error CODEGUARD_PP_PROBE\n",
            "\u{a0}#error CODEGUARD_PP_PROBE\n",
            "\u{200b}#error CODEGUARD_PP_PROBE\n",
            "%\\ \n:error CODEGUARD_PP_PROBE\n",
            "%\\\t\r\n:error CODEGUARD_PP_PROBE\n",
        ] {
            for cpp in [false, true] {
                assert!(
                    has_c_family_preprocessor_directive(source.as_bytes(), cpp),
                    "{source}"
                );
            }
        }
        for cpp in [false, true] {
            assert!(!has_c_family_preprocessor_directive(
                b"// continued \\ \n#error CODEGUARD_PP_PROBE\nint x;\n",
                cpp
            ));
            assert!(!has_c_family_preprocessor_directive(
                "const char *s=\"中文#literal\";\n".as_bytes(),
                cpp
            ));
        }
    }
    #[test]
    fn clang_cr_and_lfcr_splices_cannot_hide_digraph_directives() {
        for source in [
            b"%\\\r:error X\r".as_slice(),
            b"%\\\n\r:error X\n",
            b"%\\ \r:error X\r",
        ] {
            for cpp in [false, true] {
                assert!(has_c_family_preprocessor_directive(source, cpp));
            }
        }
    }
    #[test]
    fn input_budget_is_enforced_before_allocating_translation_buffers() {
        assert!(has_c_family_preprocessor_directive(
            &vec![b' '; 1024 * 1024 + 1],
            true
        ));
        assert!(!has_c_family_preprocessor_directive(
            &vec![b' '; 1024 * 1024],
            true
        ));
    }
}
