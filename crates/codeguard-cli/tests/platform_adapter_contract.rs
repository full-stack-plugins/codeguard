//! Portable tests compile the actual internal adapter modules without importing
//! unrelated Unix-only shell fixtures from the CLI library unit-test target.
mod platform_fs {
    pub use codeguard_runtime::platform_fs::*;
}
#[path = "../src/doctor_scratch.rs"]
mod doctor_scratch;
#[path = "../src/ruff_tool_selection.rs"]
mod ruff_tool_selection;
#[path = "../src/tool_platform.rs"]
mod tool_platform;
