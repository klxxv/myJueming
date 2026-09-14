//! The only application boundary shared by desktop, Agent and MCP.
use super::*;
use crate::research::registry;
use jueming_pipeline::pool::{DataPool, digest};
use jueming_protocol::{
    CapabilitySnapshot, CommandEnvelope, CommandId, CommandKind, JudgementKind, PlanV2,
    ResearchJudgement, ResearchRecord, SchemaRef, TextRange,
};

pub fn is_research_method(method: &str) -> bool {
    method == "research.call" || crate::research_method(method).is_some()
}
fn error(e: String) -> AppError {
    AppError::new("research_error", e)
}
impl LocalAppHost {
    pub fn configure_research(
        self: &Arc<Self>,
        device: PathBuf,
        bundle: PathBuf,
    ) -> Result<(), AppError> {
        let weak = Arc::downgrade(self);
        self.feature
            .configure(
                device,
                bundle,
                Arc::new(move |snapshot| {
                    if let Some(host) = weak.upgrade()
                        && let Ok(mut state) = host.lock()
                    {
                        host.publish_locked(
                            &mut state,
                            "feature_changed",
                            None,
                            "native",
                            json!(snapshot),
                        );
                    }
                }),
            )
            .map_err(error)
    }
    pub(super) fn research_call(
        &self,
        method: &str,
        params: Value,
        binding: Option<&str>,
        native: bool,
        request_id: &str,
    ) -> Result<Value, AppError> {
        if method == "research.call" {
            let method = params
                .get("method")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::invalid("method required"))?;
            let descriptor = crate::research_method(method)
                .filter(|d| !d.native_only)
                .ok_or_else(|| AppError::invalid("method is not available to Agent or MCP"))?;
            if descriptor.requires_binding {
                let state = self.lock()?;
                self.ensure_binding(&state, binding)?;
            }
            if method.starts_with("pool.") {
                return self.pool_call(
                    method,
                    params
                        .get("params")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default(),
                    binding,
                );
            }
            return self.research_call(
                method,
                params.get("params").cloned().unwrap_or(json!({})),
                binding,
                false,
                request_id,
            );
        }
        if crate::research_method(method).is_some_and(|d| d.native_only) && !native {
            return Err(AppError::new(
                "native_ui_required",
                "功能开关、方法保存和人工研究确认由桌面用户操作",
            ));
        }
        let snapshot = self.feature.snapshot().map_err(error)?;
        if method == "plugins.install_local" {
            let path = params["package_path"]
                .as_str()
                .ok_or_else(|| AppError::invalid("package_path required"))?;
            let manifest = self.feature.install_local(Path::new(path)).map_err(error)?;
            if snapshot.desired_enabled {
                self.research.cancel_all();
                self.graph.cancel_all();
                self.feature.disable().map_err(error)?;
                self.feature.enable().map_err(error)?;
            }
            return to_value(manifest);
        }
        match method {
            "plugins.remove_local" => {
                let package_id = params["package_id"]
                    .as_str()
                    .ok_or_else(|| AppError::invalid("package_id required"))?;
                self.feature.remove_local(package_id).map_err(error)?;
                self.research.cancel_all();
                self.graph.cancel_all();
                let enabled = snapshot.desired_enabled;
                self.feature.disable().map_err(error)?;
                if enabled {
                    self.feature.enable().map_err(error)?;
                }
                return Ok(json!({"removed":package_id}));
            }
            "slots.migrations" => {
                return to_value(
                    jueming_pipeline::registry::Registry::migrations().map_err(error)?,
                );
            }
            "capabilities.get" => {
                let r = self.feature.registry().map_err(error)?;
                return to_value(CapabilitySnapshot {
                    contract_version: "2.0".into(),
                    generation: snapshot.generation.clone(),
                    features: vec![snapshot],
                    slots: r.slots(),
                    operators: r.operators(),
                });
            }
            "slots.list" => {
                return to_value(self.feature.registry().map_err(error)?.slots());
            }
            "operators.list" => {
                return to_value(self.feature.registry().map_err(error)?.operators());
            }
            "schemas.list" => return to_value(registry().map_err(error)?.schemas()),
            "schemas.get" => {
                let schema: SchemaRef =
                    serde_json::from_value(params).map_err(|e| AppError::invalid(e.to_string()))?;
                return to_value(registry().map_err(error)?.schema(&schema).map_err(error)?);
            }
            "features.enable" | "features.retry_prepare" => {
                return to_value(self.feature.enable().map_err(error)?);
            }
            "features.disable" | "features.cancel_prepare" => {
                self.research.cancel_all();
                self.graph.cancel_all();
                return to_value(self.feature.disable().map_err(error)?);
            }
            "features.update_preferences" => {
                return to_value(self.feature.preferences(&params).map_err(error)?);
            }
            "pipeline.validate_plan" => {
                let plan: PlanV2 =
                    serde_json::from_value(params).map_err(|e| AppError::invalid(e.to_string()))?;
                return to_value(
                    self.feature
                        .registry()
                        .map_err(error)?
                        .compile(&plan)
                        .map_err(error)?,
                );
            }
            _ => {}
        }
        let project = {
            let state = self.lock()?;
            self.ensure_binding(&state, binding)?;
            state
                .current
                .as_ref()
                .ok_or_else(AppError::unavailable)?
                .clone()
        };
        let run_id = || {
            params
                .get("run_id")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::invalid("run_id required"))
        };
        if !native && matches!(method, "pipeline.cancel_run" | "pipeline.cancel_graph_run") {
            let owner = if method == "pipeline.cancel_run" {
                self.research
                    .get(&project.path, run_id()?)
                    .map_err(error)?
                    .owner_binding
            } else {
                self.graph
                    .run(&project.path, run_id()?)
                    .map_err(error)?
                    .owner_binding
            };
            if owner.as_deref() != binding {
                return Err(AppError::new(
                    "run_owner_mismatch",
                    "Only the initiating binding or native UI can cancel this run",
                ));
            }
        }
        match method {
            "pipeline.list_methods_v2" => to_value(self.graph.list(&project.path).map_err(error)?),
            "research.summary" => to_value(
                self.research
                    .summary(&project.path, &project.snapshot, run_id()?)
                    .map_err(error)?,
            ),
            "research.merge_groups" => {
                let command_id = CommandId::from_str(request_id)
                    .map_err(|_| AppError::invalid("request_id must be UUID"))?;
                to_value(self.execute_native_command(CommandEnvelope::new(
                    command_id,
                    project.snapshot.project.project_id,
                    canonical_base(&project.snapshot, command_id),
                    CommandKind::MergeResearchGroups,
                    params,
                ))?)
            }
            "pipeline.save_method_v2" => to_value(
                self.graph
                    .save(
                        &project.path,
                        params,
                        &self.feature.registry().map_err(error)?,
                    )
                    .map_err(error)?,
            ),
            "pipeline.start" => {
                let id = params["method_id"]
                    .as_str()
                    .ok_or_else(|| AppError::invalid("method_id required"))?;
                let revision = params["method_revision_id"]
                    .as_str()
                    .ok_or_else(|| AppError::invalid("method_revision_id required"))?;
                let method = self.graph.get(&project.path, id, revision).map_err(error)?;
                to_value(
                    self.graph
                        .start(
                            project.path,
                            project.snapshot,
                            project.writer_lock,
                            method,
                            crate::research::RunInitiator {
                                request_id: request_id.into(),
                                binding_id: binding.map(str::to_owned),
                            },
                            self.feature.clone(),
                        )
                        .map_err(error)?,
                )
            }
            "pipeline.get_graph_run" => {
                to_value(self.graph.run(&project.path, run_id()?).map_err(error)?)
            }
            "pipeline.cancel_graph_run" => {
                to_value(self.graph.cancel(&project.path, run_id()?).map_err(error)?)
            }
            "research.start" => {
                let query =
                    serde_json::from_value(params).map_err(|e| AppError::invalid(e.to_string()))?;
                to_value(
                    self.research
                        .start(
                            project.path,
                            project.snapshot,
                            project.writer_lock,
                            query,
                            crate::research::RunInitiator {
                                request_id: request_id.into(),
                                binding_id: binding.map(str::to_owned),
                            },
                            self.feature.clone(),
                        )
                        .map_err(error)?,
                )
            }
            "pipeline.list_runs" => to_value(
                self.research
                    .list(&project.path)
                    .map_err(error)?
                    .into_iter()
                    .map(|r| r.run)
                    .collect::<Vec<_>>(),
            ),
            "pipeline.get_run" => to_value(
                self.research
                    .get(&project.path, run_id()?)
                    .map_err(error)?
                    .run,
            ),
            "pipeline.cancel_run" => to_value(
                self.research
                    .cancel(&project.path, run_id()?)
                    .map_err(error)?,
            ),
            "pipeline.read_result" => to_value(
                self.research
                    .filtered_page(
                        &project.path,
                        &project.snapshot,
                        run_id()?,
                        params.get("cursor").and_then(Value::as_u64).unwrap_or(0) as usize,
                        params.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize,
                        params.get("group_name").and_then(Value::as_str),
                    )
                    .map_err(error)?,
            ),
            "pipeline.list_artifacts" => to_value(
                DataPool::new(
                    &project.path,
                    project.snapshot.project.project_id.to_string(),
                )
                .list()
                .map_err(error)?,
            ),
            "pipeline.read_artifact" => {
                let pool = DataPool::new(
                    &project.path,
                    project.snapshot.project.project_id.to_string(),
                );
                let handle = params
                    .get("handle")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AppError::invalid("handle required"))?;
                let manifest = pool.manifest(handle).map_err(error)?;
                to_value(
                    pool.read(handle, &manifest.input_revision_id)
                        .map_err(error)?,
                )
            }
            "research.confirm" => {
                let id = run_id()?;
                let occurrence_id = params
                    .get("occurrence_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AppError::invalid("occurrence_id required"))?;
                let item = self
                    .research
                    .occurrence(&project.path, &project.snapshot, id, occurrence_id)
                    .map_err(error)?;
                let run = self.research.get(&project.path, id).map_err(error)?.run;
                let kind: JudgementKind =
                    serde_json::from_value(params.get("kind").cloned().unwrap_or(Value::Null))
                        .map_err(|e| AppError::invalid(e.to_string()))?;
                let ranges: Vec<TextRange> = serde_json::from_value(
                    params.get("target_ranges").cloned().unwrap_or(json!([])),
                )
                .map_err(|e| AppError::invalid(e.to_string()))?;
                if ranges
                    .iter()
                    .any(|r| !item.targets.iter().any(|t| t.segment_id == r.segment_id))
                {
                    return Err(AppError::invalid(
                        "target range is outside this occurrence's aligned context",
                    ));
                }
                let hashes =
                    std::iter::once((item.source.segment_id, digest(item.source.text.as_bytes())))
                        .chain(
                            item.targets
                                .iter()
                                .map(|t| (t.segment_id, digest(t.text.as_bytes()))),
                        )
                        .collect();
                let source_ranges = item
                    .source
                    .ranges
                    .iter()
                    .map(|r| TextRange {
                        segment_id: item.source.segment_id,
                        start_utf8: r.start_utf8,
                        end_utf8: r.end_utf8,
                    })
                    .collect();
                let record = ResearchRecord {
                    context_fingerprint: self.kernel.research_context_fingerprint(
                        &self
                            .kernel
                            .snapshot_at_revision(
                                &project.path,
                                &project.snapshot,
                                run.input_revision_id,
                            )
                            .map_err(kernel_error)?,
                        item.source.segment_id,
                    ),
                    record_id: request_id.into(),
                    run_id: id.into(),
                    occurrence_id: occurrence_id.into(),
                    input_revision_id: run.input_revision_id,
                    source_ranges,
                    content_hashes: hashes,
                    judgement: ResearchJudgement {
                        kind,
                        group_name: if kind == JudgementKind::Omission {
                            "省译".into()
                        } else {
                            params
                                .get("group_name")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .into()
                        },
                        strategy: params
                            .get("strategy")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        target_ranges: if kind == JudgementKind::Omission {
                            Vec::new()
                        } else {
                            ranges
                        },
                    },
                };
                let command_id = CommandId::from_str(request_id).map_err(|_| {
                    AppError::invalid("research confirmation request_id must be a UUID")
                })?;
                if !project
                    .snapshot
                    .command_receipts
                    .iter()
                    .any(|r| r.command_id == command_id)
                {
                    let expected: std::collections::BTreeSet<_> =
                        item.targets.iter().map(|t| t.segment_id).collect();
                    let primary_ids: std::collections::BTreeSet<_> = project
                        .snapshot
                        .segments
                        .iter()
                        .filter(|segment| {
                            segment.document_id == project.snapshot.documents[1].document_id
                        })
                        .map(|segment| segment.segment_id)
                        .collect();
                    let current: std::collections::BTreeSet<_> = project
                        .snapshot
                        .alignments
                        .iter()
                        .filter(|a| {
                            a.source_segment_ids.contains(&item.source.segment_id)
                                && a.target_segment_ids
                                    .first()
                                    .is_some_and(|id| primary_ids.contains(id))
                        })
                        .flat_map(|a| a.target_segment_ids.iter().copied())
                        .collect();
                    if expected != current {
                        return Err(AppError::new(
                            "stale_alignment",
                            "段落对齐已变化，请重新检索后确认",
                        ));
                    }
                }
                let result = self.execute_native_command(CommandEnvelope::new(
                    command_id,
                    project.snapshot.project.project_id,
                    canonical_base(&project.snapshot, command_id),
                    CommandKind::ConfirmResearch,
                    json!(record),
                ))?;
                to_value(result)
            }
            _ => Err(AppError::new(
                "unknown_method",
                format!("Unsupported research method: {method}"),
            )),
        }
    }
}
fn canonical_base(snapshot: &ProjectSnapshot, command_id: CommandId) -> RevisionId {
    snapshot
        .command_receipts
        .iter()
        .find(|r| r.command_id == command_id)
        .map(|r| RevisionId::new(r.committed_revision_id.value().saturating_sub(1)))
        .unwrap_or(snapshot.project.current_revision_id)
}
