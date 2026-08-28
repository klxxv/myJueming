//! Local project layout, operation log, and storage adapters.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use tempfile::Builder;
use thiserror::Error;

const SNAPSHOT_FILE: &str = "project.json";

/// The user-visible `.jm` directory and its stable child paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLayout {
    root: PathBuf,
}

impl ProjectLayout {
    /// Creates a layout handle without touching the filesystem.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        let is_jm = root
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("jm"));
        if !is_jm {
            return Err(StorageError::InvalidProjectRoot(root));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn snapshot_path(&self) -> PathBuf {
        self.root.join(SNAPSHOT_FILE)
    }

    pub fn revision_snapshot_path(&self, revision_id: impl std::fmt::Display) -> PathBuf {
        self.revisions_dir().join(format!("{revision_id}.json"))
    }

    pub fn revisions_dir(&self) -> PathBuf {
        self.root.join("revisions")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// Creates the minimal Phase 1 directory layout. Existing content is preserved.
    pub fn ensure(&self) -> Result<(), StorageError> {
        for path in [
            self.root.clone(),
            self.revisions_dir(),
            self.assets_dir(),
            self.cache_dir(),
        ] {
            fs::create_dir_all(&path).map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
        }
        Ok(())
    }

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

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("project directory must use the .jm extension: {0}")]
    InvalidProjectRoot(PathBuf),
    #[error("path has no parent directory: {0}")]
    MissingParent(PathBuf),
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid project JSON at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Snapshot {
        revision: u64,
        text: String,
    }

    #[test]
    fn rejects_non_project_directory() {
        let error = ProjectLayout::new("example").expect_err("extension is required");
        assert!(matches!(error, StorageError::InvalidProjectRoot(_)));
    }

    #[test]
    fn snapshot_round_trip_and_replace_are_atomic() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let layout = ProjectLayout::new(temporary.path().join("sample.jm")).expect("layout");

        layout
            .write_snapshot(&Snapshot {
                revision: 1,
                text: "first".into(),
            })
            .expect("first write");
        layout
            .write_snapshot(&Snapshot {
                revision: 2,
                text: "second".into(),
            })
            .expect("replacement write");

        let restored: Snapshot = layout.read_snapshot().expect("read snapshot");
        assert_eq!(
            restored,
            Snapshot {
                revision: 2,
                text: "second".into(),
            }
        );
        let temporary_files = fs::read_dir(layout.root())
            .expect("list project")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temporary_files, 0);
    }

    #[test]
    fn cache_cleanup_never_touches_snapshot_or_revisions() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let layout = ProjectLayout::new(temporary.path().join("sample.jm")).expect("layout");
        layout
            .write_snapshot(&Snapshot {
                revision: 2,
                text: "canonical".into(),
            })
            .expect("write canonical snapshot");
        layout
            .write_revision_snapshot(
                2,
                &Snapshot {
                    revision: 2,
                    text: "history".into(),
                },
            )
            .expect("write revision snapshot");
        fs::create_dir_all(layout.cache_dir().join("search")).expect("cache folder");
        fs::write(
            layout.cache_dir().join("search/index.bin"),
            b"derived-cache",
        )
        .expect("cache file");

        assert_eq!(layout.clear_cache().expect("clear cache"), 13);
        assert!(layout.cache_dir().is_dir());
        assert_eq!(
            fs::read_dir(layout.cache_dir())
                .expect("empty cache")
                .count(),
            0
        );
        assert!(layout.snapshot_path().is_file());
        assert!(layout.revision_snapshot_path(2).is_file());
    }
}
