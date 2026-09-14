import assert from "node:assert/strict";
import { createPinia, setActivePinia } from "pinia";
import { ref } from "vue";
import { useAgentWorkspace } from "../../apps/desktop/src/composables/useAgentWorkspace";
import type { AgentClient } from "../../apps/desktop/src/domain/agent-client";
import type { AgentClientUpdate, AgentProjection, AgentReply } from "../../apps/desktop/src/domain/agent-types";
import type { ProjectIdentity } from "../../apps/desktop/src/domain/kernel-client";

setActivePinia(createPinia());

const projectId = "project-native-b";
const revisionId = "revision-native-b";
const nativeBinding = "binding-native-b";
const externalBinding = "binding-external-a";
const calls: Array<{ method: string; params: Record<string, unknown>; bindingId?: string | null }> = [];
let listener: ((update: AgentClientUpdate) => void) | null = null;
let navigations = 0;

const projection: AgentReply<AgentProjection> = {
  request_id: "projection-native-b",
  sequence: "2",
  data: {
    binding_id: nativeBinding,
    project: null,
    context: null,
    search_spec: null,
    search_results: [],
    proposals: [],
    pending_ui_actions: [],
  },
};

const client: AgentClient = {
  available: true,
  async call<T>(method, params = {}, bindingId) {
    calls.push({ method, params, bindingId });
    if (method === "app.bind_session") {
      return {
        request_id: "bind-native-b",
        sequence: "1",
        data: { binding_id: nativeBinding, project_id: projectId, revision_id: revisionId } as T,
      };
    }
    return { request_id: "native-call", sequence: "3", data: {} as T };
  },
  async projection() { return projection; },
  async subscribe(next) {
    listener = next;
    return { dispose: () => { listener = null; } };
  },
};

const workspace = useAgentWorkspace({
  tab: "review",
  mode: "review",
  projectSnapshot: ref({ project: { project_id: projectId, current_revision_id: revisionId } } as ProjectIdentity),
  selection: { segmentIds: [], alignmentIds: [] },
  searchState: { query: "", regex: false, caseSensitive: false, languageId: null },
  selectionSharingEnabled: true,
  canLeaveDraft: () => true,
  navigate: () => { navigations += 1; },
  reveal: () => undefined,
  onProjectSnapshot: () => undefined,
  onSearchState: () => undefined,
  client,
});

await workspace.start();
listener?.({
  type: "event",
  event: {
    contract_version: "1.0",
    sequence: "4",
    kind: "navigation_requested",
    binding_id: externalBinding,
    origin: "agent",
    payload: {
      tab: "search",
      request_id: "request-external-a",
      operation_id: "operation-external-a",
      project_id: projectId,
      revision_id: revisionId,
    },
  },
});
await new Promise((resolve) => setTimeout(resolve, 0));

assert.equal(navigations, 1, "a native workspace must execute a same-project event from external binding A");
const acknowledgement = calls.find((call) => call.method === "ui.ack");
assert.deepEqual(acknowledgement, {
  method: "ui.ack",
  params: {
    operation_id: "operation-external-a",
    request_id: "request-external-a",
    status: "ui_applied",
  },
  bindingId: nativeBinding,
}, "native binding B must acknowledge the exact external A operation/request pair");

workspace.dispose();
console.log("two-binding production composable acknowledgement: passed");
