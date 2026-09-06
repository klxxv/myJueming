//! Generic JSON snapshot I/O; no domain or protocol dependency.

use crate::atomic_write::write_bytes_atomic;
use crate::error::StorageError;
use crate::layout::ProjectLayout;
use serde::{Serialize, de::DeserializeOwned};
use std::fs;

impl ProjectLayout {
    /// Serializes and atomically replaces the current Phase 1 snapshot.
    pub fn write_snapshot<T: Serialize>(&self, value: &T) -> Result<(), StorageError> {
        self.ensure()?;
        let bytes = serde_json::to_vec_pretty(value).map_err(|source| StorageError::Json {
            path: self.snapshot_path(),
            source,
        })?;
        write_bytes_atomic(&self.snapshot_path(), &bytes)
    }

    pub fn read_snapshot<T: DeserializeOwned>(&self) -> Result<T, StorageError> {
        let path = self.snapshot_path();
        let bytes = fs::read(&path).map_err(|source| StorageError::Io {
            path: path.clone(),
            source,
        })?;
        serde_json::from_slice(&bytes).map_err(|source| StorageError::Json { path, source })
    }

    /// Persist one complete, independently reopenable revision snapshot. The
    /// snapshot contains ordinary canonical DTOs only; it never embeds another
    /// snapshot, so history growth is linear rather than recursive.
    pub fn write_revision_snapshot<T: Serialize>(
        &self,
        revision_id: impl std::fmt::Display,
        value: &T,
    ) -> Result<(), StorageError> {
        self.ensure()?;
        let path = self.revision_snapshot_path(revision_id);
        let bytes = serde_json::to_vec_pretty(value).map_err(|source| StorageError::Json {
            path: path.clone(),
            source,
        })?;
        write_bytes_atomic(&path, &bytes)
    }

    pub fn read_revision_snapshot<T: DeserializeOwned>(
        &self,
        revision_id: impl std::fmt::Display,
    ) -> Result<T, StorageError> {
        let path = self.revision_snapshot_path(revision_id);
        let bytes = fs::read(&path).map_err(|source| StorageError::Io {
            path: path.clone(),
            source,
        })?;
        serde_json::from_slice(&bytes).map_err(|source| StorageError::Json { path, source })
    }
}
