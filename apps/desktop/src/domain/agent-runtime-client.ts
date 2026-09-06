import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ContextSnapshot } from "./agent-types";

export type AgentRuntimeProviderKind = "loopback" | "https";
export type AgentRuntimeSecretStorage = "keyring" | "session_memory" | "none";
export type AgentRuntimeRunState = "running" | "awaiting_approval" | "completed" | "cancelled" | "failed";
export type AgentRuntimeEventKind = "run_started" | "tool_started" | "tool_finished" | "awaiting_approval" | "run_completed" | "run_cancelled" | "run_failed";

export interface AgentRuntimeConfigurationInput {
  provider_kind: AgentRuntimeProviderKind;
  endpoint: string;
  model: string;
  /** Sent only to native IPC. It is intentionally absent from all response types. */
  api_key?: string | null;
}

export interface AgentRuntimeConfiguration {
  configured: boolean;
  provider_kind: AgentRuntimeProviderKind;
  endpoint: string;
  model: string;
  secret_storage: AgentRuntimeSecretStorage;
  api_key_configured: boolean;
}

export interface AgentRuntimeStatus extends AgentRuntimeConfiguration {
  active_run_ids: string[];
  recovered_interrupted_runs: number;
}

export interface AgentRuntimeRunRequest {
  project_id: string;
  prompt: string;
  /** Frozen at send-time; native validates its project, revision, binding, stable references, and selected-text range. */
  context: ContextSnapshot;
  session_id?: string | null;
}

export interface AgentRuntimeRunHandle {
  run_id: string;
  session_id: string;
  state: AgentRuntimeRunState;
}

export interface AgentRuntimeHistoryRequest {
  project_id: string;
  /** Omit to restore the most recently active session for this local project. */
  session_id?: string | null;
  limit?: number;
}

export interface AgentRuntimeSessionSummary {
  session_id: string;
  project_id: string;
  last_message_at: string;
}

export interface AgentRuntimeMessage {
  message_id: string;
  role: "user" | "assistant" | "tool";
  content: string;
  context: unknown | null;
  tool_call_id: string | null;
  created_at: string;
}

export interface AgentRuntimeHistory {
  session_id: string;
  project_id: string;
  messages: AgentRuntimeMessage[];
}

export interface AgentRuntimeEvent {
  run_id: string;
  session_id: string;
  project_id: string;
  kind: AgentRuntimeEventKind;
  state: AgentRuntimeRunState;
  payload: unknown;
  emitted_at: string;
}

/**
 * Native asks the UI to reload runtime status and durable history after a
 * project transition, runtime recovery, or an event-stream gap.
 */
export interface AgentRuntimeResyncEvent {
  reason: "project_changed" | "runtime_recovered" | "event_gap" | string;
  project_id?: string | null;
  session_id?: string | null;
}

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const unavailable = () => new Error("嵌入式 Agent 仅可在本地 Tauri 桌面应用中运行");

export interface AgentRuntimeBridge {
  available(): boolean;
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, listener: (payload: T) => void): Promise<() => void>;
}

const tauriBridge: AgentRuntimeBridge = {
  available: inTauri,
  invoke: <T>(command: string, args?: Record<string, unknown>) => invoke<T>(command, args),
  listen: async <T>(event: string, listener: (payload: T) => void) => listen<T>(event, (received) => listener(received.payload)),
};

export interface AgentRuntimeClient {
  status(): Promise<AgentRuntimeStatus>;
  configure(config: AgentRuntimeConfigurationInput): Promise<AgentRuntimeConfiguration>;
  start(request: AgentRuntimeRunRequest): Promise<AgentRuntimeRunHandle>;
  cancel(runId: string): Promise<void>;
  history(request: AgentRuntimeHistoryRequest): Promise<AgentRuntimeHistory>;
  sessions(projectId: string): Promise<AgentRuntimeSessionSummary[]>;
  subscribe(listener: (event: AgentRuntimeEvent) => void): Promise<() => void>;
  subscribeResync(listener: (event: AgentRuntimeResyncEvent) => void): Promise<() => void>;
}

export const createAgentRuntimeClient = (bridge: AgentRuntimeBridge = tauriBridge): AgentRuntimeClient => ({
  async status() {
    if (!bridge.available()) throw unavailable();
    return bridge.invoke<AgentRuntimeStatus>("agent_runtime_status");
  },
  async configure(config) {
    if (!bridge.available()) throw unavailable();
    return bridge.invoke<AgentRuntimeConfiguration>("agent_runtime_configure", { config });
  },
  async start(request) {
    if (!bridge.available()) throw unavailable();
    return bridge.invoke<AgentRuntimeRunHandle>("agent_runtime_start", { request });
  },
  async cancel(runId) {
    if (!bridge.available()) throw unavailable();
    await bridge.invoke<void>("agent_runtime_cancel", { runId });
  },
  async history(request) {
    if (!bridge.available()) throw unavailable();
    return bridge.invoke<AgentRuntimeHistory>("agent_runtime_history", { request });
  },
  async sessions(projectId) {
    if (!bridge.available()) throw unavailable();
    return bridge.invoke<AgentRuntimeSessionSummary[]>("agent_runtime_sessions", { projectId });
  },
  async subscribe(listener) {
    if (!bridge.available()) throw unavailable();
    return bridge.listen<AgentRuntimeEvent>("agent_runtime_event", listener);
  },
  async subscribeResync(listener) {
    if (!bridge.available()) throw unavailable();
    return bridge.listen<AgentRuntimeResyncEvent>("agent_runtime_resync", listener);
  },
});

export const agentRuntimeClient = createAgentRuntimeClient();
