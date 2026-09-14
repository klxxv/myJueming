//! OS-owned locks: files remain in place; the OS releases ownership on exit.
use crate::{ProjectLayout, StorageError};
use std::fs::{File, OpenOptions};

impl ProjectLayout {
    pub fn acquire_writer_lock(&self) -> Result<File, StorageError> {
        self.acquire_lock(".writer.lock")
    }

    pub fn acquire_commit_lock(&self) -> Result<File, StorageError> {
        self.acquire_lock(".commit.lock")
    }

    fn acquire_lock(&self, name: &str) -> Result<File, StorageError> {
        self.ensure()?;
        let path = self.root().join(name);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
        file.try_lock()
            .map_err(|_| StorageError::ProjectLocked(self.root().to_path_buf()))?;
        Ok(file)
    }
}
