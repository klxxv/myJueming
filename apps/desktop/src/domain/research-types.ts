/** Local research wire contract v2. UTF-8 offsets always refer to original text. */
export interface TextRange { segment_id: string; start_utf8: number; end_utf8: number }
export interface FeatureSnapshot {
  feature_id: string;
  desired_enabled: boolean;
  status: "disabled" | "preparing" | "ready" | "cancelling" | "disabling" | "failed" | "blocked";
  stage: string | null;
  generation: string;
  activation_id: string | null;
  completed_bytes: string | null;
  total_bytes: string | null;
  reason: { code: string; message: string; retryable: boolean } | null;
  resources_ready: boolean;
  worker_state: string;
  default_similarity: string;
  auto_locate: boolean;
}
export interface SchemaRef { name: string; version: number }
export interface PortDescriptor { name: string; schemas: SchemaRef[]; required: boolean; multiple: boolean }
export interface OperatorDescriptor {
  operator_id: string; release: string; name: string; slots: string[];
  inputs: PortDescriptor[]; outputs: PortDescriptor[]; config_schema: Record<string, unknown>;
}
export interface SlotState {
  slot_id: string; name: string; state: "bound" | "unbound" | "disabled" | "incompatible";
  reason: string | null; inputs: PortDescriptor[]; outputs: PortDescriptor[]; providers: string[];
}
export interface CapabilitySnapshot {
  contract_version: string; generation: string; features: FeatureSnapshot[];
  slots: SlotState[]; operators: OperatorDescriptor[];
}
export interface ResearchRun {
  run_id: string; project_id: string; input_revision_id: string;
  status: "queued" | "running" | "completed" | "cancelled" | "failed" | "interrupted";
  error: string | null; total: number; completed: number;
}
export interface ResearchJudgement { kind: "translation" | "omission" | "paraphrase"; group_name: string; strategy: string; target_ranges: TextRange[] }
export interface ResearchOccurrence {
  occurrence_id: string;
  source: { segment_id: string; text: string; ranges: { start_utf8: number; end_utf8: number }[] };
  targets: { segment_id: string; text: string }[];
  candidates: { ranges: TextRange[]; text: string; score: number | null; score_kind: string; provider_id: string }[];
  alignment_coverage?: "complete" | "partial" | "no_links" | "context_missing" | "not_requested" | "unknown" | null;
  status: string; judgement: ResearchJudgement | null;
}
export interface ResearchResultPage { run: ResearchRun; items: ResearchOccurrence[]; total: number; next_cursor: number | null }
export interface ResearchSummary { total: number; confirmed: number; pending: number; groups: {name: string; strategy: string; count: number}[]; suggestions: {left_group: string; right_group: string; score: number; reason: string}[] }
