#[path = "../../src/check_request.rs"]
pub mod check_request;
#[path = "../../src/engineering_projection/mod.rs"]
pub mod engineering_projection;
#[path = "../../src/run_report.rs"]
pub mod run_report;

#[cfg(test)]
#[path = "../../src/check_native_results.rs"]
mod check_native_results;
#[cfg(test)]
#[path = "../../src/check_report_assembly.rs"]
mod check_report_assembly;

#[cfg(test)]
#[path = "../../src/java_p3c_task_recheck.rs"]
mod java_p3c_task_recheck;
