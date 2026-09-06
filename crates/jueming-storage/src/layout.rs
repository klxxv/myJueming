//! Validated .jm directory layout and stable child paths.

use crate::error::StorageError;
use std::fs;
use std::path::{Path, PathBuf};

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
}
