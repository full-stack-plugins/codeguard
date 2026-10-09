//! Opt-in, read-only native evidence boundary. No profile currently qualifies complete scope.
pub mod profile;
pub mod reader;
pub mod scope;
mod scope_budget;

pub mod projection;

pub mod envelope;

#[cfg(unix)]
pub mod command;

pub mod ruff_profile;
