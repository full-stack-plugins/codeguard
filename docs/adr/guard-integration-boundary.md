# Guard integration boundary, local slice

The initial boundary is a pure Rust library module that consumes bounded bytes and a separately captured invocation descriptor. Native command dispatch is unchanged. The future explicit entry is `codeguard guard-project --invocation <file> --native-report <file>`; this slice does not expose that command or promise its output. It will require frozen binding, protected mapping and pinned GuardEngine revision before publication.

This slice registers only native run_report schema 1.0, lint with JSON output, package 0.1.4. The existing strict run_report parser remains the native format authority. This is an intentionally narrow compatibility reader, not qualification for native scope completeness. Raw bytes, SHA-256, native findings and exit semantics remain available. Caller-provided invocation identity is an integrity cross-check, not provenance authentication. No filesystem reads, tool execution, policy approvals, or source mutation occur in the reader.

SDK integration is preferred over process invocation: reuse the fixed shared engine crate once its contract is frozen. No engine dependency is installed or asserted pinned yet. Production release, envelope publication, export-failure feedback format support, query/hook/legacy compatibility and authenticated consumption remain separate gates.
