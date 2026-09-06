import { invoke } from "@tauri-apps/api/core";

export interface AgentConnectionInfo {
  status: { state: "disabled" | "running" | "failed"; enabled: boolean; boundAddr: string | null };
  mcp_command: string | null;
}
export const agentConnectionClient = {
  get available() { return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window; },
  status: () => invoke<AgentConnectionInfo>("agent_connection_status"),
  setEnabled: (enabled: boolean) => invoke<AgentConnectionInfo>("agent_connection_set_enabled", { enabled }),
  configuration: (command: string) => invoke<string>("agent_connection_config", { command }),
};
