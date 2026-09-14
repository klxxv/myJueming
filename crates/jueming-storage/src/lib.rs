//! Local project layout and domain-independent storage adapters.

mod atomic_write;
mod cache;
mod error;
mod layout;
mod lock;
mod snapshot;

pub use atomic_write::write_bytes_atomic;
pub use error::StorageError;
pub use layout::ProjectLayout;
