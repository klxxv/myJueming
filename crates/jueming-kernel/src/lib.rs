//! Command/query facade for the local Jueming Kernel.
//!
//! Feature modules implement the stateless service. Shared validation,
//! revision advancement, persistence and sidecar migrations remain internal.

mod commands;
mod error;
mod export;
mod history;
mod import;
mod persistence;
mod project;
mod projection;
mod revision;
mod search;
mod sidecar;
mod validation;

pub use error::KernelError;
pub use validation::validate_snapshot;

/// Stable public facade; feature modules do not own separate canonical state.
#[derive(Debug, Default, Clone, Copy)]
pub struct KernelService;

#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod test_support;
