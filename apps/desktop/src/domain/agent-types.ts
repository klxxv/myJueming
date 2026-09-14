import type { ProjectIdentity } from "./kernel-client";

export type AgentParams = Record<string, unknown>;

export interface AgentCall {
  request_id: string;
  method: string;
  params: AgentParams;
  binding_id?: string | null;
}

export interface AgentReply<T = unknown> {
  request_id: string;
  data: T;
  sequence: string;
}

export interface AppErrorShape {
  code: string;
  message: string;
}

export interface AppEvent {
  contract_version: "1.0";
  sequence: string;
  kind: AgentEventKind;
  binding_id: string | null;
  origin: string;
  payload: unknown;
}

export type AgentEventKind =
  | "project_changed"
  | "revision_advanced"
  | "context_changed"
  | "search_changed"
  | "pipeline_changed"
  | "navigation_requested"
  | "reveal_requested"
  | "proposal_changed"
  | "operation_changed";

export interface TextRangeUtf16 {
  start_utf16: number;
  end_utf16: number;
}

export interface ContextSnapshot {
  tab: string;
  mode: string | null;
  project_id: string | null;
  revision_id: string | null;
  binding_id: string | null;
  window_focused: boolean;
  focused_control: string | null;
  selected_text: string;
  segment_ids: string[];
  alignment_ids: string[];
  text_range: TextRangeUtf16 | null;
  captured_at: string;
}

export interface SearchSpec {
  document_ids?: string[] | null;
  query: string;
  regex: boolean;
  case_sensitive: boolean;
  language_id: string | null;
}

export interface AgentSessionBinding {
  binding_id: string;
  project_id: string;
  revision_id: string;
}

export interface AgentProposal {
  proposal_id?: string;
  status?: string;
  base_revision_id?: string;
  [key: string]: unknown;
}

export interface AgentProjection {
  binding_id: string | null;
  project: ProjectIdentity | null;
  context: ContextSnapshot | null;
  search_spec: SearchSpec | null;
  search_results: unknown;
  proposals: AgentProposal[];
  pending_ui_actions?: PendingUiAction[];
}

export interface PendingUiAction {
  operation_id: string;
  request_id: string;
  binding_id: string;
  project_id: string | null;
  revision_id: string | null;
  kind: "navigation_requested" | "reveal_requested";
  payload: unknown;
  status: "accepted";
}

export interface AgentNavigationRequest {
  tab: string;
  request_id: string;
  operation_id: string;
  project_id: string | null;
  revision_id: string | null;
}

export interface AgentRevealRequest {
  segment_id?: string;
  alignment_id?: string;
  node_id?: string;
  request_id: string;
  operation_id: string;
  project_id: string | null;
  revision_id: string | null;
}

export type AgentClientUpdate =
  | { type: "projection"; projection: AgentReply<AgentProjection> }
  | { type: "event"; event: AppEvent }
  | { type: "error"; error: AppErrorShape };

export interface AgentSubscription {
  dispose(): void;
}

export interface AgentChatMessage {
  id: string;
  requestId: string;
  role: "user";
  content: string;
  sentAt: string;
  status: "sending" | "sent" | "failed";
  context: ImmutableContextSnapshot;
  error: string | null;
}

export interface ImmutableContextSnapshot extends Omit<ContextSnapshot, "segment_ids" | "alignment_ids" | "text_range"> {
  readonly segment_ids: readonly string[];
  readonly alignment_ids: readonly string[];
  readonly text_range: Readonly<TextRangeUtf16> | null;
}

export interface AgentAnnotationScope {
  segmentIds: string[];
  alignmentIds: string[];
}

export interface AgentProjectScope {
  chat: AgentChatMessage[];
  annotationScope: AgentAnnotationScope;
  drafts: Record<string, string>;
}
