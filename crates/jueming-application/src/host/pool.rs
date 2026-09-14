//! Pull-based snapshot views. Cursor is iteration state; identities remain opaque IDs.
use super::*;
use jueming_pipeline::pool::{ArtifactManifest, DataPool};
use jueming_protocol::SchemaRef;
#[derive(Serialize, Deserialize)]
struct ViewLease {
    view_id: String,
    project_id: ProjectId,
    revision_id: RevisionId,
    owner_binding: String,
    schema: SchemaRef,
    side: String,
    closed: bool,
    handles: Vec<String>,
}
fn error(message: impl ToString) -> AppError {
    AppError::new("pool_error", message.to_string())
}
fn lease_path(project: &Path, id: &str) -> Result<PathBuf, AppError> {
    uuid::Uuid::parse_str(id).map_err(error)?;
    Ok(project
        .join("cache/pipeline-v2/views")
        .join(format!("{id}.json")))
}
fn save(project: &Path, view: &ViewLease) -> Result<(), AppError> {
    let path = lease_path(project, &view.view_id)?;
    std::fs::create_dir_all(path.parent().ok_or_else(|| error("invalid view path"))?)
        .map_err(error)?;
    write_bytes_atomic(&path, &serde_json::to_vec(view).map_err(error)?).map_err(error)
}
impl LocalAppHost {
    pub(super) fn pool_call(
        &self,
        method: &str,
        params: serde_json::Map<String, Value>,
        binding: Option<&str>,
    ) -> Result<Value, AppError> {
        let project = {
            let state = self.lock()?;
            self.ensure_binding(&state, binding)?;
            state
                .current
                .as_ref()
                .ok_or_else(AppError::unavailable)?
                .clone()
        };
        let binding = binding.ok_or_else(|| error("binding required"))?;
        if method == "pool.collect_cache" {
            let pool = DataPool::new(
                &project.path,
                project.snapshot.project.project_id.to_string(),
            );
            let mut pins = std::collections::BTreeSet::new();
            let view_root = project.path.join("cache/pipeline-v2/views");
            let live_bindings: std::collections::HashSet<_> =
                self.lock()?.bindings.keys().cloned().collect();
            if view_root.exists() {
                for entry in std::fs::read_dir(&view_root).map_err(error)? {
                    let path = entry.map_err(error)?.path();
                    let view: ViewLease =
                        serde_json::from_slice(&std::fs::read(path).map_err(error)?)
                            .map_err(error)?;
                    if !view.closed && live_bindings.contains(&view.owner_binding) {
                        pins.extend(view.handles);
                    }
                }
            }
            for run in self.research.list(&project.path).map_err(error)? {
                pins.extend(run.artifacts);
            }
            let graph_root = project.path.join("extensions/jueming-pipeline/v2");
            let runs = graph_root.join("runs");
            if runs.exists() {
                for entry in std::fs::read_dir(runs).map_err(error)? {
                    let run: crate::graph::GraphRun = serde_json::from_slice(
                        &std::fs::read(entry.map_err(error)?.path()).map_err(error)?,
                    )
                    .map_err(error)?;
                    pins.extend(run.artifacts.into_iter().map(|m| m.handle));
                }
            }
            let methods = graph_root.join("methods");
            if methods.exists() {
                for entry in std::fs::read_dir(methods).map_err(error)? {
                    for file in std::fs::read_dir(entry.map_err(error)?.path()).map_err(error)? {
                        let method: crate::graph::GraphMethod = serde_json::from_slice(
                            &std::fs::read(file.map_err(error)?.path()).map_err(error)?,
                        )
                        .map_err(error)?;
                        pins.extend(method.plan.nodes.into_iter().filter_map(|n| {
                            n.config
                                .get("handle")
                                .and_then(Value::as_str)
                                .map(str::to_owned)
                        }));
                    }
                }
            }
            return Ok(json!({"removed_payloads":pool.collect(&pins).map_err(error)?}));
        }
        let _pin = jueming_pipeline::pool::pin_execution().map_err(error)?;
        if method == "pool.read_index_partition" {
            let pool = DataPool::new(
                &project.path,
                project.snapshot.project.project_id.to_string(),
            );
            let handle = params
                .get("handle")
                .and_then(Value::as_str)
                .ok_or_else(|| error("index handle required"))?;
            let manifest = pool.manifest(handle).map_err(error)?;
            if !["BasicIndexRef", "LexicalIndexRef"].contains(&manifest.schema.name.as_str())
                || manifest.schema.version != 1
            {
                return Err(error("handle does not refer to a supported index"));
            }
            let value = pool
                .read(handle, &manifest.input_revision_id)
                .map_err(error)?;
            self.feature
                .registry()
                .map_err(error)?
                .validate(&manifest.schema, &value)
                .map_err(error)?;
            let term = params.get("term").and_then(Value::as_str);
            let cursor = params.get("cursor").and_then(Value::as_u64).unwrap_or(0) as usize;
            let limit = params
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(100)
                .clamp(1, 200) as usize;
            let postings: Vec<_> = value["postings"]
                .as_array()
                .ok_or_else(|| error("missing index postings"))?
                .iter()
                .filter(|p| term.is_none_or(|t| p["term"].as_str() == Some(t)))
                .collect();
            let total = postings.len();
            let end = cursor.saturating_add(limit).min(total);
            return Ok(
                json!({"manifest":manifest,"items":postings.into_iter().skip(cursor).take(limit).collect::<Vec<_>>(),"total":total,"next_cursor":if end<total {Some(end)}else{None}}),
            );
        }
        if method == "pool.open_view" {
            let schema: SchemaRef = serde_json::from_value(
                params
                    .get("schema")
                    .cloned()
                    .unwrap_or(json!({"name":"SegmentView","version":1})),
            )
            .map_err(error)?;
            if !["SegmentView", "TextView"].contains(&schema.name.as_str()) || schema.version != 1 {
                return Err(error("view requires SegmentView or TextView schema 1"));
            }
            let side = params
                .get("side")
                .and_then(Value::as_str)
                .unwrap_or("source");
            if !["source", "target"].contains(&side) {
                return Err(error("unknown view side"));
            }
            let revision_id = params
                .get("revision_id")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(error)?
                .unwrap_or(project.snapshot.project.current_revision_id);
            self.kernel
                .snapshot_at_revision(&project.path, &project.snapshot, revision_id)
                .map_err(kernel_error)?;
            let view = ViewLease {
                view_id: uuid::Uuid::now_v7().to_string(),
                project_id: project.snapshot.project.project_id,
                revision_id,
                owner_binding: binding.into(),
                schema,
                side: side.into(),
                closed: false,
                handles: vec![],
            };
            save(&project.path, &view)?;
            return to_value(view);
        }
        let id = params
            .get("view_id")
            .and_then(Value::as_str)
            .ok_or_else(|| error("view_id required"))?;
        let mut view: ViewLease =
            serde_json::from_slice(&std::fs::read(lease_path(&project.path, id)?).map_err(error)?)
                .map_err(error)?;
        if view.project_id != project.snapshot.project.project_id
            || view.owner_binding != binding
            || view.closed
        {
            return Err(error("view is closed or belongs to another binding"));
        }
        if method == "pool.close_view" {
            view.closed = true;
            save(&project.path, &view)?;
            return Ok(json!({"closed":true}));
        }
        let snapshot = self
            .kernel
            .snapshot_at_revision(&project.path, &project.snapshot, view.revision_id)
            .map_err(kernel_error)?;
        let field = |name: &str| {
            params
                .get(name)
                .and_then(Value::as_str)
                .ok_or_else(|| error(format!("{name} required")))
        };
        match method {
            "pool.read_segment" => {
                let segment_id = SegmentId::from_str(field("segment_id")?).map_err(error)?;
                to_value(
                    snapshot
                        .segments
                        .iter()
                        .find(|s| s.segment_id == segment_id)
                        .ok_or_else(|| error("segment not found in snapshot"))?,
                )
            }
            "pool.read_alignment" => {
                let alignment_id = AlignmentId::from_str(field("alignment_id")?).map_err(error)?;
                to_value(
                    snapshot
                        .alignments
                        .iter()
                        .find(|a| a.alignment_id == alignment_id)
                        .ok_or_else(|| error("alignment not found in snapshot"))?,
                )
            }
            "pool.read_annotation" => {
                let annotation_id =
                    jueming_protocol::AnnotationId::from_str(field("annotation_id")?)
                        .map_err(error)?;
                to_value(
                    snapshot
                        .annotations
                        .iter()
                        .find(|a| a.annotation_id == annotation_id)
                        .ok_or_else(|| error("annotation not found in snapshot"))?,
                )
            }
            "pool.read_batch" | "pool.read_slice" => {
                let cursor = params.get("cursor").and_then(Value::as_u64).unwrap_or(0) as usize;
                let limit = params
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(100)
                    .clamp(1, 200) as usize;
                let data = crate::graph::view_value(
                    &snapshot,
                    &view.schema.name,
                    &view.side,
                    cursor,
                    limit,
                )
                .map_err(error)?;
                let count = data["segments"].as_array().map_or(0, Vec::len);
                let side_index = if view.side == "source" { 0 } else { 1 };
                let total = snapshot
                    .segments
                    .iter()
                    .filter(|s| s.document_id == snapshot.documents[side_index].document_id)
                    .count();
                let pool = DataPool::new(&project.path, view.project_id.to_string());
                let manifest = pool
                    .publish(
                        &self.feature.registry().map_err(error)?,
                        ArtifactManifest {
                            handle: String::new(),
                            project_id: view.project_id.to_string(),
                            input_revision_id: view.revision_id.to_string(),
                            run_id: view.view_id.clone(),
                            schema: view.schema.clone(),
                            sha256: String::new(),
                            bytes: 0,
                            provider_id: "host.snapshot".into(),
                            provider_release: "1".into(),
                            dependencies: vec![],
                        },
                        &data,
                    )
                    .map_err(error)?;
                view.handles.push(manifest.handle.clone());
                save(&project.path, &view)?;
                Ok(
                    json!({"manifest":manifest,"data":data,"total":total,"next_cursor":(cursor+count<total).then_some(cursor+count)}),
                )
            }
            _ => Err(error("unknown pool method")),
        }
    }
}
