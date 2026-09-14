import { shallowReactive } from "vue";
import type { AlignmentDto, LanguageSide, SegmentDto, SegmentText, WorkspaceProject } from "../domain/kernel-client";

interface IndexedWorkspace {
  sourceSegments: SegmentDto[];
  targetSegments: SegmentDto[];
  alignments: AlignmentDto[];
}

const reuseArray = <T>(previous: T[], next: T[]) => previous.length === next.length
  && previous.every((value, index) => value === next[index]) ? previous : next;
const sameIds = (left: string[], right: string[]) => left.length === right.length
  && left.every((id, index) => id === right[index]);

/** Rebuildable active-pair index. Only accepted Kernel snapshots change structure. */
export function useWorkspaceIndex() {
  let scope = "";
  const segmentsById = new Map<string, SegmentDto>();
  const alignmentsById = new Map<string, AlignmentDto>();
  let previousTexts: ReadonlyMap<string, SegmentText> = new Map();
  let workspace: IndexedWorkspace = { sourceSegments: [], targetSegments: [], alignments: [] };

  const setText = (segment: SegmentDto, text: SegmentText | undefined) => {
    const matchingText = text?.content_hash === segment.contentHash ? text : undefined;
    segment.text = matchingText?.content ?? "";
    segment.loaded = matchingText !== undefined;
  };

  const updateTexts = (texts: ReadonlyMap<string, SegmentText>) => {
    // Both maps are bounded body caches. Include evictions, so cached structure
    // does not accidentally keep the full project's text resident.
    for (const id of new Set([...previousTexts.keys(), ...texts.keys()])) {
      if (previousTexts.get(id) === texts.get(id)) continue;
      const segment = segmentsById.get(id);
      if (segment) setText(segment, texts.get(id));
    }
    previousTexts = texts;
  };

  const selectProject = (
    snapshot: WorkspaceProject,
    texts: ReadonlyMap<string, SegmentText>,
    targetDocumentId?: string,
  ): IndexedWorkspace => {
    const sourceDocument = snapshot.documents[0];
    const targetDocument = snapshot.documents.find(document => document.document_id === targetDocumentId)
      ?? snapshot.documents[1];
    const nextScope = `${snapshot.project.project_id}:${sourceDocument.document_id}:${targetDocument.document_id}`;
    if (scope !== nextScope) {
      scope = nextScope;
      segmentsById.clear();
      alignmentsById.clear();
      workspace = { sourceSegments: [], targetSegments: [], alignments: [] };
    }

    const descriptors = new Map(snapshot.segments.map(segment => [segment.segment_id, segment]));
    const activeSegmentIds = new Set<string>();
    const mapSide = (documentId: string, side: LanguageSide): SegmentDto[] => {
      const entries = snapshot.segment_orders.find(order => order.document_id === documentId)?.entries ?? [];
      return entries.flatMap((entry, order) => {
        const descriptor = descriptors.get(entry.segment_id);
        if (!descriptor) return [];
        activeSegmentIds.add(entry.segment_id);
        let segment = segmentsById.get(entry.segment_id);
        if (!segment || segment.order !== order || segment.side !== side
          || segment.contentHash !== descriptor.content_hash || segment.contentLength !== descriptor.content_length) {
          // Structural fields are replaced together. Text-only hydration below
          // changes shallow reactive fields without invalidating order/ID readers.
          segment = shallowReactive({
            id: entry.segment_id, side, order,
            contentHash: descriptor.content_hash, contentLength: descriptor.content_length,
            text: "", loaded: false,
          });
          segmentsById.set(entry.segment_id, segment);
        }
        setText(segment, texts.get(entry.segment_id));
        return [segment];
      });
    };

    const sourceSegments = reuseArray(workspace.sourceSegments, mapSide(sourceDocument.document_id, "source"));
    const targetSegments = reuseArray(workspace.targetSegments, mapSide(targetDocument.document_id, "target"));
    for (const id of segmentsById.keys()) if (!activeSegmentIds.has(id)) segmentsById.delete(id);

    const activeAlignmentIds = new Set<string>();
    const alignments: AlignmentDto[] = [];
    for (const relation of snapshot.alignments) {
      if (descriptors.get(relation.target_segment_ids[0])?.document_id !== targetDocument.document_id) continue;
      activeAlignmentIds.add(relation.alignment_id);
      let alignment = alignmentsById.get(relation.alignment_id);
      if (!alignment || alignment.status !== relation.producer
        || !sameIds(alignment.sourceIds, relation.source_segment_ids)
        || !sameIds(alignment.targetIds, relation.target_segment_ids)) {
        alignment = {
          id: relation.alignment_id, status: relation.producer,
          sourceIds: [...relation.source_segment_ids], targetIds: [...relation.target_segment_ids],
        };
        alignmentsById.set(relation.alignment_id, alignment);
      }
      alignments.push(alignment);
    }
    // Unlink, Group/Ungroup, and history operations can remove or replace IDs.
    for (const id of alignmentsById.keys()) if (!activeAlignmentIds.has(id)) alignmentsById.delete(id);
    workspace = { sourceSegments, targetSegments, alignments: reuseArray(workspace.alignments, alignments) };
    previousTexts = texts;
    return workspace;
  };

  return { selectProject, updateTexts };
}
