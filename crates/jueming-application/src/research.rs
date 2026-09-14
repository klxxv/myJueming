//! Asynchronous, revision-frozen research runs. Durable pages survive feature disable.
use crate::feature::FeatureManager;
use jueming_pipeline::{
    builtins::{self, Builtins},
    executor::{self, RunContext},
    pool::{ArtifactManifest, DataPool, digest},
    registry::Registry,
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
pub struct SavedRun {
    pub run: ResearchRun,
    pub query: ResearchQuery,
    pub request_id: String,
    pub fingerprint: String,
    pub pages: Vec<usize>,
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub owner_binding: Option<String>,
    #[serde(default)]
    pub bindings: Vec<OperatorDescriptor>,
}
pub struct RunInitiator {
    pub request_id: String,
    pub binding_id: Option<String>,
}
#[derive(Default)]
pub struct ResearchService {
    active: Mutex<BTreeMap<String, Arc<AtomicBool>>>,
    start_gate: Mutex<()>,
}
pub fn registry() -> Result<Registry, String> {
    let mut r = Registry::standard()?;
    builtins::register(&mut r)?;
    Ok(r)
}

fn run_root(project: &Path) -> PathBuf {
    project.join("extensions/jueming-pipeline/research-runs")
}
fn directory(project: &Path, id: &str) -> Result<PathBuf, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "invalid run ID")?;
    Ok(run_root(project).join(id))
}
fn save(project: &Path, record: &SavedRun) -> Result<(), String> {
    let path = directory(project, &record.run.run_id)?;
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    write_bytes_atomic(
        &path.join("run.json"),
        &serde_json::to_vec(record).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
impl ResearchService {
    pub fn get(&self, project: &Path, id: &str) -> Result<SavedRun, String> {
        let mut record: SavedRun = serde_json::from_slice(
            &std::fs::read(directory(project, id)?.join("run.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if matches!(record.run.status, RunStatus::Running | RunStatus::Queued)
            && !self
                .active
                .lock()
                .map_err(|_| "run lock poisoned")?
                .contains_key(id)
        {
            record.run.status = RunStatus::Interrupted;
            record.run.error = Some("应用退出时运行未完成；可重新检索，已确认记录保留".into());
        }
        Ok(record)
    }
    pub fn list(&self, project: &Path) -> Result<Vec<SavedRun>, String> {
        let root = run_root(project);
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if let Some(id) = entry.file_name().to_str()
                && let Ok(run) = self.get(project, id)
            {
                result.push(run);
            }
        }
        result.sort_by(|a, b| b.run.run_id.cmp(&a.run.run_id));
        Ok(result)
    }
    pub fn cancel(&self, project: &Path, id: &str) -> Result<ResearchRun, String> {
        if let Some(flag) = self.active.lock().map_err(|_| "run lock poisoned")?.get(id) {
            flag.store(true, Ordering::Release);
        }
        Ok(self.get(project, id)?.run)
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
        query: ResearchQuery,
        initiator: RunInitiator,
        feature: Arc<FeatureManager>,
    ) -> Result<ResearchRun, String> {
        let _start = self.start_gate.lock().map_err(|_| "run gate poisoned")?;
        let request_id = initiator.request_id;
        if feature.snapshot()?.status != "ready" {
            return Err("research feature is not ready".into());
        }
        if query.query.trim().is_empty() || query.query.len() > 2048 || query.max_gap > 10 {
            return Err("invalid research query; max_gap must be 0–10".into());
        }
        if !feature
            .registry()?
            .operator(&query.similarity_operator)
            .is_some_and(|operator| operator.slots.iter().any(|slot| slot == "text.similarity"))
        {
            return Err("unknown similarity provider".into());
        }
        if query.fuzzy
            && !query
                .minimum_similarity
                .or_else(|| builtins::default_similarity_threshold(&query.similarity_operator))
                .is_some_and(|v| v.is_finite() && v > 0.0 && v <= 1.0)
        {
            return Err("minimum_similarity in (0,1] required for this provider".into());
        }
        if query.method_id.is_some() {
            return Err("custom method must be submitted through the v2 graph API".into());
        }
        let fingerprint = digest(&serde_json::to_vec(&query).map_err(|e| e.to_string())?);
        for existing in self.list(&project)? {
            if existing.request_id == request_id {
                if existing.fingerprint != fingerprint
                    || existing.owner_binding != initiator.binding_id
                {
                    return Err("request ID conflict".into());
                }
                return Ok(existing.run);
            }
        }
        let mut active = self.active.lock().map_err(|_| "run lock poisoned")?;
        if active.len() >= 2 {
            return Err("two research runs are already active".into());
        }
        let run = ResearchRun {
            run_id: uuid::Uuid::now_v7().to_string(),
            project_id: snapshot.project.project_id,
            input_revision_id: snapshot.project.current_revision_id,
            status: RunStatus::Queued,
            error: None,
            total: snapshot
                .segments
                .iter()
                .filter(|s| s.document_id == snapshot.documents[0].document_id)
                .count() as u64,
            completed: 0,
        };
        let record = SavedRun {
            run: run.clone(),
            query,
            request_id,
            fingerprint,
            pages: vec![],
            artifacts: vec![],
            owner_binding: initiator.binding_id,
            bindings: feature
                .registry()?
                .operators()
                .into_iter()
                .filter(|op| !op.operator_id.starts_with("host.input."))
                .collect(),
        };
        save(&project, &record)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        active.insert(run.run_id.clone(), cancelled.clone());
        drop(active);
        let this = self.clone();
        std::thread::spawn(move || {
            let _execution_pin = jueming_pipeline::pool::pin_execution();
            let _lock = writer_lock;
            let mut record = record;
            record.run.status = RunStatus::Running;
            let _ = save(&project, &record);
            let result = run_research(&project, &snapshot, &mut record, &feature, &cancelled);
            record.run.status = if cancelled.load(Ordering::Acquire) {
                RunStatus::Cancelled
            } else if result.is_ok() {
                RunStatus::Completed
            } else {
                RunStatus::Failed
            };
            record.run.error = result.err();
            let _ = save(&project, &record);
            if let Ok(mut active) = this.active.lock() {
                active.remove(&record.run.run_id);
            }
        });
        Ok(run)
    }
    pub fn page(
        &self,
        project: &Path,
        snapshot: &ProjectSnapshot,
        id: &str,
        cursor: usize,
        limit: usize,
    ) -> Result<ResearchResultPage, String> {
        let saved = self.get(project, id)?;
        if saved.run.project_id != snapshot.project.project_id {
            return Err("run project mismatch".into());
        }
        let total: usize = saved.pages.iter().sum();
        let mut result = Vec::new();
        let mut offset = 0;
        let limit = limit.clamp(1, 200);
        for (page, count) in saved.pages.iter().enumerate() {
            if offset + count > cursor && result.len() < limit {
                let bytes =
                    std::fs::read(directory(project, id)?.join(format!("page-{page}.json")))
                        .map_err(|e| e.to_string())?;
                let items: Vec<ResearchOccurrence> =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                for mut item in items
                    .into_iter()
                    .skip(cursor.saturating_sub(offset))
                    .take(limit - result.len())
                {
                    if let Some(record) = snapshot
                        .research_records
                        .iter()
                        .find(|r| r.run_id == id && r.occurrence_id == item.occurrence_id)
                    {
                        let current = record.context_fingerprint
                            == jueming_kernel::KernelService
                                .research_context_fingerprint(snapshot, item.source.segment_id)
                            && record.content_hashes.iter().all(|(segment_id, hash)| {
                                snapshot
                                    .segments
                                    .iter()
                                    .find(|s| &s.segment_id == segment_id)
                                    .is_some_and(|s| digest(s.content.as_bytes()) == *hash)
                            });
                        item.judgement = Some(record.judgement.clone());
                        item.status = if current { "confirmed" } else { "needs_review" }.into();
                    }
                    result.push(item);
                }
            }
            offset += count;
        }
        let next = cursor + result.len();
        Ok(ResearchResultPage {
            run: saved.run,
            items: result,
            total: total as u64,
            next_cursor: (next < total).then_some(next as u64),
        })
    }
    pub fn occurrence(
        &self,
        project: &Path,
        snapshot: &ProjectSnapshot,
        id: &str,
        occurrence: &str,
    ) -> Result<ResearchOccurrence, String> {
        let mut cursor = 0;
        loop {
            let page = self.page(project, snapshot, id, cursor, 200)?;
            if let Some(item) = page
                .items
                .into_iter()
                .find(|i| i.occurrence_id == occurrence)
            {
                return Ok(item);
            }
            if let Some(next) = page.next_cursor {
                cursor = next as usize;
            } else {
                return Err("occurrence not found".into());
            }
        }
    }
    pub fn filtered_page(
        &self,
        project: &Path,
        snapshot: &ProjectSnapshot,
        id: &str,
        cursor: usize,
        limit: usize,
        group: Option<&str>,
    ) -> Result<ResearchResultPage, String> {
        let Some(group) = group else {
            return self.page(project, snapshot, id, cursor, limit);
        };
        let mut offset = 0;
        let mut total = 0;
        let mut result = Vec::new();
        let run = self.get(project, id)?.run;
        loop {
            let page = self.page(project, snapshot, id, offset, 200)?;
            for item in page.items {
                if item
                    .judgement
                    .as_ref()
                    .is_some_and(|j| j.group_name == group)
                {
                    if total >= cursor && result.len() < limit.clamp(1, 200) {
                        result.push(item);
                    }
                    total += 1;
                }
            }
            if let Some(next) = page.next_cursor {
                offset = next as usize;
            } else {
                break;
            }
        }
        let next = cursor + result.len();
        Ok(ResearchResultPage {
            run,
            items: result,
            total: total as u64,
            next_cursor: (next < total).then_some(next as u64),
        })
    }
    pub fn summary(
        &self,
        project: &Path,
        snapshot: &ProjectSnapshot,
        id: &str,
    ) -> Result<Value, String> {
        use jueming_pipeline::executor::Provider;
        let saved = self.get(project, id)?;
        let total: usize = saved.pages.iter().sum();
        let mut groups: BTreeMap<(String, String), usize> = BTreeMap::new();
        let mut confirmed = 0;
        let current_records: Vec<_> = snapshot
            .research_records
            .iter()
            .filter(|r| {
                r.run_id == id
                    && r.source_ranges.first().is_some_and(|range| {
                        r.context_fingerprint
                            == jueming_kernel::KernelService
                                .research_context_fingerprint(snapshot, range.segment_id)
                    })
                    && r.content_hashes.iter().all(|(segment_id, hash)| {
                        snapshot
                            .segments
                            .iter()
                            .find(|s| &s.segment_id == segment_id)
                            .is_some_and(|s| digest(s.content.as_bytes()) == *hash)
                    })
            })
            .collect();
        for record in &current_records {
            *groups
                .entry((
                    record.judgement.group_name.clone(),
                    record.judgement.strategy.clone(),
                ))
                .or_default() += 1;
            confirmed += 1;
        }
        struct NoPlugin;
        impl Provider for NoPlugin {
            fn execute(
                &self,
                _: &str,
                _: jueming_pipeline::executor::Values,
                _: &Value,
                _: &AtomicBool,
            ) -> Result<BTreeMap<String, Value>, String> {
                Err("not used by grouping".into())
            }
        }
        let suggestions=Builtins{plugins:&NoPlugin}.execute("builtin.translation_grouping",BTreeMap::from([("judgements".into(),vec![json!({"items":current_records.iter().map(|r|json!({"record_id":r.record_id,"group_name":r.judgement.group_name,"strategy":r.judgement.strategy,"text":""})).collect::<Vec<_>>()})])]),&json!({}),&AtomicBool::new(false))?;
        Ok(
            json!({"total":total,"confirmed":confirmed,"pending":total.saturating_sub(confirmed),"groups":groups.into_iter().map(|((name,strategy),count)|json!({"name":name,"strategy":strategy,"count":count})).collect::<Vec<_>>(),"suggestions":suggestions["suggestions"]["items"]}),
        )
    }
}
fn seed(
    pool: &DataPool,
    r: &Registry,
    run: &ResearchRun,
    name: &str,
    value: Value,
) -> Result<ArtifactManifest, String> {
    pool.publish(
        r,
        ArtifactManifest {
            handle: String::new(),
            project_id: run.project_id.to_string(),
            input_revision_id: run.input_revision_id.to_string(),
            run_id: run.run_id.clone(),
            schema: SchemaRef {
                name: name.into(),
                version: 1,
            },
            sha256: String::new(),
            bytes: 0,
            provider_id: "host.snapshot".into(),
            provider_release: "1".into(),
            dependencies: vec![],
        },
        &value,
    )
}
fn input_node(manifest: &ArtifactManifest) -> PlanNodeV2 {
    PlanNodeV2 {
        node_id: uuid::Uuid::now_v7().to_string(),
        slot_id: format!("runtime.input.{}", manifest.schema.name),
        operator_id: format!("host.input.{}", manifest.schema.name),
        inputs: BTreeMap::new(),
        config: json!({"handle":manifest.handle}),
    }
}
fn edge(node: &PlanNodeV2, port: &str) -> PortRef {
    PortRef {
        node_id: node.node_id.clone(),
        port: port.into(),
    }
}
fn node(slot: &str, operator: &str, ports: Vec<(&str, PortRef)>) -> PlanNodeV2 {
    PlanNodeV2 {
        node_id: uuid::Uuid::now_v7().to_string(),
        slot_id: slot.into(),
        operator_id: operator.into(),
        inputs: ports
            .into_iter()
            .map(|(name, p)| (name.into(), vec![p]))
            .collect(),
        config: json!({}),
    }
}
fn run_research(
    project: &Path,
    snapshot: &ProjectSnapshot,
    saved: &mut SavedRun,
    feature: &FeatureManager,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let _pin = jueming_pipeline::pool::pin_execution()?;
    let r = feature.registry()?;
    let pool = DataPool::new(project, snapshot.project.project_id.to_string());
    let provider = Builtins { plugins: feature };
    let by_id: BTreeMap<_, _> = snapshot
        .segments
        .iter()
        .map(|s| (s.segment_id, s))
        .collect();
    let source_order = snapshot
        .segment_orders
        .iter()
        .find(|o| o.document_id == snapshot.documents[0].document_id)
        .ok_or("missing source order")?;
    let source_ids: Vec<_> = source_order.entries.iter().map(|e| e.segment_id).collect();
    for batch in source_ids.chunks(16) {
        if cancelled.load(Ordering::Acquire) {
            return Err("cancelled".into());
        }
        let run = saved.run.clone();
        let project_id = run.project_id.to_string();
        let revision_id = run.input_revision_id.to_string();
        let context = || RunContext {
            run_id: &run.run_id,
            project_id: &project_id,
            revision_id: &revision_id,
            cancelled,
        };
        let segments:Vec<_>=batch.iter().filter_map(|id|by_id.get(id)).map(|s|json!({"segment_id":s.segment_id,"text":s.content,"language":snapshot.project.source_language})).collect();
        let segment_input = input_node(&seed(
            &pool,
            &r,
            &run,
            "SegmentView",
            json!({"segments":segments}),
        )?);
        let text_input = input_node(&seed(
            &pool,
            &r,
            &run,
            "TextView",
            json!({"segments":segments}),
        )?);
        let query_input = input_node(&seed(&pool, &r, &run, "QuerySpec", {
            let mut value = json!({"text":saved.query.query,"fuzzy":saved.query.fuzzy,"max_gap":saved.query.max_gap,"similarity_operator":saved.query.similarity_operator});
            if let Some(threshold) = saved.query.minimum_similarity.or_else(|| {
                builtins::default_similarity_threshold(&saved.query.similarity_operator)
            }) {
                value["minimum_similarity"] = json!(threshold);
            }
            value
        })?);
        let window_input = input_node(&seed(
            &pool,
            &r,
            &run,
            "WindowSpec",
            json!({"before":80,"after":80}),
        )?);
        let search = node(
            if saved.query.fuzzy {
                "analysis.fuzzy_search"
            } else {
                "analysis.basic_search"
            },
            if saved.query.fuzzy {
                "builtin.fuzzy_search"
            } else {
                "builtin.search"
            },
            vec![
                ("query", edge(&query_input, "value")),
                ("segments", edge(&segment_input, "value")),
            ],
        );
        let kwic = node(
            "analysis.kwic",
            "builtin.kwic",
            vec![
                ("occurrences", edge(&search, "occurrences")),
                ("text", edge(&text_input, "value")),
                ("window", edge(&window_input, "value")),
            ],
        );
        let plan = PlanV2 {
            format_version: 2,
            outputs: vec![edge(&search, "occurrences"), edge(&kwic, "kwic")],
            nodes: vec![
                segment_input,
                text_input,
                query_input,
                window_input,
                search,
                kwic,
            ],
        };
        let artifacts = executor::execute(&r, &pool, &provider, &plan, context())?;
        let occurrences = pool.read(&artifacts[0].handle, &revision_id)?;
        saved
            .artifacts
            .extend(artifacts.iter().map(|a| a.handle.clone()));
        let occurrence_items = occurrences["occurrences"]
            .as_array()
            .ok_or("missing occurrences")?;
        let mut contexts = Vec::new();
        for alignment in &snapshot.alignments {
            // This research view names the primary translation; other panels must
            // not silently contribute candidates under that translation's label.
            if !alignment
                .target_segment_ids
                .first()
                .and_then(|id| by_id.get(id))
                .is_some_and(|segment| segment.document_id == snapshot.documents[1].document_id)
            {
                continue;
            }
            if !occurrence_items.iter().any(|o| {
                alignment
                    .source_segment_ids
                    .iter()
                    .any(|id| o["source"]["segment_id"] == id.to_string())
            }) {
                continue;
            }
            let side = |ids: &Vec<SegmentId>, language: &str| {
                ids.iter()
                    .filter_map(|id| by_id.get(id))
                    .map(
                        |s| json!({"segment_id":s.segment_id,"text":s.content,"language":language}),
                    )
                    .collect::<Vec<_>>()
            };
            contexts.push(json!({"context_id":alignment.alignment_id,"sources":side(&alignment.source_segment_ids,&snapshot.project.source_language),"targets":side(&alignment.target_segment_ids,&snapshot.project.target_language)}));
        }
        let mut candidate_values = json!({"items":[]});
        if saved.query.auto_locate && !contexts.is_empty() {
            let context_input = input_node(&seed(
                &pool,
                &r,
                &run,
                "ParallelContextSet",
                json!({"contexts":contexts}),
            )?);
            let occurrence_input = input_node(&artifacts[0]);
            let align = node(
                "relation.word_alignment",
                "xlmr.contextual_alignment",
                vec![("contexts", edge(&context_input, "value"))],
            );
            let candidate = node(
                "analysis.translation_candidates",
                "builtin.translation_candidates",
                vec![
                    ("occurrences", edge(&occurrence_input, "value")),
                    ("contexts", edge(&context_input, "value")),
                    ("alignments", edge(&align, "alignments")),
                ],
            );
            let plan = PlanV2 {
                format_version: 2,
                outputs: vec![edge(&candidate, "candidates")],
                nodes: vec![context_input, occurrence_input, align, candidate],
            };
            let outputs = executor::execute(&r, &pool, &provider, &plan, context())?;
            candidate_values = pool.read(&outputs[0].handle, &revision_id)?;
            saved
                .artifacts
                .extend(outputs.into_iter().map(|a| a.handle));
        }
        let mut page = Vec::new();
        for occurrence in occurrence_items {
            let source_range: TextRange =
                serde_json::from_value(occurrence["source"].clone()).map_err(|e| e.to_string())?;
            let source = by_id
                .get(&source_range.segment_id)
                .ok_or("missing source segment")?;
            let ranges: Vec<TextRange> =
                serde_json::from_value(occurrence["matched_ranges"].clone())
                    .map_err(|e| e.to_string())?;
            let targets: Vec<ResearchTarget> = contexts
                .iter()
                .filter(|c| {
                    c["sources"].as_array().is_some_and(|sources| {
                        sources
                            .iter()
                            .any(|s| s["segment_id"] == source.segment_id.to_string())
                    })
                })
                .flat_map(|c| c["targets"].as_array().into_iter().flatten())
                .map(|t| {
                    serde_json::from_value(json!({"segment_id":t["segment_id"],"text":t["text"]}))
                })
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let candidates = candidate_values["items"]
                .as_array()
                .and_then(|items| {
                    items
                        .iter()
                        .find(|c| c["occurrence_id"] == occurrence["occurrence_id"])
                })
                .map(|c| serde_json::from_value(c["candidates"].clone()))
                .transpose()
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            page.push(ResearchOccurrence {
                alignment_coverage: Some(if !saved.query.auto_locate {
                    "not_requested".into()
                } else {
                    candidate_values["items"]
                        .as_array()
                        .and_then(|items| {
                            items
                                .iter()
                                .find(|c| c["occurrence_id"] == occurrence["occurrence_id"])
                        })
                        .and_then(|item| item["alignment_coverage"].as_str())
                        .unwrap_or(if targets.is_empty() {
                            "context_missing"
                        } else {
                            "unknown"
                        })
                        .into()
                }),
                occurrence_id: occurrence["occurrence_id"]
                    .as_str()
                    .ok_or("missing occurrence ID")?
                    .into(),
                source: ResearchSource {
                    segment_id: source.segment_id,
                    text: source.content.clone(),
                    ranges: ranges
                        .into_iter()
                        .map(|r| Utf8Range {
                            start_utf8: r.start_utf8,
                            end_utf8: r.end_utf8,
                        })
                        .collect(),
                },
                targets,
                candidates,
                status: "pending".into(),
                judgement: None,
            });
        }
        if !page.is_empty() {
            write_bytes_atomic(
                &directory(project, &run.run_id)?.join(format!("page-{}.json", saved.pages.len())),
                &serde_json::to_vec(&page).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            saved.pages.push(page.len());
        }
        saved.run.completed += batch.len() as u64;
        save(project, saved)?;
    }
    Ok(())
}
