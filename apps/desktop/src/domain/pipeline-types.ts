export type PipelineMethodId = string;
export type PipelineMethodRevisionId = string;
export type PipelineNodeId = string;
export type PipelineArtifactId = string;

export interface PipelineNode {
  node_id: PipelineNodeId;
  operator: "source" | "normalize" | "chinese_tokenize" | "artifact" | string;
  inputs: PipelineNodeId[];
  config: Record<string, unknown>;
}

export interface PipelinePlanSnapshot {
  method_id: PipelineMethodId;
  method_revision_id: PipelineMethodRevisionId;
  nodes: PipelineNode[];
  output_node_id: PipelineNodeId;
}

export interface PipelineMethodRevisionSummary {
  method_revision_id: PipelineMethodRevisionId;
  parent_method_revision_id: PipelineMethodRevisionId | null;
}

export interface PipelineMethodRevision {
  method_id: PipelineMethodId;
  method_revision_id: PipelineMethodRevisionId;
  parent_method_revision_id: PipelineMethodRevisionId | null;
  method_name: string;
  plan: PipelinePlanSnapshot;
}

export interface PipelineMethod {
  method_id: PipelineMethodId;
  name: string;
  current: PipelineMethodRevision;
  revisions: PipelineMethodRevisionSummary[];
}

export interface PipelineMethodSummary {
  method_id: PipelineMethodId;
  name: string;
  current_method_revision_id: PipelineMethodRevisionId;
  revision_count: number;
}

export interface UpdatePipelineMethod {
  method_id: PipelineMethodId;
  base_method_revision_id: PipelineMethodRevisionId;
  name: string | null;
  plan: PipelinePlanSnapshot;
}

export interface PipelineMethodProposal {
  proposal_id: string;
  project_id: string;
  status: "pending" | "applying" | "approved" | "rejected";
  base_canonical_revision_id: string;
  method_id: PipelineMethodId;
  base_method_revision_id: PipelineMethodRevisionId;
  before: PipelineMethod;
  request: UpdatePipelineMethod;
  reserved_method_revision_id: PipelineMethodRevisionId | null;
  approved_method_revision_id: PipelineMethodRevisionId | null;
}

export interface DerivedToken {
  text: string;
  start_utf8: number;
  end_utf8: number;
}

export interface TokenizerProvenance {
  implementation: string;
  version: string;
  hmm: boolean;
  custom_dictionary_sha256: string;
}

export interface TokenArtifact {
  artifact_id: PipelineArtifactId;
  artifact_format: string;
  input_revision_id: string;
  segment_id: string;
  source_content_sha256: string;
  normalized_content: string;
  normalized_content_sha256: string;
  method_id: PipelineMethodId;
  method_revision_id: PipelineMethodRevisionId;
  tokenizer: TokenizerProvenance;
  tokens: DerivedToken[];
}

export interface PipelineArtifactSummary {
  artifact_id: PipelineArtifactId;
  input_revision_id: string;
  segment_id: string;
  method_id: PipelineMethodId;
  method_revision_id: PipelineMethodRevisionId;
}

export interface PipelineExecution {
  operation_id: string;
  artifact: TokenArtifact;
}

export interface PipelineChangedEvent {
  project_id: string;
  revision_id: string;
  method_id: PipelineMethodId;
  action:
    | "created"
    | "updated"
    | "proposal_created"
    | "proposal_approved"
    | "proposal_rejected"
    | "executed"
    | "cancelled";
}
