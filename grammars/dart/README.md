# Dart grammar rebuild

The pinned CodeGraph source commit is `1072f82ce24db3d133258d30165cef6b74d108b2`. Its Dart WASM (`7f5364e4256cf7e55efd01dd52421ef2663caa8061b82659b7e4bf61064545ec`) cannot be instantiated by Rust `WasmStore`: legacy `dylink` metadata and an unresolved external scanner import prevent loading. That file remains identified by `grammars/codegraph-coverage.json` but is **not** the bundled candidate.

The checked-in C source and MIT license are byte-identical to [`UserNobody14/tree-sitter-dart@d4d8f3e337d8be23be27ffc35a0aef972343cd54`](https://github.com/UserNobody14/tree-sitter-dart/tree/d4d8f3e337d8be23be27ffc35a0aef972343cd54). They include the real `scanner.c`; no scanner stub is used. Source SHA-256 values:

| File | SHA-256 |
| --- | --- |
| `source/parser.c` | `5a42b47abb4d494f125dbdee9138979248041689b1aa36355550fa3e28dcb8b8` |
| `source/scanner.c` | `07a7b7818b175e9460523e705dd88d20f7b5141bac95c593d4426e6d52284996` |
| `source/tree_sitter/parser.h` | `180b893c8734778fd32f372dfbc27bd6ad1cd2221f26150b31256ff6716320d2` |
| `source/tree_sitter/alloc.h` | `b29c1c9fb7cc82f58c84b376df1297d6e2737a1d655fd356db0859e3c29c2fea` |
| `source/tree_sitter/array.h` | `5bdf6ed1a78e3409fd443e085ca967a64c188a5d082aaf7f819bccd53a471c94` |
| `LICENSE` | `d270cb3a4985d75033bd77d875ccebff1d66e32788a3f727891e28d76132dd46` |

`scripts/rebuild-dart-grammar.sh` uses the already installed Zig 0.16.0 and offline Cargo dependencies. It compiles `parser.c` **and** `scanner.c` to `source.wasm` (`bbb37cc6aebca30188fab912bb2ae7201604d2b001a64ea6ef2c397674f81f5c`, 989331 bytes). The Rust adapter replaces the single incompatible `__main_argc_argv` import with the same-signature `args_get` import within the import section. It verifies the exact input and output hashes, producing `parser.wasm` (`7dad281b3b24924d619cb7059a42b409e7690ebeb8ebbc82882e68167656e012`, 989323 bytes). The script compares both rebuilt files byte-for-byte to the committed assets; it does not install tools or overwrite them.

The Rust loader measures ABI 15. Its test replays all 150 cases in the pinned upstream `test/corpus` files (146 expected without parse errors and 4 expected with errors); the source fixtures are copied under `corpus/` from the same commit. Additional focused examples cover strings, interpolation, documentation comments, and block comments. Isolated worker output remains `incomplete` with `grammar_qualified=false`. Independent language-version corpus, native Dart analyzer comparison, false-positive measurement, public native-first `lint dart` routing, host feedback, and release package acceptance are still required.
