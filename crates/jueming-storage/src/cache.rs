//! Derived-cache cleanup, separate from canonical snapshot persistence.

use crate::error::StorageError;
use crate::layout::ProjectLayout;
use std::fs;
use std::path::Path;

impl ProjectLayout {
    /// Removes only the derived cache tree and recreates its empty root.
    /// Canonical project and revision files are deliberately out of scope.
    pub fn clear_cache(&self) -> Result<u64, StorageError> {
        self.ensure()?;
        let cache = self.cache_dir();
        let removed_bytes = directory_size(&cache)?;
        fs::remove_dir_all(&cache).map_err(|source| StorageError::Io {
            path: cache.clone(),
            source,
        })?;
        fs::create_dir_all(&cache).map_err(|source| StorageError::Io {
            path: cache,
            source,
        })?;
        Ok(removed_bytes)
    }
}

fn directory_size(path: &Path) -> Result<u64, StorageError> {
    let mut total = 0;
    for entry in fs::read_dir(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|source| StorageError::Io {
            path: entry_path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            total += directory_size(&entry_path)?;
        } else if file_type.is_file() {
            total += entry
                .metadata()
                .map_err(|source| StorageError::Io {
                    path: entry_path,
                    source,
                })?
                .len();
        }
    }
    Ok(total)
}
