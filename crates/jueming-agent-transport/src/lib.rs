//! Authenticated, loopback-only adapters for the local Jueming application host.
//!
//! This crate is deliberately transport-only. It never opens a `.jm` project and every
//! request is forwarded to the already-running [`jueming_application::LocalAppHost`].

mod server;

pub use server::{
    AuthToken, ServerConfig, ServerHandle, ServerStatus, TransportError, TransportState,
    start_server,
};
