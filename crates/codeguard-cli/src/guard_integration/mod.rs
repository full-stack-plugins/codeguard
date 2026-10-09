//! Opt-in, read-only evidence boundary. Each profile covers only its documented native scope.
pub mod profile;
pub mod reader;
pub mod scope;
mod scope_budget;

pub mod projection;

pub mod envelope;

#[cfg(unix)]
pub mod command;

pub mod ruff_profile;

#[cfg(unix)]
pub mod ruff_command;

pub mod shadow;

pub mod consumer;

pub mod audit;
