import { agentClient } from "./agent-client";
import type { CapabilitySnapshot, FeatureSnapshot, ResearchResultPage, ResearchRun, ResearchSummary, TextRange } from "./research-types";

const call = async <T>(method: string, params: Record<string, unknown> = {}, bindingId?: string | null): Promise<T> =>
  (await agentClient.call<T>(method, params, bindingId)).data;

export const researchClient = {
  capabilities: () => call<CapabilitySnapshot>("capabilities.get"),
  featureEnable: () => call<FeatureSnapshot>("features.enable"),
  featureDisable: () => call<FeatureSnapshot>("features.disable"),
  featureCancel: () => call<FeatureSnapshot>("features.cancel_prepare"),
  featureRetry: () => call<FeatureSnapshot>("features.retry_prepare"),
  updatePreferences: (value: { default_similarity?: string; auto_locate?: boolean }) => call<FeatureSnapshot>("features.update_preferences", value),
  start: (spec: { query: string; fuzzy: boolean; similarity_operator: string; auto_locate: boolean; max_gap: number; minimum_similarity?: number; method_id?: string }, bindingId: string) => call<ResearchRun>("research.start", spec, bindingId),
  getRun: (runId: string, bindingId: string) => call<ResearchRun>("pipeline.get_run", { run_id: runId }, bindingId),
  listRuns: (bindingId: string) => call<ResearchRun[]>("pipeline.list_runs", {}, bindingId),
  cancelRun: (runId: string, bindingId: string) => call<ResearchRun>("pipeline.cancel_run", { run_id: runId }, bindingId),
  result: (runId: string, cursor: number, limit: number, bindingId: string, groupName?: string | null) => call<ResearchResultPage>("pipeline.read_result", { run_id: runId, cursor, limit, ...(groupName != null ? {group_name: groupName} : {}) }, bindingId),
  summary: (runId: string, bindingId: string) => call<ResearchSummary>("research.summary", {run_id: runId}, bindingId),
  mergeGroups: (value: {run_id: string; from_group: string; to_group: string}, bindingId: string) => call<{committed_revision_id: string}>("research.merge_groups", value, bindingId),
  confirm: (value: { run_id: string; occurrence_id: string; kind: "translation" | "omission" | "paraphrase"; target_ranges?: TextRange[]; group_name?: string; strategy?: string }, bindingId: string) => call<{ committed_revision_id: string }>("research.confirm", value, bindingId),
};
