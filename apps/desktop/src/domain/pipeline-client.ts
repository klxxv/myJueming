import { agentClient } from "./agent-client";
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
