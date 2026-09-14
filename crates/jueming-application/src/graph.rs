//! Versioned v2 methods and asynchronous generic graph execution.
use crate::feature::FeatureManager;
use jueming_pipeline::{
    builtins::Builtins,
    executor::{RunContext, execute_observed},
    pool::{ArtifactManifest, DataPool},
};
use jueming_protocol::*;
use jueming_storage::write_bytes_atomic;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Clone, Serialize, Deserialize)]
pub struct GraphMethod {
    pub method_id: String,
    pub name: String,
    pub method_revision_id: String,
    pub plan: PlanV2,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct GraphRun {
    #[serde(flatten)]
    pub run: ResearchRun,
    pub method: GraphMethod,
    pub artifacts: Vec<ArtifactManifest>,
    pub request_id: String,
    #[serde(default)]
    pub owner_binding: Option<String>,
    #[serde(default)]
    pub bindings: Vec<OperatorDescriptor>,
    #[serde(default)]
    pub nodes: BTreeMap<String, String>,
}
#[derive(Default)]
pub struct GraphService {
    active: Mutex<BTreeMap<String, Arc<AtomicBool>>>,
    method_lock: Mutex<()>,
    start_gate: Mutex<()>,
}
fn root(project: &Path) -> PathBuf {
    project.join("extensions/jueming-pipeline/v2")
}
fn uuid(id: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| "invalid opaque UUID".into())
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn write(path: &Path, value: &impl Serialize) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("path missing parent")?)
        .map_err(|e| e.to_string())?;
    write_bytes_atomic(path, &serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
impl GraphService {
    pub fn list(&self, project: &Path) -> Result<Vec<GraphMethod>, String> {
        let path = root(project).join("methods");
        if !path.exists() {
            return Ok(vec![]);
        }
        let mut items = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if let Ok(m) = read(&entry.path().join("head.json")) {
                items.push(m);
            }
        }
        Ok(items)
    }
    pub fn get(&self, project: &Path, id: &str, revision: &str) -> Result<GraphMethod, String> {
        uuid(id)?;
        uuid(revision)?;
        read(
            &root(project)
                .join("methods")
                .join(id)
                .join(format!("{revision}.json")),
        )
    }
    pub fn save(
        &self,
        project: &Path,
        params: Value,
        registry: &jueming_pipeline::registry::Registry,
    ) -> Result<GraphMethod, String> {
        let _guard = self
            .method_lock
            .lock()
            .map_err(|_| "method lock poisoned")?;
        let plan: PlanV2 =
            serde_json::from_value(params["plan"].clone()).map_err(|e| e.to_string())?;
        registry.compile(&plan)?;
        let name = params["name"]
            .as_str()
            .filter(|n| !n.trim().is_empty() && n.len() <= 256)
            .ok_or("method name required")?;
        let id = params["method_id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        uuid(&id)?;
        let dir = root(project).join("methods").join(&id);
        if dir.join("head.json").exists() {
            let head: GraphMethod = read(&dir.join("head.json"))?;
            if params["base_method_revision_id"].as_str() != Some(&head.method_revision_id) {
                return Err("method revision changed; reload before saving".into());
            }
        } else if params
            .get("base_method_revision_id")
            .is_some_and(|v| !v.is_null())
        {
            return Err("method does not exist".into());
        }
        let method = GraphMethod {
            method_id: id,
            name: name.into(),
            method_revision_id: uuid::Uuid::now_v7().to_string(),
            plan,
        };
        write(
            &dir.join(format!("{}.json", method.method_revision_id)),
            &method,
        )?;
        write(&dir.join("head.json"), &method)?;
        Ok(method)
    }
    pub fn run(&self, project: &Path, id: &str) -> Result<GraphRun, String> {
        uuid(id)?;
        let mut run: GraphRun = read(&root(project).join("runs").join(format!("{id}.json")))?;
        if matches!(run.run.status, RunStatus::Queued | RunStatus::Running)
            && !self
                .active
                .lock()
                .map_err(|_| "graph lock poisoned")?
                .contains_key(id)
        {
            run.run.status = RunStatus::Interrupted;
            run.run.error = Some("Application exited before graph completed".into());
        }
        Ok(run)
    }
    pub fn cancel(&self, project: &Path, id: &str) -> Result<GraphRun, String> {
        if let Some(flag) = self
            .active
            .lock()
            .map_err(|_| "graph lock poisoned")?
            .get(id)
        {
            flag.store(true, Ordering::Release);
        }
        self.run(project, id)
    }
    pub fn cancel_all(&self) {
        if let Ok(active) = self.active.lock() {
            for flag in active.values() {
                flag.store(true, Ordering::Release);
            }
        }
    }
    pub fn start(
        self: &Arc<Self>,
        project: PathBuf,
        snapshot: ProjectSnapshot,
        writer_lock: Arc<std::fs::File>,
        method: GraphMethod,
        initiator: crate::research::RunInitiator,
        feature: Arc<FeatureManager>,
    ) -> Result<GraphRun, String> {
        let _start = self.start_gate.lock().map_err(|_| "graph gate poisoned")?;
        let request_id = initiator.request_id;
        let r = feature.registry()?;
        r.compile(&method.plan)?;
        let run_dir = root(&project).join("runs");
        if run_dir.exists() {
            for entry in std::fs::read_dir(&run_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if let Ok(existing) = read::<GraphRun>(&entry.path())
                    && existing.request_id == request_id
                {
                    if existing.method.method_revision_id != method.method_revision_id
                        || existing.owner_binding != initiator.binding_id
                    {
                        return Err("request ID conflict".into());
                    }
                    return Ok(existing);
                }
            }
        }
        let mut active = self.active.lock().map_err(|_| "graph lock poisoned")?;
        if active.len() >= 2 {
            return Err("graph queue full".into());
        }
        let run = GraphRun {
            nodes: method
                .plan
                .nodes
                .iter()
                .map(|n| (n.node_id.clone(), "queued".into()))
                .collect(),
            bindings: method
                .plan
                .nodes
                .iter()
                .filter_map(|n| r.operator(&n.operator_id).cloned())
                .collect(),
            owner_binding: initiator.binding_id,
            run: ResearchRun {
                run_id: uuid::Uuid::now_v7().to_string(),
                project_id: snapshot.project.project_id,
                input_revision_id: snapshot.project.current_revision_id,
                status: RunStatus::Queued,
                error: None,
                total: method.plan.nodes.len() as u64,
                completed: 0,
            },
            method,
            artifacts: vec![],
            request_id,
        };
        let filename = run_dir.join(format!("{}.json", run.run.run_id));
        write(&filename, &run)?;
        let flag = Arc::new(AtomicBool::new(false));
        active.insert(run.run.run_id.clone(), flag.clone());
        drop(active);
        let this = self.clone();
        let mut saved = run.clone();
        std::thread::spawn(move || {
            let _execution_pin = jueming_pipeline::pool::pin_execution();
            let _lock = writer_lock;
            saved.run.status = RunStatus::Running;
            let _ = write(&filename, &saved);
            let pool = DataPool::new(&project, snapshot.project.project_id.to_string());
            let result = (|| {
                let _pin = jueming_pipeline::pool::pin_execution()?;
                let mut plan = saved.method.plan.clone();
                for node in &mut plan.nodes {
                    if node.operator_id.starts_with("host.input.")
                        && node.config.get("handle").is_none()
                    {
                        let schema = r
                            .operator(&node.operator_id)
                            .ok_or("unknown input operator")?
                            .outputs[0]
                            .schemas[0]
                            .clone();
                        let value = if let Some(value) = node.config.get("value") {
                            value.clone()
                        } else {
                            view_value(
                                &snapshot,
                                &schema.name,
                                node.config["view"].as_str().ok_or("missing view")?,
                                0,
                                usize::MAX,
                            )?
                        };
                        let m = pool.publish(
                            &r,
                            ArtifactManifest {
                                handle: String::new(),
                                project_id: saved.run.project_id.to_string(),
                                input_revision_id: saved.run.input_revision_id.to_string(),
                                run_id: saved.run.run_id.clone(),
                                schema,
                                sha256: String::new(),
                                bytes: 0,
                                provider_id: "host.snapshot".into(),
                                provider_release: "1".into(),
                                dependencies: vec![],
                            },
                            &value,
                        )?;
                        node.config = json!({"handle":m.handle});
                    }
                }
                let run_id = saved.run.run_id.clone();
                let project_id = saved.run.project_id.to_string();
                let revision_id = saved.run.input_revision_id.to_string();
                execute_observed(
                    &r,
                    &pool,
                    &Builtins {
                        plugins: feature.as_ref(),
                    },
                    &plan,
                    RunContext {
                        run_id: &run_id,
                        project_id: &project_id,
                        revision_id: &revision_id,
                        cancelled: &flag,
                    },
                    |node, completed, artifacts| {
                        saved.nodes.insert(
                            node.into(),
                            if completed { "completed" } else { "running" }.into(),
                        );
                        saved.run.completed = saved
                            .nodes
                            .values()
                            .filter(|s| s.as_str() == "completed")
                            .count() as u64;
                        saved.artifacts.extend_from_slice(artifacts);
                        write(&filename, &saved)
                    },
                )
            })();
            match result {
                Ok(artifacts) => {
                    saved.artifacts = artifacts;
                    saved.run.status = RunStatus::Completed;
                    saved.run.completed = saved.run.total;
                }
                Err(error) => {
                    saved.run.status = if flag.load(Ordering::Acquire) {
                        RunStatus::Cancelled
                    } else {
                        RunStatus::Failed
                    };
                    saved.run.error = Some(error);
                    for state in saved.nodes.values_mut().filter(|s| s.as_str() == "running") {
                        *state = if flag.load(Ordering::Acquire) {
                            "cancelled"
                        } else {
                            "failed"
                        }
                        .into();
                    }
                }
            }
            let _ = write(&filename, &saved);
            if let Ok(mut active) = this.active.lock() {
                active.remove(&saved.run.run_id);
            }
        });
        Ok(run)
    }
}
pub fn view_value(
    snapshot: &ProjectSnapshot,
    schema: &str,
    side: &str,
    cursor: usize,
    limit: usize,
) -> Result<Value, String> {
    if !["SegmentView", "TextView"].contains(&schema) {
        return Err("source/target views require SegmentView or TextView; other schemas use typed value inputs".into());
    }
    let index = match side {
        "source" => 0,
        "target" => 1,
        _ => return Err("unknown view".into()),
    };
    let document = &snapshot.documents[index];
    let order = snapshot
        .segment_orders
        .iter()
        .find(|o| o.document_id == document.document_id)
        .ok_or("missing segment order")?;
    let by_id: BTreeMap<_, _> = snapshot
        .segments
        .iter()
        .map(|s| (s.segment_id, s))
        .collect();
    Ok(
        json!({"segments":order.entries.iter().skip(cursor).take(limit).filter_map(|e|by_id.get(&e.segment_id)).map(|s|json!({"segment_id":s.segment_id,"text":s.content,"language":document.language_id})).collect::<Vec<_>>()}),
    )
}
