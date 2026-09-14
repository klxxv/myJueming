import { agentClient } from "./agent-client";
import type { OperatorDescriptor, ResearchRun, SchemaRef, SlotState } from "./research-types";
import type { GraphRunV2, PipelineMethodV2, PlanV2, SchemaDefinitionV2 } from "./pipeline-v2-types";
import type {
  PipelineArtifactId,
  PipelineArtifactSummary,
  PipelineExecution,
  PipelineMethod,
  PipelineMethodId,
  PipelineMethodProposal,
  PipelineMethodRevisionId,
  PipelineMethodSummary,
  TokenArtifact,
  UpdatePipelineMethod,
} from "./pipeline-types";

type BindingId = string | null | undefined;

const call = async <T>(method: string, params: Record<string, unknown>, bindingId: BindingId): Promise<T> =>
  (await agentClient.call<T>(method, params, bindingId)).data;

/**
 * Pipeline is available only through the native Agent bridge. agentClient
 * rejects ordinary browser use before any request is emitted. Trusted routes
 * are also enforced again by LocalAppHost, so an external transport cannot
 * use this facade to gain approval authority.
 */
export const pipelineClient = {
  v2: {
    operators: () => call<OperatorDescriptor[]>("operators.list", {}, null),
    slots: () => call<SlotState[]>("slots.list", {}, null),
    schema: (schema: SchemaRef) => call<SchemaDefinitionV2>("schemas.get", { ...schema }, null),
    listMethods: (bindingId: BindingId) => call<PipelineMethodV2[]>("pipeline.list_methods_v2", {}, bindingId),
    validatePlan: (plan: PlanV2, bindingId: BindingId) => call<string[]>("pipeline.validate_plan", { ...plan }, bindingId),
    saveMethod: (request: { method_id?: string; name: string; base_method_revision_id?: string; plan: PlanV2 }, bindingId: BindingId) => call<PipelineMethodV2>("pipeline.save_method_v2", request, bindingId),
    start: (method: { method_id: string; method_revision_id: string }, bindingId: BindingId) => call<ResearchRun>("pipeline.start", method, bindingId),
    getRun: (runId: string, bindingId: BindingId) => call<GraphRunV2>("pipeline.get_graph_run", { run_id: runId }, bindingId),
    cancelRun: (runId: string, bindingId: BindingId) => call<ResearchRun>("pipeline.cancel_graph_run", { run_id: runId }, bindingId),
  },
  list: (bindingId: BindingId) => call<PipelineMethodSummary[]>("pipeline.list", {}, bindingId),

  get: (methodId: PipelineMethodId, bindingId: BindingId) =>
    call<PipelineMethod>("pipeline.get", { method_id: methodId }, bindingId),

  createDefault: (name: string, bindingId: BindingId) =>
    call<PipelineMethod>("pipeline.create_default", { name }, bindingId),

  update: (request: UpdatePipelineMethod, bindingId: BindingId) =>
    call<PipelineMethod>("pipeline.update", { request }, bindingId),

  previewUpdate: (request: UpdatePipelineMethod, bindingId: BindingId) =>
    call<PipelineMethodProposal>("pipeline.preview_update", { request }, bindingId),

  approveUpdate: (proposalId: string, bindingId: BindingId) =>
    call<PipelineMethodProposal>("pipeline.approve_update", { proposal_id: proposalId }, bindingId),

  rejectUpdate: (proposalId: string, bindingId: BindingId) =>
    call<PipelineMethodProposal>("pipeline.reject_update", { proposal_id: proposalId }, bindingId),

  listProposals: (bindingId: BindingId) =>
    call<PipelineMethodProposal[]>("pipeline.list_proposals", {}, bindingId),

  execute: (
    input: {
      method_id: PipelineMethodId;
      method_revision_id?: PipelineMethodRevisionId | null;
      segment_id: string;
      base_revision_id: string;
      operation_id?: string;
    },
    bindingId: BindingId,
  ) => call<PipelineExecution>("pipeline.execute", input, bindingId),

  artifact: (artifactId: PipelineArtifactId, bindingId: BindingId) =>
    call<TokenArtifact>("pipeline.artifacts", { artifact_id: artifactId }, bindingId),

  artifactsForRevision: (revisionId: string, bindingId: BindingId) =>
    call<PipelineArtifactSummary[]>("pipeline.artifacts", { input_revision_id: revisionId }, bindingId),

  cancel: (operationId: string, bindingId: BindingId) =>
    call<{ operation_id: string; status: string }>("pipeline.cancel", { operation_id: operationId }, bindingId),
};
