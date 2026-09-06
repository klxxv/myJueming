# Pipeline service API v0.1

Owner: Pipeline worker. Status: integration contract for M3.

## Required workspace registration

Main agent must add `"crates/jueming-pipeline"` to root `Cargo.toml` workspace `members`. The new crate uses these dependencies and no host, UI, shell or plugin-runtime dependency:

```toml
jueming-core = { path = "../jueming-core" }
jueming-storage = { path = "../jueming-storage" }
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
thiserror.workspace = true
uuid.workspace = true
jieba-rs = "0.10.3"
```

Run Cargo dependency resolution from the root so `Cargo.lock` remains main-owned. `jieba-rs 0.10.3` was checked against the current docs.rs crate page on 2026-09-06.

## Public Rust surface

```rust
pub struct PipelineService;

impl PipelineService {
    pub fn open(project_root: impl AsRef<Path>) -> Result<Self, PipelineError>;
    pub fn list_methods(&self) -> Result<Vec<PipelineMethodSummary>, PipelineError>;
    pub fn load_method(&self, method_id: PipelineMethodId) -> Result<PipelineMethod, PipelineError>;
    pub fn load_method_revision(
        &self,
        method_id: PipelineMethodId,
        method_revision_id: PipelineMethodRevisionId,
    ) -> Result<PipelineMethodRevision, PipelineError>;
    pub fn create_method(&self, request: CreatePipelineMethod) -> Result<PipelineMethod, PipelineError>;
    pub fn update_method(&self, request: UpdatePipelineMethod) -> Result<PipelineMethod, PipelineError>;
    pub fn update_method_with_revision_id(
        &self,
        request: UpdatePipelineMethod,
        method_revision_id: PipelineMethodRevisionId,
    ) -> Result<PipelineMethod, PipelineError>;
    pub fn execute(
        &self,
        request: ExecutePipelineRequest,
        cancellation: &CancellationToken,
    ) -> Result<PipelineExecution, PipelineError>;
    pub fn load_artifact(&self, artifact_id: PipelineArtifactId) -> Result<TokenArtifact, PipelineError>;
    pub fn list_artifacts_for_revision(
        &self,
        input_revision_id: RevisionId,
    ) -> Result<Vec<PipelineArtifactSummary>, PipelineError>;
}
```

`PipelineMethodId`, `PipelineMethodRevisionId`, `PipelineNodeId` and `PipelineArtifactId` are serde-transparent UUIDv7 newtypes. `PipelineMethod` contains the current immutable revision and its revision history summaries. `PipelinePlanSnapshot` is a validation-ready, frontend-friendly representation of a graph revision. `TokenArtifact` contains input canonical `RevisionId`, stable `SegmentId`, original-content SHA-256, normalized derived text and SHA-256, the exact Pipeline MethodRevision, and ordered token records with UTF-8 byte offsets into that normalized text.

`UpdatePipelineMethod` includes `base_method_revision_id`; a mismatch returns `PipelineError::StaleMethodRevision`. `update_method` generates a fresh UUIDv7 revision ID. `update_method_with_revision_id` is for a trusted host's pre-reserved UUIDv7 only; it rejects a non-v7 or already-used ID and must never be exposed as a caller-controlled gateway field. `load_method_revision` reads immutable historical data, even when a later revision is current. Each `PipelineMethodRevision` also stores the method name at that revision, so a reserved commit can be verified independently of a later manifest name. `ExecutePipelineRequest` includes `method_id`, optional `method_revision_id`, `input_revision_id`, `segment_id`, and `source_content`. It has no canonical write capability. `CancellationToken::cancel()` may be called by the host; `execute` returns `PipelineError::Cancelled` before persistence.

History integrations use `list_artifacts_for_revision(input_revision_id)` to find derived artifacts associated with the selected canonical revision. These summaries are navigation data only; method revisions never appear in canonical revision history.

## JSON / frontend shape

All public request and response types derive `Serialize` and `Deserialize`, use `snake_case` enum tags, and use string IDs. `RevisionId` stays its existing decimal-string serialization. `PipelinePlanSnapshot` uses:

```json
{
  "method_id": "uuid-v7",
  "method_revision_id": "uuid-v7",
  "nodes": [{ "node_id": "uuid-v7", "operator": "source", "inputs": [], "config": {} }],
  "output_node_id": "uuid-v7"
}
```

The only accepted `operator` strings are `source`, `normalize`, `chinese_tokenize`, and `artifact`. The service rejects unknown values and invalid slot topology before method publication or execution.

## LocalAppHost gateway v0.1

`LocalAppHost::pipeline_call(&self, method: &str, params: serde_json::Map<String, Value>, binding_id: Option<&str>, trusted_native: bool) -> Result<Value, AppError>` is the only application gateway. It always validates the bound project and canonical revision before it reads source text. No caller may supply `source_content`.

| Method | Parameters | Authority | Result |
| --- | --- | --- | --- |
| `pipeline.list` | none | bound caller | `PipelineMethodSummary[]` |
| `pipeline.get` | `{ method_id }` | bound caller | `PipelineMethod` |
| `pipeline.create_default` | `{ name }` | trusted native | a real four-node `PipelineMethod` |
| `pipeline.update` | `{ request: UpdatePipelineMethod }` | trusted native | updated `PipelineMethod` |
| `pipeline.preview_update` | `{ request: UpdatePipelineMethod }` | bound caller | durable immutable `PipelineMethodProposal` |
| `pipeline.approve_update` | `{ proposal_id }` | trusted native | approved proposal and updated method |
| `pipeline.reject_update` | `{ proposal_id }` | bound caller | rejected proposal |
| `pipeline.list_proposals` | none | bound caller | `PipelineMethodProposal[]` |
| `pipeline.execute` | `{ method_id, method_revision_id?, segment_id, base_revision_id, operation_id? }` | bound caller | `{ operation_id, artifact }` |
| `pipeline.artifacts` | `{ artifact_id? , input_revision_id? }` | bound caller | one `TokenArtifact` or `PipelineArtifactSummary[]` |
| `pipeline.cancel` | `{ operation_id }` | bound caller | operation status |

`pipeline.execute` reads the identified Segment from the LocalAppHost's current Kernel snapshot. It checks `base_revision_id` before execution and again after the tokenizer returns. A changed canonical revision yields `stale_revision`; a historical derived artifact can remain discoverable by `input_revision_id`, but it is never applied to canonical content. To cancel an in-flight synchronous call, clients may provide a non-empty `operation_id` and invoke `pipeline.cancel` concurrently; callers that omit it receive a server-generated ID only when execution completes.

`PipelineMethodProposal` is persisted at `.jm/extensions/pipeline/method-proposals-v1.json` with `project_id`, `base_canonical_revision_id`, `base_method_revision_id`, the full before snapshot, requested update plan, and a server-created `reserved_method_revision_id`. The host writes this UUIDv7 reservation before approval begins; callers cannot provide it. Approval checks both revisions and invokes the exact stored `UpdatePipelineMethod` with that reservation; it never reconstructs a request from current UI data. Legacy proposals with no reservation remain pending and receive a persisted reservation only when a native approval starts. Events use `pipeline_changed` with the stable `method_id` and an action of `created`, `updated`, `proposal_created`, `proposal_approved`, `proposal_rejected`, `executed`, or `cancelled`.

Method creation, direct update, proposal preview, approval and rejection are serialized by a narrow lock keyed by `.jm` project path. The lock registry is only held while looking up that project lock; it is never held during Jieba execution or `pipeline.cancel`. Active runs use a separate `(project_id, operation_id)` registry entry and record the initiating `binding_id`. `pipeline.cancel` requires that same binding, so another external client in the same project cannot cancel a guessed operation ID. The same client-supplied operation ID can exist in different projects and every execute result removes its entry. Proposal recovery marks a `pending` or `applying` proposal approved only when the method's immutable history contains its exact reserved ID with the stored parent, name and plan. It searches history rather than only the current revision, so a later ordinary update does not hide a committed approval. An identical direct native update has a different revision ID and cannot approve a proposal by inference. If no reserved commit exists after an interrupted `applying` write, recovery returns it to `pending` with the same reservation; retry remains exactly-once.

### Host-owner integration required

The host owner/main agent must make these exact changes in `crates/jueming-application`:

```toml
# Cargo.toml [dependencies]
jueming-pipeline = { path = "../jueming-pipeline" }
```

```rust
// host.rs, alongside the other submodules
mod pipeline;

// host.rs, in dispatch_as match before the fallback arm
method if method.starts_with("pipeline.") => self.pipeline_call(
    method,
    call.params,
    call.binding_id.as_deref(),
    trusted_native,
)?,
```

The frontend event union must include `pipeline_changed`. Native Tauri `agent_call` already enters `dispatch_native`; external transport enters `dispatch`, so trusted-only Pipeline routes cannot be reached through the HTTP/MCP SDK.

For the real host gateway acceptance coverage, host integration must also include the shared test source in `crates/jueming-application/tests/host.rs`:

```rust
#[path = "../../../tests/agent-pipeline/pipeline_gateway.rs"]
mod pipeline_gateway;
```
