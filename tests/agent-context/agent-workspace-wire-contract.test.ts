import assert from "node:assert/strict";
import { createPinia, setActivePinia } from "pinia";
import { ref } from "vue";
import { useAgentWorkspace } from "../../apps/desktop/src/composables/useAgentWorkspace";
import type { AgentClient } from "../../apps/desktop/src/domain/agent-client";
import type { AgentClientUpdate, AgentProjection, AgentReply } from "../../apps/desktop/src/domain/agent-types";
import type { ProjectIdentity } from "../../apps/desktop/src/domain/kernel-client";

setActivePinia(createPinia());
const snapshot = ref({
  project: { project_id: "project-wire", current_revision_id: "9007199254740993" },
} as ProjectIdentity);
const calls: Array<{ method: string; params: Record<string, unknown>; bindingId?: string | null }> = [];
const projection: AgentReply<AgentProjection> = {
  request_id: "projection-1",
  sequence: "1",
  data: { binding_id: null, project: null, context: null, search_spec: null, search_results: [], proposals: [] },
};
const resnapshot: AgentReply<AgentProjection> = {
  request_id: "projection-3",
  sequence: "3",
  data: {
    binding_id: "native-ui-binding-b",
    project: null,
    context: null,
    search_spec: null,
    search_results: [],
    proposals: [],
    pending_ui_actions: [{
      operation_id: "operation-reveal-after-gap",
      request_id: "request-reveal-after-gap",
      binding_id: "mcp-binding-a",
      project_id: "project-wire",
      revision_id: "9007199254740993",
      kind: "reveal_requested",
      payload: { segment_id: "segment-wire" },
      status: "accepted",
    }],
  },
};
let revealCalls = 0;
const client: AgentClient = {
  available: true,
  async call<T>(method, params = {}, bindingId) {
    calls.push({ method, params, bindingId });
    if (method === "app.bind_session") {
      return { request_id: "bind-1", sequence: "2", data: { binding_id: "native-ui-binding-b", project_id: "project-wire", revision_id: "9007199254740993" } as T };
    }
    return { request_id: "publish-1", sequence: "3", data: {} as T };
  },
  async projection() { return resnapshot; },
  async subscribe(listener: (update: AgentClientUpdate) => void) {
    listener({ type: "projection", projection });
    return { dispose: () => undefined };
  },
};

const workspace = useAgentWorkspace({
  tab: "parallel",
  mode: "review",
  projectSnapshot: snapshot,
  selection: { segmentIds: ["segment-wire"], alignmentIds: ["alignment-wire"] },
  searchState: { query: "", regex: false, caseSensitive: false, languageId: null },
  selectionSharingEnabled: true,
  canLeaveDraft: () => true,
  navigate: () => undefined,
  reveal: () => { revealCalls += 1; },
  onProjectSnapshot: () => undefined,
  onSearchState: () => undefined,
  client,
});

await workspace.start();
await new Promise((resolve) => setTimeout(resolve, 0));
const published = calls.find((call) => call.method === "ui.publish_context");
assert.ok(published, "workspace must publish context after a successful binding");
assert.equal(published.bindingId, "native-ui-binding-b");
assert.equal(published.params.tab, "parallel");
assert.equal(published.params.project_id, "project-wire");
assert.deepEqual(published.params.segment_ids, ["segment-wire"]);
assert.equal("context" in published.params, false, "ui.publish_context parameters must be the top-level ContextSnapshot wire shape");
assert.equal(revealCalls, 1, "pending UI actions returned by the resnapshot are replayed after binding");
const acknowledgement = calls.find((call) => call.method === "ui.ack");
assert.deepEqual(acknowledgement?.params, {
  operation_id: "operation-reveal-after-gap",
  request_id: "request-reveal-after-gap",
  status: "ui_applied",
}, "replayed action acknowledgement uses the host operation and request IDs");
assert.equal(acknowledgement?.bindingId, "native-ui-binding-b", "native UI acknowledges an external MCP action with its own binding");
workspace.dispose();

setActivePinia(createPinia());
const appOnlyCalls: Array<{ method: string; params: Record<string, unknown>; bindingId?: string | null }> = [];
const appOnlyProjection: AgentReply<AgentProjection> = {
  request_id: "app-only-projection",
  sequence: "4",
  data: {
    binding_id: null,
    project: null,
    context: null,
    search_spec: null,
    search_results: [],
    proposals: [],
    pending_ui_actions: [{
      operation_id: "operation-settings-app-only",
      request_id: "request-settings-app-only",
      binding_id: "mcp-binding-a",
      project_id: null,
      revision_id: null,
      kind: "navigation_requested",
      payload: { tab: "settings" },
      status: "accepted",
    }],
  },
};
let appOnlyNavigations = 0;
const appOnlyClient: AgentClient = {
  available: true,
  async call<T>(method, params = {}, bindingId) {
    appOnlyCalls.push({ method, params, bindingId });
    return { request_id: "app-only", sequence: "4", data: {} as T };
  },
  async projection() { return appOnlyProjection; },
  async subscribe(listener: (update: AgentClientUpdate) => void) {
    listener({ type: "projection", projection: appOnlyProjection });
    return { dispose: () => undefined };
  },
};
const appOnly = useAgentWorkspace({
  tab: "settings",
  mode: null,
  projectSnapshot: ref<ProjectIdentity | null>(null),
  selection: { segmentIds: ["fixture-segment-must-not-leak"], alignmentIds: ["fixture-alignment-must-not-leak"] },
  searchState: { query: "", regex: false, caseSensitive: false, languageId: null },
  selectionSharingEnabled: true,
  canLeaveDraft: () => true,
  navigate: () => { appOnlyNavigations += 1; },
  reveal: () => undefined,
  onProjectSnapshot: () => undefined,
  onSearchState: () => undefined,
  client: appOnlyClient,
});
await appOnly.start();
await new Promise((resolve) => setTimeout(resolve, 0));
const appOnlyContext = appOnlyCalls.find((call) => call.method === "ui.publish_context");
assert.equal(appOnlyCalls.some((call) => call.method === "app.bind_session"), false, "no-project startup must not bind fixture data");
assert.equal(appOnlyContext?.bindingId, null);
assert.equal(appOnlyContext?.params.project_id, null);
assert.deepEqual(appOnlyContext?.params.segment_ids, [], "no-project startup must not leak fixture segment IDs");
assert.deepEqual(appOnlyContext?.params.alignment_ids, [], "no-project startup must not leak fixture alignment IDs");
assert.equal(appOnlyNavigations, 1, "null-project settings navigation is allowed without a project binding");
const appOnlyAcknowledgement = appOnlyCalls.find((call) => call.method === "ui.ack");
assert.equal(appOnlyAcknowledgement?.bindingId, null, "app-only action acknowledgement uses the null native binding");
appOnly.dispose();

setActivePinia(createPinia());
const revisionBeforeProjection = "revision-before-projection";
const revisionFromProjection = "revision-from-projection";
const revisionSnapshot = ref({
  project: { project_id: "project-search-order", current_revision_id: revisionBeforeProjection },
} as ProjectIdentity);
let revisionListener: ((update: AgentClientUpdate) => void) | null = null;
const acceptedSearchRevisions: string[] = [];
const revisionProjection: AgentReply<AgentProjection> = {
  request_id: "projection-before-revision-change",
  sequence: "0",
  data: { binding_id: "native-search-binding", project: null, context: null, search_spec: null, search_results: [], proposals: [] },
};
const revisionClient: AgentClient = {
  available: true,
  async call<T>(method, _params = {}, _bindingId) {
    if (method === "app.bind_session") {
      return {
        request_id: "bind-search-order",
        sequence: "1",
        data: {
          binding_id: "native-search-binding",
          project_id: "project-search-order",
          revision_id: revisionBeforeProjection,
        } as T,
      };
    }
    return { request_id: "search-order-call", sequence: "1", data: {} as T };
  },
  async projection() { return revisionProjection; },
  async subscribe(listener) {
    revisionListener = listener;
    return { dispose: () => { revisionListener = null; } };
  },
};
let revisionWorkspace!: ReturnType<typeof useAgentWorkspace>;
revisionWorkspace = useAgentWorkspace({
  tab: "search",
  mode: null,
  projectSnapshot: revisionSnapshot,
  selection: { segmentIds: [], alignmentIds: [] },
  searchState: { query: "", regex: false, caseSensitive: false, languageId: null },
  selectionSharingEnabled: true,
  canLeaveDraft: () => true,
  navigate: () => undefined,
  reveal: () => undefined,
  onProjectSnapshot: async next => {
    revisionSnapshot.value = next;
    revisionWorkspace.setProjectSnapshot(next);
  },
  onSearchState: ({ results }) => {
    const result = results as { revision_id?: string } | null;
    if (result?.revision_id === revisionSnapshot.value.project.current_revision_id) {
      acceptedSearchRevisions.push(result.revision_id);
    }
  },
  client: revisionClient,
});
await revisionWorkspace.start();
revisionListener?.({
  type: "projection",
  projection: {
    request_id: "projection-with-new-revision-search",
    sequence: "2",
    data: {
      binding_id: "native-search-binding",
      project: { project: { project_id: "project-search-order", current_revision_id: revisionFromProjection } } as ProjectIdentity,
      context: null,
      search_spec: { query: "needle", regex: false, case_sensitive: false, language_id: null },
      search_results: { revision_id: revisionFromProjection, hits: [] },
      proposals: [],
    },
  },
});
await new Promise((resolve) => setTimeout(resolve, 0));
assert.equal(revisionSnapshot.value.project.current_revision_id, revisionFromProjection, "the projection project snapshot is applied before its search state");
assert.deepEqual(acceptedSearchRevisions, [revisionFromProjection], "a search result for the projected revision reaches a revision-checking owner after the matching snapshot");
revisionWorkspace.dispose();

console.log("agent workspace publishes top-level ContextSnapshot wire params: passed");
