//! Durable sibling-temp-file replacement shared by snapshots and exports.

use crate::error::StorageError;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::Builder;

/// Writes a sibling temporary file, flushes it, then atomically replaces `path`.
pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let parent = path
        .parent()
        .ok_or_else(|| StorageError::MissingParent(path.to_path_buf()))?;
    fs::create_dir_all(parent).map_err(|source| StorageError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    let mut temporary = Builder::new()
        .prefix(".jueming-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|source| StorageError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    temporary
        .write_all(bytes)
        .and_then(|()| temporary.flush())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|source| StorageError::Io {
            path: temporary.path().to_path_buf(),
            source,
        })?;

    temporary.persist(path).map_err(|error| StorageError::Io {
        path: path.to_path_buf(),
        source: error.error,
    })?;
    Ok(())
}
