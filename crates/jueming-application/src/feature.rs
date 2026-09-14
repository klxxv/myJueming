//! Device-scoped activation, verified resources and independent worker lifecycle.
use jueming_pipeline::{
    executor::{Provider, Values},
    plugin::PluginWorker,
};
use jueming_protocol::{FeatureReason, FeatureSnapshot};
use jueming_storage::write_bytes_atomic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize)]
pub struct ResourceFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub executable: bool,
    pub url: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub protocol_version: u32,
    pub package_id: String,
    pub release: String,
    pub entrypoint: String,
    pub runtime: String,
    pub operators: Vec<String>,
    pub slots: BTreeMap<String, String>,
    #[serde(default)]
    pub files: Vec<ResourceFile>,
    #[serde(default)]
    pub config_schemas: BTreeMap<String, Value>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct BundleManifest {
    pub format_version: u32,
    pub release: String,
    pub python: String,
    pub packages: String,
    pub model: String,
    pub model_revision: String,
    pub files: Vec<ResourceFile>,
    pub plugins: Vec<PluginManifest>,
}
struct Configuration {
    device: PathBuf,
    bundle: PathBuf,
}
type Listener = Arc<dyn Fn(FeatureSnapshot) + Send + Sync>;
pub struct FeatureManager {
    state: Mutex<FeatureSnapshot>,
    config: Mutex<Option<Configuration>>,
    listener: Mutex<Option<Listener>>,
    cancellation: Mutex<Arc<AtomicBool>>,
    workers: Mutex<BTreeMap<String, Arc<Mutex<PluginWorker>>>>,
    bindings: Mutex<Vec<jueming_protocol::OperatorDescriptor>>,
    lifecycle: Mutex<()>,
    publication: Mutex<()>,
}
impl Default for FeatureManager {
    fn default() -> Self {
        Self {
            state: Mutex::new(FeatureSnapshot::default()),
            config: Mutex::new(None),
            listener: Mutex::new(None),
            cancellation: Mutex::new(Arc::new(AtomicBool::new(false))),
            workers: Mutex::new(BTreeMap::new()),
            bindings: Mutex::new(Vec::new()),
            lifecycle: Mutex::new(()),
            publication: Mutex::new(()),
        }
    }
}
impl FeatureManager {
    /// Remove a separately installed binding; retained releases keep old runs inspectable.
    pub fn remove_local(&self, package_id: &str) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().map_err(|_| "feature lock poisoned")?;
        safe_relative(package_id)?;
        let device = self
            .config
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .as_ref()
            .ok_or("feature not configured")?
            .device
            .clone();
        let ledger = device.join("research-plugins/installed.json");
        let mut installed: BTreeMap<String, PathBuf> =
            serde_json::from_slice(&std::fs::read(&ledger).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if installed.remove(package_id).is_none() {
            return Err("local package is not installed".into());
        }
        write_bytes_atomic(
            &ledger,
            &serde_json::to_vec(&installed).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
    /// Native installation uses a package directory; normal activation needs only a toggle.
    pub fn install_local(&self, source: &Path) -> Result<PluginManifest, String> {
        let _lifecycle = self.lifecycle.lock().map_err(|_| "feature lock poisoned")?;
        let source = source.canonicalize().map_err(|e| e.to_string())?;
        let bytes = std::fs::read(source.join("plugin.json")).map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("package manifest too large".into());
        }
        let plugin: PluginManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        safe_relative(&plugin.package_id)?;
        safe_relative(&plugin.release)?;
        safe_relative(&plugin.entrypoint)?;
        if plugin.protocol_version != 1
            || plugin.runtime != "python-3.12"
            || plugin.files.is_empty()
            || plugin.files.len() > 256
        {
            return Err("incompatible local package".into());
        }
        let total = plugin.files.iter().try_fold(0u64, |n, f| {
            n.checked_add(f.bytes).ok_or("package size overflow")
        })?;
        if total > 128 * 1024 * 1024 {
            return Err("local algorithm package exceeds 128 MiB".into());
        }
        if !plugin.files.iter().any(|f| f.path == plugin.entrypoint) {
            return Err("entrypoint is not integrity-covered".into());
        }
        let mut registry = crate::research::registry()?;
        for operator in &plugin.operators {
            register_plugin_operator(
                &mut registry,
                &plugin,
                operator,
                &jueming_pipeline::pool::digest(&bytes),
            )?;
        }
        let device = self
            .config
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .as_ref()
            .ok_or("feature not configured")?
            .device
            .clone();
        let directory = device
            .join("research-plugins")
            .join(&plugin.package_id)
            .join(format!("{}-{}", plugin.release, uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        for file in &plugin.files {
            let relative = safe_relative(&file.path)?;
            let path = source
                .join(&relative)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !path.starts_with(&source) || !verified(&path, file)? {
                return Err("local package integrity check failed".into());
            }
            let destination = directory.join(relative);
            std::fs::create_dir_all(destination.parent().ok_or("invalid package path")?)
                .map_err(|e| e.to_string())?;
            std::fs::copy(path, destination).map_err(|e| e.to_string())?;
        }
        write_bytes_atomic(&directory.join("plugin.json"), &bytes).map_err(|e| e.to_string())?;
        let ledger = device.join("research-plugins/installed.json");
        let mut installed: BTreeMap<String, PathBuf> = std::fs::read(&ledger)
            .ok()
            .map(|b| serde_json::from_slice(&b))
            .transpose()
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        installed.insert(plugin.package_id.clone(), directory);
        write_bytes_atomic(
            &ledger,
            &serde_json::to_vec(&installed).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(plugin)
    }
    pub fn configure(
        self: &Arc<Self>,
        device: PathBuf,
        bundle: PathBuf,
        listener: Listener,
    ) -> Result<(), String> {
        let mut config = self.config.lock().map_err(|_| "feature lock poisoned")?;
        let bundle = if bundle
            .join(std::env::consts::ARCH)
            .join("bundle.json")
            .is_file()
        {
            bundle.join(std::env::consts::ARCH)
        } else {
            bundle
        };
        if config.is_some() {
            return Err("feature manager already configured".into());
        }
        std::fs::create_dir_all(&device).map_err(|e| e.to_string())?;
        let persisted = std::fs::read(device.join("research-feature.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<FeatureSnapshot>(&bytes).ok());
        let enabled = persisted.as_ref().is_some_and(|s| s.desired_enabled);
        if let Some(mut state) = persisted {
            state.status = "disabled".into();
            state.resources_ready = false;
            state.worker_state = "stopped".into();
            *self.state.lock().map_err(|_| "feature lock poisoned")? = state;
        }
        *config = Some(Configuration { device, bundle });
        drop(config);
        *self.listener.lock().map_err(|_| "feature lock poisoned")? = Some(listener);
        if enabled {
            self.enable()?;
        }
        Ok(())
    }
    pub fn snapshot(&self) -> Result<FeatureSnapshot, String> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .clone())
    }
    pub fn registry(&self) -> Result<jueming_pipeline::registry::Registry, String> {
        let mut registry = crate::research::registry()?;
        if self.snapshot()?.status == "ready" {
            for binding in self
                .bindings
                .lock()
                .map_err(|_| "feature lock poisoned")?
                .iter()
            {
                registry.register_operator(binding.clone())?;
            }
        }
        Ok(registry)
    }
    fn update(&self, change: impl FnOnce(&mut FeatureSnapshot)) -> Result<FeatureSnapshot, String> {
        let _publication = self
            .publication
            .lock()
            .map_err(|_| "feature publication lock poisoned")?;
        let mut state = self.snapshot()?;
        change(&mut state);
        state.generation = (state.generation.parse::<u64>().unwrap_or(0) + 1).to_string();
        let snapshot = state.clone();
        if let Some(config) = self
            .config
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .as_ref()
        {
            write_bytes_atomic(
                &config.device.join("research-feature.json"),
                &serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        *self.state.lock().map_err(|_| "feature lock poisoned")? = snapshot.clone();
        if let Some(listener) = self
            .listener
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .as_ref()
        {
            listener(snapshot.clone());
        }
        Ok(snapshot)
    }
    pub fn enable(self: &Arc<Self>) -> Result<FeatureSnapshot, String> {
        let _lifecycle = self.lifecycle.lock().map_err(|_| "feature lock poisoned")?;
        let current = self.snapshot()?;
        if current.status == "ready" || current.status == "preparing" {
            return Ok(current);
        }
        if self
            .config
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .is_none()
        {
            return Err("local feature resource manager is not configured".into());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        *self
            .cancellation
            .lock()
            .map_err(|_| "feature lock poisoned")? = cancelled.clone();
        let activation = uuid::Uuid::now_v7().to_string();
        let result = self.update(|s| {
            s.desired_enabled = true;
            s.status = "preparing".into();
            s.stage = Some("checking_resources".into());
            s.reason = None;
            s.activation_id = Some(activation.clone());
        })?;
        let this = self.clone();
        std::thread::spawn(move || {
            let outcome = this.prepare(&cancelled);
            if cancelled.load(Ordering::Acquire) {
                return;
            }
            let _ = this.update(|s| {
                if s.activation_id.as_deref() != Some(&activation) {
                    return;
                }
                match outcome {
                    Ok(()) => {
                        s.status = "ready".into();
                        s.stage = None;
                        s.resources_ready = true;
                        s.worker_state = "ready".into();
                    }
                    Err(ref e) => {
                        s.status = "failed".into();
                        s.stage = None;
                        s.worker_state = "stopped".into();
                        s.reason = Some(FeatureReason {
                            code: "preparation_failed".into(),
                            message: e.clone(),
                            retryable: true,
                        });
                    }
                }
            });
        });
        Ok(result)
    }
    pub fn disable(&self) -> Result<FeatureSnapshot, String> {
        let _lifecycle = self.lifecycle.lock().map_err(|_| "feature lock poisoned")?;
        self.cancellation
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .store(true, Ordering::Release);
        self.workers
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .clear();
        self.update(|s| {
            s.desired_enabled = false;
            s.status = "disabled".into();
            s.stage = None;
            s.activation_id = None;
            s.worker_state = "stopped".into();
            s.reason = None;
        })
    }
    pub fn preferences(&self, params: &Value) -> Result<FeatureSnapshot, String> {
        let provider = params.get("default_similarity").and_then(Value::as_str);
        if provider.is_some_and(|p| p != "fuzzy.edit_distance" && p != "fuzzy.char_ngram") {
            return Err("unknown similarity provider".into());
        }
        self.update(|s| {
            if let Some(p) = provider {
                s.default_similarity = p.into();
            }
            if let Some(v) = params.get("auto_locate").and_then(Value::as_bool) {
                s.auto_locate = v;
            }
        })
    }
    fn prepare(&self, cancelled: &AtomicBool) -> Result<(), String> {
        let (bundle, device) = {
            let c = self.config.lock().map_err(|_| "feature lock poisoned")?;
            let c = c.as_ref().ok_or("feature not configured")?;
            (c.bundle.clone(), c.device.clone())
        };
        let platform_manifest = bundle.join(format!("bundle.{}.json", std::env::consts::ARCH));
        let manifest_path = if platform_manifest.is_file() {
            platform_manifest
        } else {
            bundle.join("bundle.json")
        };
        let data = std::fs::read(manifest_path).map_err(|_| {
            "安装包未包含研究资源目录；请安装完整研究版或配置官方资源清单".to_owned()
        })?;
        let manifest: BundleManifest = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        if manifest.format_version != 1 || manifest.files.is_empty() || manifest.plugins.is_empty()
        {
            return Err("incompatible resource manifest".into());
        }
        safe_relative(&manifest.release)?;
        let complete = manifest
            .files
            .iter()
            .all(|f| safe_relative(&f.path).is_ok_and(|p| bundle.join(p).is_file()));
        let root = if complete {
            bundle.clone()
        } else {
            device.join("research-resources").join(&manifest.release)
        };
        let total = manifest.files.iter().try_fold(0u64, |sum, f| {
            sum.checked_add(f.bytes).ok_or("resource size overflow")
        })?;
        let mut done = 0;
        self.update(|s| {
            s.total_bytes = Some(total.to_string());
            s.completed_bytes = Some("0".into());
        })?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| e.to_string())?;
        for file in &manifest.files {
            if cancelled.load(Ordering::Acquire) {
                return Err("cancelled".into());
            }
            let relative = safe_relative(&file.path)?;
            let target = root.join(&relative);
            if !verified_progress(&target, file, cancelled, |bytes| {
                let _ = self.update(|s| s.completed_bytes = Some((done + bytes).to_string()));
            })? {
                if complete {
                    return Err(format!("资源校验失败：{}", file.path));
                }
                std::fs::create_dir_all(target.parent().ok_or("invalid resource path")?)
                    .map_err(|e| e.to_string())?;
                let temporary = target.with_extension("preparing");
                let mut output = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
                let local = bundle.join(&relative);
                let mut source: Box<dyn Read> = if local.is_file() {
                    Box::new(std::fs::File::open(local).map_err(|e| e.to_string())?)
                } else {
                    let url = file
                        .url
                        .as_deref()
                        .filter(|url| url.starts_with("https://"))
                        .ok_or_else(|| format!("资源缺失且没有官方下载地址：{}", file.path))?;
                    self.update(|s| s.stage = Some("downloading".into()))?;
                    Box::new(
                        client
                            .get(url)
                            .send()
                            .and_then(reqwest::blocking::Response::error_for_status)
                            .map_err(|e| e.to_string())?,
                    )
                };
                let mut buffer = [0u8; 64 * 1024];
                let mut written = 0;
                let mut progress_at = std::time::Instant::now();
                loop {
                    if cancelled.load(Ordering::Acquire) {
                        let _ = std::fs::remove_file(&temporary);
                        return Err("cancelled".into());
                    }
                    let n = source.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    written += n as u64;
                    if written > file.bytes {
                        return Err("download exceeded declared size".into());
                    }
                    output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                    if progress_at.elapsed() > Duration::from_millis(750) {
                        self.update(|s| s.completed_bytes = Some((done + written).to_string()))?;
                        progress_at = std::time::Instant::now();
                    }
                }
                output.sync_all().map_err(|e| e.to_string())?;
                drop(output);
                if !verified_progress(&temporary, file, cancelled, |_| {})? {
                    let _ = std::fs::remove_file(&temporary);
                    return Err("download integrity mismatch".into());
                }
                #[cfg(unix)]
                if file.executable {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o755))
                        .map_err(|e| e.to_string())?;
                }
                std::fs::rename(temporary, &target).map_err(|e| e.to_string())?;
            }
            done += file.bytes;
            // Publish on file boundaries; no idle polling or background network checks.
            if file.bytes > 1024 * 1024 || done == total {
                self.update(|s| s.completed_bytes = Some(done.to_string()))?;
            }
        }
        self.update(|s| {
            s.stage = Some("loading_model".into());
            s.worker_state = "starting".into();
        })?;
        let python = root.join(safe_relative(&manifest.python)?);
        let packages = root.join(safe_relative(&manifest.packages)?);
        let model = root.join(safe_relative(&manifest.model)?);
        let mut workers = BTreeMap::new();
        let mut registry = crate::research::registry()?;
        let mut package_sources: BTreeMap<String, (PluginManifest, PathBuf)> = manifest
            .plugins
            .iter()
            .map(|p| {
                (
                    p.package_id.clone(),
                    (p.clone(), root.join("plugins").join(&p.package_id)),
                )
            })
            .collect();
        if let Ok(bytes) = std::fs::read(device.join("research-plugins/installed.json")) {
            let installed: BTreeMap<String, PathBuf> =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            for (id, path) in installed {
                let plugin: PluginManifest = serde_json::from_slice(
                    &std::fs::read(path.join("plugin.json")).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if plugin.package_id != id {
                    return Err("installed package identity mismatch".into());
                }
                for file in &plugin.files {
                    if !verified(&path.join(safe_relative(&file.path)?), file)? {
                        return Err("installed package integrity mismatch".into());
                    }
                }
                package_sources.insert(id, (plugin, path));
            }
        }
        let mut plugin_bindings = Vec::new();
        for (plugin, plugin_root) in package_sources.values() {
            safe_relative(&plugin.package_id)?;
            safe_relative(&plugin.entrypoint)?;
            if plugin.protocol_version != 1 {
                return Err("incompatible plugin protocol".into());
            }
            if plugin.operators.len() != plugin.slots.len() {
                return Err("plugin slot bindings do not match its operators".into());
            }
            for operator in &plugin.operators {
                let fingerprint = jueming_pipeline::pool::digest(
                    &[
                        data.as_slice(),
                        serde_json::to_vec(plugin)
                            .map_err(|e| e.to_string())?
                            .as_slice(),
                    ]
                    .concat(),
                );
                register_plugin_operator(&mut registry, plugin, operator, &fingerprint)?;
                plugin_bindings.push(
                    registry
                        .operator(operator)
                        .ok_or("plugin registration failed")?
                        .clone(),
                );
            }
            let mut worker = PluginWorker::start(
                &python,
                &packages,
                &plugin_root.join(&plugin.entrypoint),
                &model,
            )?;
            worker.handshake(&plugin.operators, cancelled)?;
            let worker = Arc::new(Mutex::new(worker));
            for operator in &plugin.operators {
                if workers.insert(operator.clone(), worker.clone()).is_some() {
                    return Err("duplicate plugin operator".into());
                }
            }
        }
        let mut active_workers = self.workers.lock().map_err(|_| "feature lock poisoned")?;
        if cancelled.load(Ordering::Acquire) {
            return Err("cancelled".into());
        }
        if [
            "fuzzy.edit_distance",
            "fuzzy.char_ngram",
            "xlmr.contextual_alignment",
        ]
        .iter()
        .any(|id| !workers.contains_key(*id))
        {
            return Err("research feature requires both fuzzy providers and XLM-R".into());
        }
        *active_workers = workers;
        *self.bindings.lock().map_err(|_| "feature lock poisoned")? = plugin_bindings;
        Ok(())
    }
}
fn register_plugin_operator(
    registry: &mut jueming_pipeline::registry::Registry,
    plugin: &PluginManifest,
    operator: &str,
    fingerprint: &str,
) -> Result<(), String> {
    let slot_id = plugin
        .slots
        .get(operator)
        .ok_or("missing plugin slot binding")?;
    let slot = registry.slot(slot_id).ok_or("unknown slot")?.clone();
    registry.register_operator(jueming_protocol::OperatorDescriptor {
        operator_id: operator.into(),
        release: format!("{}+resources.{}", plugin.release, fingerprint),
        name: operator.into(),
        slots: vec![slot.slot_id],
        inputs: slot.inputs,
        outputs: slot.outputs,
        config_schema: plugin
            .config_schemas
            .get(operator)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({"type":"object","maxProperties":0})),
    })
}
impl Provider for FeatureManager {
    fn execute(
        &self,
        operator: &str,
        inputs: Values,
        config: &Value,
        cancelled: &AtomicBool,
    ) -> Result<BTreeMap<String, Value>, String> {
        if self.snapshot()?.status != "ready" {
            return Err("research feature is not ready".into());
        }
        let worker = self
            .workers
            .lock()
            .map_err(|_| "feature lock poisoned")?
            .get(operator)
            .cloned()
            .ok_or("unbound plugin operator")?;
        let mut guard = loop {
            if cancelled.load(Ordering::Acquire) {
                return Err("cancelled".into());
            }
            match worker.try_lock() {
                Ok(guard) => break guard,
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err("worker lock poisoned".into());
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    std::thread::sleep(Duration::from_millis(25))
                }
            }
        };
        let result = guard.execute(operator, inputs, config, cancelled);
        drop(guard);
        if let Err(ref error) = result
            && error != "cancelled"
        {
            let _ = self.update(|s| {
                s.status = "failed".into();
                s.worker_state = "failed".into();
                s.reason = Some(FeatureReason {
                    code: "worker_failed".into(),
                    message: error.clone(),
                    retryable: true,
                });
            });
        }
        result
    }
}
fn safe_relative(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if p.as_os_str().is_empty() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("unsafe resource path".into());
    }
    Ok(p.into())
}
fn verified(path: &Path, file: &ResourceFile) -> Result<bool, String> {
    verified_progress(path, file, &AtomicBool::new(false), |_| {})
}
fn verified_progress(
    path: &Path,
    file: &ResourceFile,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<bool, String> {
    if !path.is_file() {
        return Ok(false);
    }
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() != file.bytes {
        return Ok(false);
    }
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    let mut input = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 64 * 1024];
    let mut consumed = 0;
    let mut last = std::time::Instant::now();
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("cancelled".into());
        }
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        consumed += n as u64;
        if last.elapsed() > Duration::from_millis(750) {
            progress(consumed);
            last = std::time::Instant::now();
        }
    }
    let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(actual == file.sha256)
}
