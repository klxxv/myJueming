import assert from "node:assert/strict";
import { computed, createRenderer, ref } from "vue";
import { useWorkspaceIndex } from "../../apps/desktop/src/composables/useWorkspaceIndex";
import { buildAlignmentBlocks } from "../../apps/desktop/src/domain/workspace-projection";
import { useAlignedBlockLayout } from "../../apps/desktop/src/components/parallel-workspace/useAlignedBlockLayout";
import type { AlignmentDto, CoreAlignment, SegmentDto, SegmentText, WorkspaceProject } from "../../apps/desktop/src/domain/kernel-client";

const relation = (id: string, source: string[], target: string[]): CoreAlignment => ({
  alignment_id: id, source_segment_ids: source, target_segment_ids: target,
  producer: "manual", cardinality: "n:m", project_id: "project",
  created_revision_id: "1", updated_revision_id: "1",
});
const initial = {
  project: { project_id: "project", current_revision_id: "1" },
  documents: ["source", "target", "second-target"].map(document_id => ({ document_id })),
  segments: ["s", "t", "u"].flatMap((side, document) => Array.from({ length: 6 }, (_, order) => ({
    segment_id: `${side}${order}`, document_id: ["source", "target", "second-target"][document],
    content_hash: `h-${side}${order}`, content_length: 20,
  }))),
  segment_orders: ["s", "t", "u"].map((side, document) => ({
    document_id: ["source", "target", "second-target"][document],
    entries: Array.from({ length: 6 }, (_, order) => ({ segment_id: `${side}${order}`, position_key: String(order) })),
  })),
  alignments: [relation("one", ["s0"], ["t0"]), relation("one-many", ["s1"], ["t1", "t2"]),
    relation("many-one", ["s2", "s3"], ["t3"]), relation("other-pair", ["s0"], ["u0"])],
} as WorkspaceProject;
const text = (id: string, content = `body ${id}`, hash = `h-${id}`): SegmentText => ({ segment_id: id, content, content_hash: hash });
const index = useWorkspaceIndex();
const texts = new Map([["s0", text("s0")]]);
const first = index.selectProject(initial, texts);
assert.equal(first.sourceSegments[0].text, "body s0");
assert.equal(first.targetSegments[0].loaded, false);
assert.deepEqual(first.alignments.map(item => item.id), ["one", "one-many", "many-one"]);

// This is the actual App -> ParallelWorkspace reactive dependency chain.
const source = ref(first.sourceSegments), target = ref(first.targetSegments), alignments = ref(first.alignments);
let builds = 0;
const rows = computed(() => { builds++; return buildAlignmentBlocks(source.value, target.value, alignments.value); });
const firstRows = rows.value;
const renderedText = computed(() => rows.value.flatMap(row => row.sourceSegments.map(segment => segment.text)).join("|"));
const beforeText = renderedText.value;
const hydrated = new Map([...texts, ["s1", text("s1")], ["t0", text("t0")]]);
index.updateTexts(hydrated);
assert.notEqual(renderedText.value, beforeText, "hydrated text reaches reactive consumers");
assert.equal(rows.value, firstRows, "body loading keeps the projection and block identities");
assert.equal(builds, 1, "scroll hydration must not rebuild alignment structure");
index.updateTexts(new Map([["s1", text("s1")]]));
assert.equal(first.sourceSegments[0].text, "", "body eviction clears the indexed object too");
assert.equal(first.sourceSegments[0].loaded, false);
assert.equal(rows.value, firstRows);

const unchanged = index.selectProject({ ...structuredClone(initial), project: { ...initial.project, current_revision_id: "2" } }, texts);
assert.equal(unchanged.sourceSegments, first.sourceSegments, "unchanged revisions reuse segment arrays");
assert.equal(unchanged.alignments, first.alignments, "unchanged revisions reuse relation arrays");
const added = structuredClone(initial);
added.alignments.push(relation("many-many", ["s4", "s5"], ["t4", "t5"]));
const withAdded = index.selectProject(added, texts);
assert.equal(withAdded.alignments[0], first.alignments[0], "adding an alignment retains unchanged index entries");
assert.deepEqual(withAdded.alignments.map(item => item.id), ["one", "one-many", "many-one", "many-many"]);
assert.equal(withAdded.sourceSegments, first.sourceSegments);

const grouped = structuredClone(added);
grouped.alignments = grouped.alignments.filter(item => !["one", "one-many"].includes(item.alignment_id));
grouped.alignments.push(relation("grouped", ["s0", "s1"], ["t0", "t1", "t2"]));
const withGroup = index.selectProject(grouped, texts);
assert.ok(!withGroup.alignments.some(item => ["one", "one-many"].includes(item.id)), "replaced relationships leave the active index");
assert.deepEqual(withGroup.alignments.find(item => item.id === "grouped")?.targetIds, ["t0", "t1", "t2"]);
const unlinked = structuredClone(grouped);
unlinked.alignments = unlinked.alignments.filter(item => item.alignment_id !== "grouped");
const withoutGroup = index.selectProject(unlinked, texts);
assert.ok(!withoutGroup.alignments.some(item => item.id === "grouped"));
assert.ok(buildAlignmentBlocks(withoutGroup.sourceSegments, withoutGroup.targetSegments, withoutGroup.alignments)
  .some(row => !row.linked && row.sourceSegments.some(segment => segment.id === "s0")));
const undone = index.selectProject(added, texts);
assert.deepEqual(undone.alignments.map(item => item.id), withAdded.alignments.map(item => item.id), "Undo/Restore rebuild removed active entries from canonical state");

const split = structuredClone(added);
split.segments.push({ segment_id: "s0-split", document_id: "source", content_hash: "split-hash", content_length: 10 });
split.segment_orders[0].entries.splice(1, 0, { segment_id: "s0-split", position_key: "0.5" });
split.alignments.find(item => item.alignment_id === "one")!.source_segment_ids.push("s0-split");
const withSplit = index.selectProject(split, texts);
assert.equal(withSplit.sourceSegments[0].id, "s0");
assert.equal(withSplit.sourceSegments[1].id, "s0-split");
assert.deepEqual(withSplit.alignments.find(item => item.id === "one")?.sourceIds, ["s0", "s0-split"]);
const merged = index.selectProject(added, texts);
assert.ok(!merged.sourceSegments.some(item => item.id === "s0-split"), "Merge discards absorbed segment IDs");
assert.deepEqual(merged.alignments.find(item => item.id === "one")?.sourceIds, ["s0"]);

const moved = structuredClone(added);
moved.segment_orders[0].entries.unshift(moved.segment_orders[0].entries.pop()!);
const reordered = index.selectProject(moved, texts);
assert.equal(reordered.sourceSegments[0].id, "s5");
assert.equal(reordered.sourceSegments[0].order, 0);
assert.equal(reordered.alignments, merged.alignments, "reordering retains relationship identities and membership");
const edited = structuredClone(moved);
edited.segments.find(item => item.segment_id === "s0")!.content_hash = "edited";
const afterEdit = index.selectProject(edited, texts);
assert.equal(afterEdit.sourceSegments.find(item => item.id === "s0")!.loaded, false, "old-hash bodies cannot survive an edit");
index.updateTexts(new Map([["s0", text("s0", "edited body", "edited")]]));
assert.equal(afterEdit.sourceSegments.find(item => item.id === "s0")!.text, "edited body");

const secondPair = index.selectProject(initial, texts, "second-target");
assert.deepEqual(secondPair.alignments.map(item => item.id), ["other-pair"]);
assert.ok(secondPair.targetSegments.every(item => item.id.startsWith("u")), "same-language translations use DocumentId scope");
const newProject = structuredClone(initial); newProject.project.project_id = "new-project";
const opened = index.selectProject(newProject, new Map());
assert.notEqual(opened.sourceSegments[0], secondPair.sourceSegments[0]);
assert.equal(opened.sourceSegments[0].text, "", "opening another project clears cached body ownership");

// Count structural property reads instead of imposing hardware-dependent timings.
let orderReads = 0;
const countedSide = (side: "source" | "target"): SegmentDto[] => Array.from({ length: 300 }, (_, order) => ({
  id: `${side}-${order}`, side, text: "text", get order() { orderReads++; return order; },
}));
const countedSource = countedSide("source"), countedTarget = countedSide("target");
const countedRelations: AlignmentDto[] = countedSource.map((segment, i) => ({
  id: `relation-${i}`, sourceIds: [segment.id], targetIds: [countedTarget[i].id], status: "manual",
}));
const countedRows = buildAlignmentBlocks(countedSource, countedTarget, countedRelations);
assert.equal(countedRows.length, 300);
assert.ok(orderReads < 300 * 30, `1:1 projection must not scan every document for every relation: ${orderReads}`);

const renderer = createRenderer<object, object>({
  createElement: () => ({}), createText: () => ({}), createComment: () => ({}),
  insert() {}, remove() {}, setText() {}, setElementText() {}, patchProp() {}, parentNode: () => null, nextSibling: () => null,
});
let layout!: ReturnType<typeof useAlignedBlockLayout>;
const scrollTop = ref(0), viewportHeight = ref(700);
const app = renderer.createApp({ setup() {
  layout = useAlignedBlockLayout({ rows: computed(() => countedRows), viewportWidth: ref(1400), scrollTop, viewportHeight });
  return () => null;
} });
app.mount({});
for (let i = 0; i < 20; i++) layout.reportHeight(`relation-${i}`, "source", 76 + i * 17);
for (const offset of [-100, 0, 76, 720, 1459, 2500, 26_000, 1_000_000]) {
  scrollTop.value = offset;
  const expected = layout.positions.value.filter(position => position.bandTop + position.bandHeight >= Math.max(0, offset - 720)
    && position.bandTop <= offset + viewportHeight.value + 720);
  assert.deepEqual(layout.visiblePositions.value.map(position => position.block.alignmentId), expected.map(position => position.block.alignmentId));
}
app.unmount();
console.log("workspace index: reactive body updates without reprojection; cache eviction, canonical relation changes, split/merge, reorder, history, project/pair scope, linear 1:1 indexing, and viewport boundaries passed");
