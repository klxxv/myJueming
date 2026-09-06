use super::*;
use jueming_pipeline::{
    CancellationToken, CreatePipelineMethod, ExecutePipelineRequest, PipelineArtifactId,
    PipelineError, PipelineMethod, PipelineMethodId, PipelineMethodRevisionId, PipelineNode,
    PipelineNodeId, PipelinePlanSnapshot, PipelineService, UpdatePipelineMethod,
};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};

const PIPELINE_PROPOSAL_FORMAT: &str = "1.0";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PipelineProposalStatus {
    Pending,
    Applying,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PipelineMethodProposal {
    proposal_id: String,
    project_id: String,
    status: PipelineProposalStatus,
    base_canonical_revision_id: RevisionId,
    method_id: PipelineMethodId,
    base_method_revision_id: PipelineMethodRevisionId,
    before: PipelineMethod,
    request: UpdatePipelineMethod,
    /// A host-generated UUIDv7 written with the proposal before approval.
    /// It is never accepted from the proposal caller and is the sole durable
    /// identity of an interrupted method commit.
    #[serde(default)]
    reserved_method_revision_id: Option<PipelineMethodRevisionId>,
    #[serde(default)]
    approved_method_revision_id: Option<PipelineMethodRevisionId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PipelineProposalJournal {
    format_version: String,
    project_id: String,
    proposals: Vec<PipelineMethodProposal>,
}

#[derive(Clone)]
struct PipelineRun {
    method_id: PipelineMethodId,
    initiating_binding_id: String,
    cancellation: CancellationToken,
}

static PIPELINE_RUNS: OnceLock<StdMutex<HashMap<(ProjectId, String), PipelineRun>>> =
    OnceLock::new();
static PIPELINE_PROJECT_LOCKS: OnceLock<StdMutex<HashMap<PathBuf, Arc<StdMutex<()>>>>> =
    OnceLock::new();

impl LocalAppHost {
    /// Pipeline methods are derived-data operations. This gateway owns binding,
    /// source snapshot and proposal validation; the service never sees caller
    /// supplied segment text or canonical write capability.
    pub fn pipeline_call(
        &self,
        method: &str,
        params: serde_json::Map<String, Value>,
        binding_id: Option<&str>,
        trusted_native: bool,
    ) -> Result<Value, AppError> {
        match method {
            "pipeline.list" => self.pipeline_list(binding_id),
            "pipeline.get" => self.pipeline_get(binding_id, params),
            "pipeline.create_default" => {
                self.require_pipeline_native(trusted_native, "create Pipeline methods")?;
                self.pipeline_create_default(binding_id, params)
            }
            "pipeline.update" => {
                self.require_pipeline_native(trusted_native, "update Pipeline methods")?;
                self.pipeline_update(binding_id, params)
            }
            "pipeline.preview_update" => self.pipeline_preview_update(binding_id, params),
            "pipeline.approve_update" => {
                self.require_pipeline_native(trusted_native, "approve Pipeline method proposals")?;
                self.pipeline_approve_update(binding_id, params)
            }
            "pipeline.reject_update" => self.pipeline_reject_update(binding_id, params),
            "pipeline.list_proposals" => self.pipeline_list_proposals(binding_id),
            "pipeline.execute" => self.pipeline_execute(binding_id, params),
            "pipeline.artifacts" => self.pipeline_artifacts(binding_id, params),
            "pipeline.cancel" => self.pipeline_cancel(binding_id, params),
            _ => Err(AppError::new(
                "unknown_method",
                format!("Unsupported Pipeline method: {method}"),
            )),
        }
    }

    fn require_pipeline_native(&self, trusted_native: bool, action: &str) -> Result<(), AppError> {
        if trusted_native {
            Ok(())
        } else {
            Err(AppError::new(
                "native_approval_required",
                format!("Trusted native UI is required to {action}."),
            ))
        }
    }

    fn pipeline_list(&self, binding_id: Option<&str>) -> Result<Value, AppError> {
        let project = self.pipeline_bound_project(binding_id)?;
        to_value(
            pipeline_service(&project.path)?
                .list_methods()
                .map_err(pipeline_error)?,
        )
    }

    fn pipeline_get(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            method_id: PipelineMethodId,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        to_value(
            pipeline_service(&project.path)?
                .load_method(input.method_id)
                .map_err(pipeline_error)?,
        )
    }

    fn pipeline_create_default(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            name: String,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        let method = {
            let project_lock = pipeline_project_lock(&project.path)?;
            let _guard = lock_pipeline_project(&project_lock)?;
            pipeline_service(&project.path)?
                .create_method(CreatePipelineMethod {
                    name: input.name,
                    plan: default_plan(),
                })
                .map_err(pipeline_error)?
        };
        self.publish_pipeline_changed(binding_id, "native", &project, method.method_id, "created")?;
        to_value(method)
    }

    fn pipeline_update(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            request: UpdatePipelineMethod,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        let method = {
            let project_lock = pipeline_project_lock(&project.path)?;
            let _guard = lock_pipeline_project(&project_lock)?;
            pipeline_service(&project.path)?
                .update_method(input.request)
                .map_err(pipeline_error)?
        };
        self.publish_pipeline_changed(binding_id, "native", &project, method.method_id, "updated")?;
        to_value(method)
    }

    fn pipeline_preview_update(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            request: UpdatePipelineMethod,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        let proposal = {
            let project_lock = pipeline_project_lock(&project.path)?;
            let _guard = lock_pipeline_project(&project_lock)?;
            let service = pipeline_service(&project.path)?;
            let before = service
                .load_method(input.request.method_id)
                .map_err(pipeline_error)?;
            if before.current.method_revision_id != input.request.base_method_revision_id {
                return Err(AppError::new(
                    "stale_method_revision",
                    "The method changed; reload it before preparing an update proposal.",
                ));
            }
            let proposal = PipelineMethodProposal {
                proposal_id: jueming_protocol::OperationId::new().to_string(),
                project_id: project.project_id.to_string(),
                status: PipelineProposalStatus::Pending,
                base_canonical_revision_id: project.revision_id,
                method_id: input.request.method_id,
                base_method_revision_id: input.request.base_method_revision_id,
                before,
                request: input.request,
                reserved_method_revision_id: Some(PipelineMethodRevisionId::new()),
                approved_method_revision_id: None,
            };
            let mut proposals = read_pipeline_proposals(&project)?;
            proposals.push(proposal.clone());
            persist_pipeline_proposals(&project, &proposals)?;
            proposal
        };
        self.publish_pipeline_changed(
            binding_id,
            "agent",
            &project,
            proposal.method_id,
            "proposal_created",
        )?;
        to_value(proposal)
    }

    fn pipeline_approve_update(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            proposal_id: String,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        let proposal = {
            let project_lock = pipeline_project_lock(&project.path)?;
            let _guard = lock_pipeline_project(&project_lock)?;
            let mut proposals = read_pipeline_proposals(&project)?;
            let index = proposals
                .iter()
                .position(|proposal| proposal.proposal_id == input.proposal_id)
                .ok_or_else(|| {
                    AppError::new("proposal_not_found", "Pipeline proposal is unavailable.")
                })?;
            if proposals[index].status == PipelineProposalStatus::Approved {
                return to_value(proposals[index].clone());
            }
            if proposals[index].status != PipelineProposalStatus::Pending {
                return Err(AppError::new(
                    "proposal_not_pending",
                    "Only a pending Pipeline proposal can be approved.",
                ));
            }
            if proposals[index].base_canonical_revision_id != project.revision_id {
                return Err(AppError::new(
                    "stale_revision",
                    "The Pipeline proposal was reviewed against an earlier project revision.",
                ));
            }
            let service = pipeline_service(&project.path)?;
            let current = service
                .load_method(proposals[index].method_id)
                .map_err(pipeline_error)?;
            if current.current.method_revision_id != proposals[index].base_method_revision_id {
                return Err(AppError::new(
                    "stale_method_revision",
                    "The Pipeline method changed after this proposal was reviewed.",
                ));
            }
            if !self.pipeline_revision_is_current(&project)? {
                return Err(AppError::new(
                    "stale_revision",
                    "The project changed while the Pipeline proposal was being approved.",
                ));
            }
            // Journals written before reservation support remain compatible,
            // but cannot be inferred as approved. Persist their reservation
            // before entering the service commit window.
            if proposals[index].reserved_method_revision_id.is_none() {
                proposals[index].reserved_method_revision_id =
                    Some(PipelineMethodRevisionId::new());
                persist_pipeline_proposals(&project, &proposals)?;
            }
            let reserved_method_revision_id = proposals[index]
                .reserved_method_revision_id
                .expect("reservation was persisted before approval");
            proposals[index].status = PipelineProposalStatus::Applying;
            persist_pipeline_proposals(&project, &proposals)?;
            let updated = service
                .update_method_with_revision_id(
                    proposals[index].request.clone(),
                    reserved_method_revision_id,
                )
                .map_err(pipeline_error)?;
            proposals[index].status = PipelineProposalStatus::Approved;
            proposals[index].approved_method_revision_id = Some(reserved_method_revision_id);
            debug_assert_eq!(
                updated.current.method_revision_id,
                reserved_method_revision_id
            );
            persist_pipeline_proposals(&project, &proposals)?;
            proposals[index].clone()
        };
        self.publish_pipeline_changed(
            binding_id,
            "native",
            &project,
            proposal.method_id,
            "proposal_approved",
        )?;
        to_value(proposal)
    }

    fn pipeline_reject_update(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            proposal_id: String,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        let proposal = {
            let project_lock = pipeline_project_lock(&project.path)?;
            let _guard = lock_pipeline_project(&project_lock)?;
            let mut proposals = read_pipeline_proposals(&project)?;
            let index = proposals
                .iter()
                .position(|proposal| proposal.proposal_id == input.proposal_id)
                .ok_or_else(|| {
                    AppError::new("proposal_not_found", "Pipeline proposal is unavailable.")
                })?;
            if proposals[index].status == PipelineProposalStatus::Pending {
                proposals[index].status = PipelineProposalStatus::Rejected;
                persist_pipeline_proposals(&project, &proposals)?;
            }
            proposals[index].clone()
        };
        self.publish_pipeline_changed(
            binding_id,
            "agent",
            &project,
            proposal.method_id,
            "proposal_rejected",
        )?;
        to_value(proposal)
    }

    fn pipeline_list_proposals(&self, binding_id: Option<&str>) -> Result<Value, AppError> {
        let project = self.pipeline_bound_project(binding_id)?;
        let proposals = {
            let project_lock = pipeline_project_lock(&project.path)?;
            let _guard = lock_pipeline_project(&project_lock)?;
            read_pipeline_proposals(&project)?
        };
        to_value(proposals)
    }

    fn pipeline_execute(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            method_id: PipelineMethodId,
            #[serde(default)]
            method_revision_id: Option<PipelineMethodRevisionId>,
            segment_id: SegmentId,
            base_revision_id: RevisionId,
            #[serde(default)]
            operation_id: Option<String>,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        if input.base_revision_id != project.revision_id {
            return Err(AppError::new(
                "stale_revision",
                "The supplied Pipeline input revision is no longer current; rebind first.",
            ));
        }
        let source_content = self.pipeline_source_content(&project, input.segment_id)?;
        let initiating_binding_id = binding_id
            .ok_or_else(|| {
                AppError::new("binding_required", "Pipeline execution requires a binding.")
            })?
            .to_owned();
        let operation_id = input
            .operation_id
            .filter(|operation_id| !operation_id.trim().is_empty())
            .unwrap_or_else(|| jueming_protocol::OperationId::new().to_string());
        let cancellation = CancellationToken::new();
        let mut runs = pipeline_runs().lock().map_err(|_| {
            AppError::new("host_unavailable", "Pipeline run registry is unavailable.")
        })?;
        let run_key = (project.project_id, operation_id.clone());
        if runs.contains_key(&run_key) {
            return Err(AppError::new(
                "operation_conflict",
                "A Pipeline operation already uses this operation_id.",
            ));
        }
        runs.insert(
            run_key,
            PipelineRun {
                method_id: input.method_id,
                initiating_binding_id,
                cancellation: cancellation.clone(),
            },
        );
        drop(runs);
        self.pipeline_operation_changed(binding_id, &operation_id, "running")?;

        let result = pipeline_service(&project.path).and_then(|service| {
            service
                .execute(
                    ExecutePipelineRequest {
                        method_id: input.method_id,
                        method_revision_id: input.method_revision_id,
                        input_revision_id: input.base_revision_id,
                        segment_id: input.segment_id,
                        source_content,
                    },
                    &cancellation,
                )
                .map_err(pipeline_error)
        });
        pipeline_runs()
            .lock()
            .map_err(|_| {
                AppError::new("host_unavailable", "Pipeline run registry is unavailable.")
            })?
            .remove(&(project.project_id, operation_id.clone()));

        match result {
            Err(app_error) => {
                let status = if app_error.code == "operation_cancelled" {
                    "cancelled"
                } else {
                    "failed"
                };
                self.pipeline_operation_changed(binding_id, &operation_id, status)?;
                Err(app_error)
            }
            Ok(execution) => {
                if !self.pipeline_revision_is_current(&project)? {
                    self.pipeline_operation_changed(binding_id, &operation_id, "stale")?;
                    return Err(AppError::new(
                        "stale_revision",
                        "The project changed while the Pipeline ran; review the historical artifact before reuse.",
                    ));
                }
                self.pipeline_operation_changed(binding_id, &operation_id, "completed")?;
                self.publish_pipeline_changed(
                    binding_id,
                    "agent",
                    &project,
                    execution.artifact.method_id,
                    "executed",
                )?;
                Ok(json!({"operation_id": operation_id, "artifact": execution.artifact}))
            }
        }
    }

    fn pipeline_artifacts(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            #[serde(default)]
            artifact_id: Option<PipelineArtifactId>,
            #[serde(default)]
            input_revision_id: Option<RevisionId>,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        let service = pipeline_service(&project.path)?;
        match (input.artifact_id, input.input_revision_id) {
            (Some(artifact_id), None) => {
                to_value(service.load_artifact(artifact_id).map_err(pipeline_error)?)
            }
            (None, Some(revision_id)) => to_value(
                service
                    .list_artifacts_for_revision(revision_id)
                    .map_err(pipeline_error)?,
            ),
            _ => Err(AppError::invalid(
                "pipeline.artifacts requires exactly one of artifact_id or input_revision_id",
            )),
        }
    }

    fn pipeline_cancel(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            operation_id: String,
        }
        let input: Input = from_params(params)?;
        let project = self.pipeline_bound_project(binding_id)?;
        let run = pipeline_runs()
            .lock()
            .map_err(|_| {
                AppError::new("host_unavailable", "Pipeline run registry is unavailable.")
            })?
            .get(&(project.project_id, input.operation_id.clone()))
            .cloned();
        let run = run.ok_or_else(|| {
            AppError::new(
                "operation_not_found",
                "No active Pipeline operation has this ID.",
            )
        })?;
        if binding_id != Some(run.initiating_binding_id.as_str()) {
            return Err(AppError::new(
                "operation_not_owner",
                "Only the binding that started this Pipeline operation may cancel it.",
            ));
        }
        run.cancellation.cancel();
        self.pipeline_operation_changed(binding_id, &input.operation_id, "cancelling")?;
        self.publish_pipeline_changed(binding_id, "agent", &project, run.method_id, "cancelled")?;
        Ok(json!({"operation_id": input.operation_id, "status":"cancelling"}))
    }

    fn pipeline_bound_project(
        &self,
        binding_id: Option<&str>,
    ) -> Result<PipelineProject, AppError> {
        let state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        let current = state.current.as_ref().expect("binding requires current");
        Ok(PipelineProject {
            path: current.path.clone(),
            project_id: current.snapshot.project.project_id,
            revision_id: current.snapshot.project.current_revision_id,
            snapshot: current.snapshot.clone(),
        })
    }

    fn pipeline_source_content(
        &self,
        project: &PipelineProject,
        segment_id: SegmentId,
    ) -> Result<String, AppError> {
        project
            .snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .map(|segment| segment.content.clone())
            .ok_or_else(|| {
                AppError::new(
                    "segment_not_found",
                    "Segment does not exist in the current project.",
                )
            })
    }

    fn pipeline_revision_is_current(&self, project: &PipelineProject) -> Result<bool, AppError> {
        let state = self.lock()?;
        Ok(state.current.as_ref().is_some_and(|current| {
            current.snapshot.project.project_id == project.project_id
                && current.snapshot.project.current_revision_id == project.revision_id
        }))
    }

    fn pipeline_operation_changed(
        &self,
        binding_id: Option<&str>,
        operation_id: &str,
        status: &str,
    ) -> Result<(), AppError> {
        let mut state = self.lock()?;
        state.operations.insert(operation_id.into(), status.into());
        self.publish_locked(
            &mut state,
            "operation_changed",
            binding_id,
            "agent",
            json!({"operation_id": operation_id, "status": status}),
        );
        Ok(())
    }

    fn publish_pipeline_changed(
        &self,
        binding_id: Option<&str>,
        origin: &str,
        project: &PipelineProject,
        method_id: PipelineMethodId,
        action: &str,
    ) -> Result<(), AppError> {
        let mut state = self.lock()?;
        self.publish_locked(
            &mut state,
            "pipeline_changed",
            binding_id,
            origin,
            json!({
                "project_id": project.project_id,
                "revision_id": project.revision_id,
                "method_id": method_id,
                "action": action,
            }),
        );
        Ok(())
    }
}

#[derive(Clone)]
struct PipelineProject {
    path: PathBuf,
    project_id: ProjectId,
    revision_id: RevisionId,
    snapshot: ProjectSnapshot,
}

fn default_plan() -> PipelinePlanSnapshot {
    let source = PipelineNodeId::new();
    let normalize = PipelineNodeId::new();
    let tokenize = PipelineNodeId::new();
    let artifact = PipelineNodeId::new();
    PipelinePlanSnapshot {
        method_id: PipelineMethodId::new(),
        method_revision_id: PipelineMethodRevisionId::new(),
        nodes: vec![
            PipelineNode {
                node_id: source,
                operator: "source".into(),
                inputs: Vec::new(),
                config: json!({}),
            },
            PipelineNode {
                node_id: normalize,
                operator: "normalize".into(),
                inputs: vec![source],
                config: json!({"trim": false, "collapse_whitespace": false}),
            },
            PipelineNode {
                node_id: tokenize,
                operator: "chinese_tokenize".into(),
                inputs: vec![normalize],
                config: json!({"hmm": true, "custom_dictionary": []}),
            },
            PipelineNode {
                node_id: artifact,
                operator: "artifact".into(),
                inputs: vec![tokenize],
                config: json!({}),
            },
        ],
        output_node_id: artifact,
    }
}

fn pipeline_service(path: &Path) -> Result<PipelineService, AppError> {
    PipelineService::open(path).map_err(pipeline_error)
}

fn pipeline_error(error: PipelineError) -> AppError {
    let code = match &error {
        PipelineError::StaleMethodRevision { .. } => "stale_method_revision",
        PipelineError::Cancelled => "operation_cancelled",
        PipelineError::MethodNotFound(_) | PipelineError::MethodRevisionNotFound(_) => {
            "pipeline_method_not_found"
        }
        PipelineError::ArtifactNotFound(_) => "pipeline_artifact_not_found",
        PipelineError::InvalidProjectRoot(_) | PipelineError::Io { .. } => "pipeline_storage_error",
        _ => "pipeline_error",
    };
    AppError::new(code, error.to_string())
}

fn pipeline_runs() -> &'static StdMutex<HashMap<(ProjectId, String), PipelineRun>> {
    PIPELINE_RUNS.get_or_init(|| StdMutex::new(HashMap::new()))
}

/// Returns a narrow, per-project method/proposal lock. The registry mutex is
/// held only long enough to obtain the Arc; tokenization and cancellation do
/// not acquire this lock.
fn pipeline_project_lock(path: &Path) -> Result<Arc<StdMutex<()>>, AppError> {
    let mut locks = PIPELINE_PROJECT_LOCKS
        .get_or_init(|| StdMutex::new(HashMap::new()))
        .lock()
        .map_err(|_| AppError::new("host_unavailable", "Pipeline lock registry is unavailable."))?;
    Ok(locks
        .entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(StdMutex::new(())))
        .clone())
}

fn lock_pipeline_project(
    lock: &Arc<StdMutex<()>>,
) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
    lock.lock()
        .map_err(|_| AppError::new("host_unavailable", "Pipeline project lock is unavailable."))
}

fn pipeline_proposal_path(project: &Path) -> PathBuf {
    project
        .join("extensions")
        .join("pipeline")
        .join("method-proposals-v1.json")
}

fn read_pipeline_proposals(
    project: &PipelineProject,
) -> Result<Vec<PipelineMethodProposal>, AppError> {
    let path = pipeline_proposal_path(&project.path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&path)
        .map_err(|error| AppError::new("pipeline_proposal_read_failed", error.to_string()))?;
    let journal: PipelineProposalJournal = serde_json::from_slice(&bytes)
        .map_err(|error| AppError::new("pipeline_proposal_invalid", error.to_string()))?;
    if journal.format_version != PIPELINE_PROPOSAL_FORMAT {
        return Err(AppError::new(
            "pipeline_proposal_unsupported",
            format!(
                "Unsupported Pipeline proposal journal version {}.",
                journal.format_version
            ),
        ));
    }
    if journal.project_id != project.project_id.to_string() {
        return Err(AppError::new(
            "pipeline_proposal_project_mismatch",
            "The Pipeline proposal journal belongs to another project.",
        ));
    }
    let mut proposals = journal.proposals;
    if reconcile_pipeline_proposals(project, &mut proposals)? {
        persist_pipeline_proposals(project, &proposals)?;
    }
    Ok(proposals)
}

/// Pipeline method updates are non-canonical but still use a durable two-step
/// proposal journal. Recovery relies on the host-reserved revision ID, never
/// on a content match: an ordinary identical manual update is not an approval.
fn reconcile_pipeline_proposals(
    project: &PipelineProject,
    proposals: &mut [PipelineMethodProposal],
) -> Result<bool, AppError> {
    let service = pipeline_service(&project.path)?;
    let mut changed = false;
    for proposal in proposals.iter_mut().filter(|proposal| {
        matches!(
            proposal.status,
            PipelineProposalStatus::Pending | PipelineProposalStatus::Applying
        )
    }) {
        let Some(reserved_method_revision_id) = proposal.reserved_method_revision_id else {
            // Legacy journals have no unambiguous commit identity. They must
            // remain pending; approval will reserve and persist one first.
            if proposal.status == PipelineProposalStatus::Applying {
                proposal.status = PipelineProposalStatus::Pending;
                changed = true;
            }
            continue;
        };
        match service.load_method_revision(proposal.method_id, reserved_method_revision_id) {
            Ok(revision) => {
                let expected_name = proposal
                    .request
                    .name
                    .as_deref()
                    .unwrap_or(&proposal.before.name)
                    .trim();
                let exact_reserved_commit = revision.method_id == proposal.method_id
                    && revision.method_revision_id == reserved_method_revision_id
                    && revision.parent_method_revision_id == Some(proposal.base_method_revision_id)
                    && revision.method_name == expected_name
                    && revision.plan.nodes == proposal.request.plan.nodes
                    && revision.plan.output_node_id == proposal.request.plan.output_node_id;
                if exact_reserved_commit
                    && (proposal.status != PipelineProposalStatus::Approved
                        || proposal.approved_method_revision_id
                            != Some(reserved_method_revision_id))
                {
                    proposal.status = PipelineProposalStatus::Approved;
                    proposal.approved_method_revision_id = Some(reserved_method_revision_id);
                    changed = true;
                }
            }
            Err(PipelineError::MethodRevisionNotFound(_))
                if proposal.status == PipelineProposalStatus::Applying =>
            {
                // The journal reached Applying but the service commit did not;
                // keep the same reservation so a retry cannot duplicate it.
                proposal.status = PipelineProposalStatus::Pending;
                changed = true;
            }
            Err(PipelineError::MethodRevisionNotFound(_)) => {}
            Err(error) => return Err(pipeline_error(error)),
        }
    }
    Ok(changed)
}

fn persist_pipeline_proposals(
    project: &PipelineProject,
    proposals: &[PipelineMethodProposal],
) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(&PipelineProposalJournal {
        format_version: PIPELINE_PROPOSAL_FORMAT.into(),
        project_id: project.project_id.to_string(),
        proposals: proposals.to_vec(),
    })
    .map_err(|error| AppError::new("pipeline_proposal_serialize_failed", error.to_string()))?;
    write_bytes_atomic(&pipeline_proposal_path(&project.path), &bytes)
        .map_err(|error| AppError::new("pipeline_proposal_write_failed", error.to_string()))
}
