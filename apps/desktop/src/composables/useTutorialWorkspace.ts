import { computed, ref, type Ref } from "vue";
import { governmentSource, governmentTarget } from "../data/governmentFixture";
import type { AlignmentDto, LanguageSide, SegmentDto } from "../domain/kernel-client";

interface TutorialSnapshot {
  source: SegmentDto[];
  target: SegmentDto[];
  alignments: AlignmentDto[];
}

const initialSnapshot = (): TutorialSnapshot => ({
  source: governmentSource.map((text, order) => ({ id: `tutorial-source-${order + 1}`, side: "source", text, order, loaded: true, contentLength: text.length, contentHash: `tutorial-source-hash-${order + 1}` })),
  target: governmentTarget.map((text, order) => ({ id: `tutorial-target-${order + 1}`, side: "target", text, order, loaded: true, contentLength: text.length, contentHash: `tutorial-target-hash-${order + 1}` })),
  alignments: [
    { id: "tutorial-alignment-1", sourceIds: ["tutorial-source-1"], targetIds: ["tutorial-target-1"], status: "manual" },
    { id: "tutorial-alignment-2", sourceIds: ["tutorial-source-2"], targetIds: ["tutorial-target-2"], status: "manual" },
    { id: "tutorial-alignment-complex", sourceIds: ["tutorial-source-3", "tutorial-source-4"], targetIds: ["tutorial-target-3", "tutorial-target-4"], status: "manual" },
    { id: "tutorial-alignment-6", sourceIds: ["tutorial-source-6"], targetIds: ["tutorial-target-6"], status: "manual" },
    { id: "tutorial-alignment-7", sourceIds: ["tutorial-source-7"], targetIds: ["tutorial-target-7"], status: "manual" },
    { id: "tutorial-alignment-8", sourceIds: ["tutorial-source-8"], targetIds: ["tutorial-target-8"], status: "manual" },
  ],
});
const clone = (snapshot: TutorialSnapshot): TutorialSnapshot => ({
  source: snapshot.source.map(segment => ({ ...segment })),
  target: snapshot.target.map(segment => ({ ...segment })),
  alignments: snapshot.alignments.map(alignment => ({ ...alignment, sourceIds: [...alignment.sourceIds], targetIds: [...alignment.targetIds] })),
});

export function useTutorialWorkspace(source: Ref<SegmentDto[]>, target: Ref<SegmentDto[]>, alignments: Ref<AlignmentDto[]>) {
  const history = ref<TutorialSnapshot[]>([]);
  const cursor = ref(-1);
  const revision = ref(0);
  let idSequence = 0;
  const current = (): TutorialSnapshot => ({ source: source.value, target: target.value, alignments: alignments.value });
  const apply = (snapshot: TutorialSnapshot) => {
    const value = clone(snapshot);
    source.value = value.source;
    target.value = value.target;
    alignments.value = value.alignments;
    revision.value++;
  };
  const remember = () => {
    history.value = [...history.value.slice(0, cursor.value + 1), clone(current())];
    cursor.value = history.value.length - 1;
    revision.value++;
  };
  const mutate = (change: () => void) => { change(); remember(); };
  const normalize = (side: LanguageSide) => {
    const rows = side === "source" ? source : target;
    rows.value = rows.value.map((segment, order) => ({ ...segment, order }));
  };
  const start = () => {
    idSequence = 0;
    apply(initialSnapshot());
    history.value = [clone(current())];
    cursor.value = 0;
  };
  const reset = start;
  const undo = () => { if (cursor.value > 0) { cursor.value--; apply(history.value[cursor.value]); } };
  const redo = () => { if (cursor.value + 1 < history.value.length) { cursor.value++; apply(history.value[cursor.value]); } };
  const edit = (segmentId: string, text: string) => mutate(() => {
    for (const rows of [source, target]) rows.value = rows.value.map(segment => segment.id === segmentId
      ? { ...segment, text, contentLength: text.length, contentHash: `tutorial-edit-${++idSequence}` }
      : segment);
  });
  const reorder = (side: LanguageSide, orderedIds: string[]) => mutate(() => {
    const rows = side === "source" ? source : target;
    const byId = new Map(rows.value.map(segment => [segment.id, segment]));
    rows.value = orderedIds.flatMap(id => byId.get(id) ?? []).map((segment, order) => ({ ...segment, order }));
  });
  const move = (side: LanguageSide, segmentId: string, direction: "up" | "down") => {
    const rows = side === "source" ? source.value : target.value;
    const index = rows.findIndex(segment => segment.id === segmentId);
    const destination = index + (direction === "up" ? -1 : 1);
    if (index < 0 || destination < 0 || destination >= rows.length) return false;
    const ids = rows.map(segment => segment.id);
    [ids[index], ids[destination]] = [ids[destination], ids[index]];
    reorder(side, ids);
    return true;
  };
  const resetOrder = () => mutate(() => {
    const baseline = initialSnapshot();
    for (const [side, rows, ordered] of [["source", source, baseline.source], ["target", target, baseline.target]] as const) {
      const rank = new Map(ordered.map((segment, index) => [segment.id, index]));
      rows.value = [...rows.value].sort((a, b) => (rank.get(a.id) ?? Number.MAX_SAFE_INTEGER) - (rank.get(b.id) ?? Number.MAX_SAFE_INTEGER));
      normalize(side);
    }
  });
  const link = (sourceIds: string[], targetIds: string[]) => {
    const occupied = new Set([...sourceIds, ...targetIds]);
    const id = `tutorial-alignment-new-${++idSequence}`;
    mutate(() => {
      alignments.value = alignments.value.filter(alignment => ![...alignment.sourceIds, ...alignment.targetIds].some(segmentId => occupied.has(segmentId)));
      alignments.value = [...alignments.value, { id, sourceIds: [...sourceIds], targetIds: [...targetIds], status: "manual" }];
    });
    return id;
  };
  const unlink = (alignmentId: string) => mutate(() => { alignments.value = alignments.value.filter(alignment => alignment.id !== alignmentId); });
  const group = (alignmentIds: string[], unlinkedIds: string[]) => {
    const selected = alignments.value.filter(alignment => alignmentIds.includes(alignment.id));
    const sourceSet = new Set(selected.flatMap(alignment => alignment.sourceIds));
    const targetSet = new Set(selected.flatMap(alignment => alignment.targetIds));
    const sourceIds = new Set(source.value.map(segment => segment.id));
    for (const id of unlinkedIds) (sourceIds.has(id) ? sourceSet : targetSet).add(id);
    const ordered = (side: LanguageSide, ids: Set<string>) => (side === "source" ? source.value : target.value).filter(segment => ids.has(segment.id)).map(segment => segment.id);
    const id = `tutorial-alignment-group-${++idSequence}`;
    mutate(() => {
      alignments.value = [...alignments.value.filter(alignment => !alignmentIds.includes(alignment.id)), {
        id, sourceIds: ordered("source", sourceSet), targetIds: ordered("target", targetSet), status: "manual",
      }];
    });
    return id;
  };
  const ungroup = (alignmentId: string, sourceGroups: string[][], targetGroups: string[][]) => mutate(() => {
    const count = Math.max(sourceGroups.length, targetGroups.length);
    const replacements = Array.from({ length: count }, (_, index) => ({
      id: `tutorial-alignment-ungroup-${++idSequence}`,
      sourceIds: [...(sourceGroups[index] ?? [])], targetIds: [...(targetGroups[index] ?? [])], status: "manual" as const,
    })).filter(alignment => alignment.sourceIds.length && alignment.targetIds.length);
    alignments.value = [...alignments.value.filter(alignment => alignment.id !== alignmentId), ...replacements];
  });
  const merge = (segmentIds: string[], content: string) => mutate(() => {
    const ids = new Set(segmentIds);
    const rows = source.value.some(segment => ids.has(segment.id)) ? source : target;
    const selected = rows.value.filter(segment => ids.has(segment.id)).sort((a, b) => a.order - b.order);
    const keeper = selected[0];
    if (!keeper) return;
    rows.value = rows.value.filter(segment => !ids.has(segment.id) || segment.id === keeper.id).map(segment => segment.id === keeper.id
      ? { ...segment, text: content, contentLength: content.length, contentHash: `tutorial-merge-${++idSequence}` }
      : segment);
    normalize(keeper.side);
    alignments.value = alignments.value.map(alignment => {
      const replace = (members: string[]) => [...new Set(members.map(id => ids.has(id) ? keeper.id : id))];
      return { ...alignment, sourceIds: replace(alignment.sourceIds), targetIds: replace(alignment.targetIds) };
    });
  });
  const split = (segmentId: string, parts: string[]) => mutate(() => {
    const rows = source.value.some(segment => segment.id === segmentId) ? source : target;
    const index = rows.value.findIndex(segment => segment.id === segmentId);
    if (index < 0) return;
    const original = rows.value[index];
    const pieces = parts.map((text, part) => part === 0
      ? { ...original, text, contentLength: text.length, contentHash: `tutorial-split-${++idSequence}` }
      : { ...original, id: `tutorial-${original.side}-split-${++idSequence}`, text, contentLength: text.length, contentHash: `tutorial-split-${idSequence}` });
    rows.value = [...rows.value.slice(0, index), ...pieces, ...rows.value.slice(index + 1)];
    normalize(original.side);
    alignments.value = alignments.value.map(alignment => {
      const expand = (members: string[]) => members.flatMap(id => id === segmentId ? pieces.map(piece => piece.id) : id);
      return { ...alignment, sourceIds: expand(alignment.sourceIds), targetIds: expand(alignment.targetIds) };
    });
  });
  const insertGap = (segmentId: string) => {
    const alignment = alignments.value.find(item => [...item.sourceIds, ...item.targetIds].includes(segmentId));
    if (alignment) unlink(alignment.id);
  };

  return {
    revision, canUndo: computed(() => cursor.value > 0), canRedo: computed(() => cursor.value + 1 < history.value.length),
    start, reset, undo, redo, edit, move, reorder, resetOrder, link, unlink, group, ungroup, merge, split, insertGap,
  };
}
