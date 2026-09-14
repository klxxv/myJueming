//! The single authoritative in-process application host.
//!
//! Transports and Tauri commands never retain their own project snapshot. They
//! enter this host so project locking, revision publication and proposal
//! journalling occur in one serial critical section.

mod host;
mod types;

pub use host::LocalAppHost;
pub use types::{AgentCall, AgentReply, AppError, AppEvent, ContextSnapshot, SearchSpec};
mod feature;
mod graph;
mod methods;
mod research;
pub use methods::{MethodDescriptor, RESEARCH_METHODS, research_method};
