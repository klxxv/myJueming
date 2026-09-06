import assert from "node:assert/strict";
import { createPinia, setActivePinia } from "pinia";
import { utf16CodeUnitLength } from "../../apps/desktop/src/composables/useAgentWorkspace";
import { useAgentWorkspaceStore } from "../../apps/desktop/src/stores/agent-workspace";
import type { ContextSnapshot } from "../../apps/desktop/src/domain/agent-types";

setActivePinia(createPinia());
const store = useAgentWorkspaceStore();

const context = (projectId: string, selectedText: string): ContextSnapshot => ({
  tab: "parallel",
  mode: "review",
  project_id: projectId,
  revision_id: "9007199254740993",
  binding_id: `binding-${projectId}`,
  window_focused: true,
  focused_control: "segment-content",
  selected_text: selectedText,
  segment_ids: [`segment-${projectId}`],
  alignment_ids: [],
  text_range: { start_utf16: 1, end_utf16: 3 },
  captured_at: "2026-09-06T00:00:00.000Z",
});

// An astral character occupies two UTF-16 code units even though it is one visible character.
assert.equal(utf16CodeUnitLength("汉😀A"), 4);

const alphaContext = context("alpha", "汉😀");
store.recordOutboundMessage("alpha", {
  id: "message-alpha",
  requestId: "request-alpha",
  content: "审阅当前选段",
  sentAt: "2026-09-06T00:00:01.000Z",
  status: "sending",
}, alphaContext);
store.setDraft("alpha", "annotation:alpha", "alpha draft");
store.setAnnotationScope("alpha", { segmentIds: ["segment-alpha"], alignmentIds: ["alignment-alpha"] });

store.recordOutboundMessage("beta", {
  id: "message-beta",
  requestId: "request-beta",
  content: "review current selection",
  sentAt: "2026-09-06T00:00:02.000Z",
  status: "sending",
}, context("beta", "plain"));
store.setDraft("beta", "annotation:beta", "beta draft");

assert.equal(store.projectScopes.alpha.chat.length, 1, "project chat must remain project-scoped");
assert.equal(store.projectScopes.beta.chat.length, 1, "second project must not replace the first chat");
assert.equal(store.projectScopes.alpha.drafts["annotation:alpha"], "alpha draft");
assert.equal(store.projectScopes.beta.drafts["annotation:alpha"], undefined);
assert.deepEqual(store.projectScopes.alpha.annotationScope.segmentIds, ["segment-alpha"]);
assert.equal(store.projectScopes.alpha.chat[0].context.selected_text, "汉😀", "message captures the send-time context");

alphaContext.selected_text = "mutated after send";
alphaContext.segment_ids.push("segment-mutated");
assert.equal(store.projectScopes.alpha.chat[0].context.selected_text, "汉😀", "stored context is immutable");
assert.deepEqual(store.projectScopes.alpha.chat[0].context.segment_ids, ["segment-alpha"]);

console.log("agent context UTF-16, immutable sends, and project isolation: passed");
