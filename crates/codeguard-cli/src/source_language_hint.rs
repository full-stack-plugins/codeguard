//! 只读语言候选证据；供默认发现与可选 WASM 路由共用。

/// 识别 `.m` 源码行首的 Objective-C 专属词法标记。
/// 参数为有界原始源码；返回值仅是语言候选证据，不证明语法或项目配置。
pub(crate) fn has_objc_marker(source: &[u8]) -> bool {
    let markers: &[&[u8]] = &[
        b"@interface".as_slice(),
        b"@implementation",
        b"@protocol",
        b"@class",
        b"@property",
        b"@synthesize",
        b"@autoreleasepool",
        b"#import",
    ];
    let mut matlab_block_comment = false;
    source.split(|byte| *byte == b'\n').any(|line| {
        let line = line.trim_ascii_start();
        if matlab_block_comment {
            if line.starts_with(b"%}") {
                matlab_block_comment = false;
            }
            return false;
        }
        if line.starts_with(b"%{") {
            matlab_block_comment = true;
            return false;
        }
        markers.iter().any(|marker| {
            line.starts_with(marker)
                && line.get(marker.len()).is_some_and(|next| {
                    next.is_ascii_whitespace() || matches!(next, b'(' | b'{' | b'<' | b'"')
                })
        })
    })
}
