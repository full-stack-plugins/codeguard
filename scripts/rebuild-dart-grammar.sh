#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dart_source="$repo_root/grammars/dart/source"
if [[ "$(zig version)" != "0.16.0" ]]; then
  printf '%s\n' 'Dart grammar 重建要求 Zig 0.16.0' >&2
  exit 2
fi

scratch_dir="$(mktemp -d)"
trap 'rm -rf "$scratch_dir"' EXIT
zig cc -target wasm32-wasi -Os -shared -fPIC -fno-exceptions \
  -Wl,--no-entry -Wl,--export=tree_sitter_dart \
  -I "$dart_source" "$dart_source/parser.c" "$dart_source/scanner.c" \
  -o "$scratch_dir/source.wasm"
cmp "$scratch_dir/source.wasm" "$repo_root/grammars/dart/source.wasm"

cd "$repo_root"
cargo run --locked --offline -p codeguard-adapters --example adapt_dart_wasm -- \
  "$scratch_dir/source.wasm" "$scratch_dir/parser.wasm"
cmp "$scratch_dir/parser.wasm" "$repo_root/grammars/dart/parser.wasm"
printf '%s\n' 'Dart grammar 原始与适配后 WASM 均可重复构建'
