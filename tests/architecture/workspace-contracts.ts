import assert from "node:assert/strict";
import { createRenderer, ref } from "vue";
import { useViewModeController } from "../../apps/desktop/src/composables/useViewModeController";
import { useWorkspaceText } from "../../apps/desktop/src/composables/useWorkspaceText";
import type { CommandContext, WorkspaceProject, ParallelSlice } from "../../apps/desktop/src/domain/kernel-client";

// Mount the composable so its disposal hooks have a real component lifecycle.
const renderer = createRenderer<object, object>({
  createElement: () => ({}), createText: () => ({}), createComment: () => ({}),
  insert() {}, remove() {}, setText() {}, setElementText() {}, patchProp() {},
  parentNode: () => null, nextSibling: () => null,
});
const disposers: Array<() => void> = [];
const mountEditor = (options: Parameters<typeof useViewModeController>[0]) => {
  let controller!: ReturnType<typeof useViewModeController>;
  const app = renderer.createApp({ setup() { controller = useViewModeController(options); return () => null; } });
  app.mount({}); disposers.push(() => app.unmount());
  return controller;
};

let saveFailure = true;
const contexts: CommandContext[] = [];
const editor = mountEditor({ autosaveDelayMs: ref(60_000), scope: () => ({ project_id: "project", base_revision_id: "1" }), persist: async (_id, _text, context) => {
  contexts.push(context!);
  if (saveFailure) throw new Error("disk unavailable");
  return { project_id: "project", base_revision_id: "2" };
}});
editor.enterEdit("segment", "alignment", "original");
editor.updateDraft("unsaved draft");
let left = false;
const leave = editor.guardProjectChange().then(allowed => { left = allowed; });
assert.equal(await editor.confirmPendingWithSave(), false);
assert.equal(left, false);
assert.equal(editor.editSession.value?.draft, "unsaved draft");
assert.equal(editor.editSession.value?.status, "error");
saveFailure = false;
assert.equal(await editor.confirmPendingWithSave(), true);
await leave;
assert.equal(left, true);
assert.equal(contexts[0].command_id, contexts[1].command_id, "retry retains command identity");
assert.equal(contexts[1].base_revision_id, "1", "retry retains the draft's original revision");
assert.equal(editor.editSession.value, null);

let finishSave: () => void = () => undefined;
const saving = mountEditor({ autosaveDelayMs: ref(60_000), persist: () => new Promise<void>(resolve => { finishSave = resolve; }) });
saving.enterEdit("s", "a", "before"); saving.updateDraft("after");
const inFlight = saving.persistDraft(false);
const switching = saving.guardProjectChange();
let discarded = false;
const discard = saving.confirmPendingWithDiscard().then(() => { discarded = true; });
await Promise.resolve(); assert.equal(discarded, false, "discard cannot switch projects during an in-flight write");
finishSave(); await inFlight; await discard;
assert.equal(await switching, true);
saving.enterEdit("s", "a", "before"); saving.updateDraft("keep me");
saving.activeMode.value = "review";
const cancelled = saving.guardProjectChange(); saving.cancelPendingTransition();
assert.equal(await cancelled, false); assert.equal(saving.editSession.value?.draft, "keep me"); saving.discardAndExit();

const project = (revision = "1", hash = "h"): WorkspaceProject => ({
  project: { project_id: "project", current_revision_id: revision },
  documents: [{ document_id: "source" }, { document_id: "target" }],
  segments: Array.from({ length: 1600 }, (_, i) => ({ segment_id: String(i), document_id: "source", content_hash: hash, content_length: 10 })),
  segment_orders: [{ entries: Array.from({ length: 1600 }, (_, i) => ({ segment_id: String(i) })) }],
} as WorkspaceProject);
const batches: number[] = [];
let resolveLate: ((slice: ParallelSlice) => void) | null = null;
let delay = false;
const cache = useWorkspaceText({ loadParallelSlice: async (view, ids) => {
  batches.push(ids.length);
  if (delay) return new Promise(resolve => { resolveLate = resolve; });
  return { project_id: view.project.project_id, revision_id: view.project.current_revision_id, segments: ids.map(id => ({ segment_id: id, content: `text ${id}`, content_hash: view.segments[0].content_hash })) };
}});
cache.selectProject(project());
await cache.visible(["0", "1"]);
assert.equal(cache.texts.value.size, 2, "initial viewport does not load the whole project");
for (let i = 2; i < 1600; i += 200) await cache.visible(Array.from({ length: Math.min(200, 1600 - i) }, (_, n) => String(i + n)));
assert.ok(cache.texts.value.size <= 1200, "body cache is bounded");
assert.ok(batches.every(size => size <= 200));
const retained = cache.texts.value.size;
cache.selectProject(project("2")); assert.equal(cache.texts.value.size, retained, "unchanged text survives revision invalidation");
const found = await cache.find("text 1599"); assert.deepEqual(found, ["1599"]); assert.equal(cache.texts.value.size, retained, "view find does not fill the body cache");
cache.selectProject(project("3", "new-hash")); assert.equal(cache.texts.value.size, 0);
delay = true;
const late = cache.ensure(["0"]); cache.selectProject(project("3", "new-hash")); cache.selectProject(project("4", "newest"));
resolveLate!({ project_id: "project", revision_id: "3", segments: [{ segment_id: "0", content: "old reply", content_hash: "new-hash" }] });
await late; assert.equal(cache.texts.value.size, 0, "stale slice responses cannot repopulate a new revision");
disposers.forEach(dispose => dispose());
console.log("project guards, revision-bound edit retries, bounded slices, and stale-response rejection: passed");
