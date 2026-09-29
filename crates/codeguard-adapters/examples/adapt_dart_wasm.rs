//! 固定 Dart 重建资产的离线发行字节生成器。

use std::path::Path;

use codeguard_adapters::adapt_dart_wasm;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let [_, source, output] = args.as_slice() else {
        return Err("用法：adapt_dart_wasm <source.wasm> <parser.wasm>".into());
    };
    let source = std::fs::read(Path::new(source))?;
    let adapted = adapt_dart_wasm(&source)?;
    std::fs::write(Path::new(output), adapted)?;
    Ok(())
}
