//! CodeGuard专业报告到共享协议的双读适配；不签发工程通过或授权。
mod feedback_java;
mod feedback_native;
mod feedback_schema;
mod projection_checker_pin;
mod projection_context;
mod projection_java_execution;
mod projection_native_rule;
mod projection_result;
mod projection_run;
mod projection_source_proof;
mod provider;
mod schema_sources;
pub use projection_checker_pin::ProjectionCheckerPin;
pub use projection_context::ProjectionContext;
pub use projection_native_rule::ProjectionNativeRule;
pub use projection_result::ProjectionResult;
pub use projection_run::ProjectionRun;
pub use provider::project_raw;
#[cfg(test)]
mod tests;

pub use codeguard_core::UnboundNativeDiagnostic;
pub use projection_source_proof::ProjectionSourceProof;
