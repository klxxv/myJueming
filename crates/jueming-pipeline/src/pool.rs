//! Immutable, schema-checked payloads. Paths are private to the host; handles are UUIDs.
use crate::registry::Registry;
use jueming_protocol::SchemaRef;
use jueming_storage::write_bytes_atomic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
static GC_GATE: std::sync::RwLock<()> = std::sync::RwLock::new(());
pub fn pin_execution() -> Result<std::sync::RwLockReadGuard<'static, ()>, String> {
    GC_GATE.read().map_err(|_| "data pool gate poisoned".into())
}

pub const MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactManifest {
    pub handle: String,
    pub project_id: String,
    pub input_revision_id: String,
    pub run_id: String,
    pub schema: SchemaRef,
    pub sha256: String,
    pub bytes: u64,
    pub provider_id: String,
    pub provider_release: String,
    pub dependencies: Vec<String>,
}
pub struct DataPool {
    root: PathBuf,
    project_id: String,
}
impl DataPool {
    pub fn cached_outputs(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<std::collections::BTreeMap<String, Value>> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let path = self
            .root
            .parent()?
            .join("executions")
            .join(format!("{key}.json"));
        let handles: std::collections::BTreeMap<String, String> =
            serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        handles
            .into_iter()
            .map(|(port, handle)| self.read(&handle, revision).ok().map(|value| (port, value)))
            .collect()
    }
    pub fn remember_outputs(
        &self,
        key: &str,
        handles: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), String> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid cache key".into());
        }
        let directory = self
            .root
            .parent()
            .ok_or("missing cache root")?
            .join("executions");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        write_bytes_atomic(
            &directory.join(format!("{key}.json")),
            &serde_json::to_vec(handles).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
    pub fn new(project: &Path, project_id: String) -> Self {
        Self {
            root: project.join("cache/pipeline-v2/payloads"),
            project_id,
        }
    }
    fn directory(&self, handle: &str) -> Result<PathBuf, String> {
        uuid::Uuid::parse_str(handle).map_err(|_| "invalid data handle")?;
        Ok(self.root.join(handle))
    }
    pub fn manifest(&self, handle: &str) -> Result<ArtifactManifest, String> {
        let data = std::fs::read(self.directory(handle)?.join("manifest.json"))
            .map_err(|e| e.to_string())?;
        let m: ArtifactManifest = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        if m.handle != handle || m.project_id != self.project_id {
            return Err("data handle scope mismatch".into());
        }
        Ok(m)
    }
    pub fn read(&self, handle: &str, revision: &str) -> Result<Value, String> {
        let _pin = pin_execution()?;
        let m = self.manifest(handle)?;
        if m.input_revision_id != revision {
            return Err("data handle revision mismatch".into());
        }
        if m.bytes > MAX_PAYLOAD_BYTES as u64 {
            return Err("payload exceeds bounded batch limit".into());
        }
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(self.directory(handle)?.join("payload.json"))
            .map_err(|e| e.to_string())?
            .take((MAX_PAYLOAD_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != m.bytes || digest(&bytes) != m.sha256 {
            return Err("payload integrity mismatch".into());
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    pub fn publish(
        &self,
        registry: &Registry,
        mut manifest: ArtifactManifest,
        value: &Value,
    ) -> Result<ArtifactManifest, String> {
        let _pin = pin_execution()?;
        registry.validate(&manifest.schema, value)?;
        if manifest.project_id != self.project_id {
            return Err("wrong project".into());
        }
        for dependency in &manifest.dependencies {
            let parent = self.manifest(dependency)?;
            if parent.input_revision_id != manifest.input_revision_id {
                return Err("mixed snapshot dependencies".into());
            }
        }
        let data = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        if data.len() > MAX_PAYLOAD_BYTES {
            return Err("split output into bounded batches".into());
        }
        manifest.handle = uuid::Uuid::now_v7().to_string();
        manifest.sha256 = digest(&data);
        manifest.bytes = data.len() as u64;
        let dir = self.directory(&manifest.handle)?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        write_bytes_atomic(&dir.join("payload.json"), &data).map_err(|e| e.to_string())?;
        // The manifest is the visibility boundary and is always published last.
        write_bytes_atomic(
            &dir.join("manifest.json"),
            &serde_json::to_vec(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(manifest)
    }
    pub fn list(&self) -> Result<Vec<ArtifactManifest>, String> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if let Some(id) = entry.file_name().to_str()
                && let Ok(m) = self.manifest(id)
            {
                result.push(m);
            }
        }
        Ok(result)
    }
    /// Caller supplies active run/view pins; only reconstructible cache is touched.
    pub fn collect(&self, pins: &BTreeSet<String>) -> Result<usize, String> {
        let _exclusive = GC_GATE
            .try_write()
            .map_err(|_| "active data views or graph executions pin the cache")?;
        let mut reachable = pins.clone();
        let mut pending: Vec<_> = pins.iter().cloned().collect();
        while let Some(id) = pending.pop() {
            for dep in self.manifest(&id)?.dependencies {
                if reachable.insert(dep.clone()) {
                    pending.push(dep);
                }
            }
        }
        let mut count = 0;
        for m in self.list()? {
            if !reachable.contains(&m.handle) {
                std::fs::remove_dir_all(self.directory(&m.handle)?).map_err(|e| e.to_string())?;
                count += 1;
            }
        }
        Ok(count)
    }
}
pub fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
