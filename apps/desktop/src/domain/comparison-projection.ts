import type { CoreAlignment, CoreDocument, SegmentDto, SegmentText, WorkspaceProject } from './kernel-client';
import { buildAlignmentBlocks } from './workspace-projection';

export interface ComparisonBand {
  key: string;
  cells: Map<string, SegmentDto[]>;
  relations: CoreAlignment[];
}
export interface ComparisonDocument { document: CoreDocument; segments: SegmentDto[] }

export function comparisonDocuments(project: WorkspaceProject, texts: ReadonlyMap<string, SegmentText>): ComparisonDocument[] {
  const descriptors = new Map(project.segments.map(segment => [segment.segment_id, segment]));
  return project.documents.map((document, index) => ({
    document,
    segments: (project.segment_orders.find(order => order.document_id === document.document_id)?.entries ?? []).flatMap((entry, order) => {
      const segment = descriptors.get(entry.segment_id);
      return segment ? [{ id: segment.segment_id, side: index === 0 ? 'source' as const : 'target' as const, order,
        text: texts.get(segment.segment_id)?.content ?? '', loaded: texts.has(segment.segment_id), contentLength: segment.content_length, contentHash: segment.content_hash }] : [];
    }),
  }));
}

/** Presentation bands are connected pairwise blocks, never new canonical alignments. */
export function buildComparisonBands(documents: ComparisonDocument[], alignments: CoreAlignment[]): ComparisonBand[] {
  if (documents.length < 2) return [];
  if (documents.length === 2) {
    const targetIds = new Set(documents[1].segments.map(segment => segment.id));
    const pair = alignments.filter(relation => targetIds.has(relation.target_segment_ids[0]));
    const byId = new Map(pair.map(relation => [relation.alignment_id, relation]));
    return buildAlignmentBlocks(documents[0].segments, documents[1].segments, pair.map(relation => ({ id: relation.alignment_id, sourceIds: relation.source_segment_ids, targetIds: relation.target_segment_ids, status: relation.producer }))).map(block => ({
      key: block.alignmentId,
      cells: new Map([[documents[0].document.document_id, block.sourceSegments], [documents[1].document.document_id, block.targetSegments]]),
      relations: byId.has(block.alignmentId) ? [byId.get(block.alignmentId)!] : [],
    }));
  }
  const parents = new Map<string, string>();
  for (const { segments } of documents) for (const segment of segments) parents.set(segment.id, segment.id);
  const root = (id: string): string => {
    let result = id;
    while (parents.get(result) && parents.get(result) !== result) result = parents.get(result)!;
    while (parents.has(id) && parents.get(id) !== result) { const next = parents.get(id)!; parents.set(id, result); id = next; }
    return result;
  };
  const unite = (ids: string[]) => { const first = ids[0]; if (!first) return; for (const id of ids.slice(1)) parents.set(root(id), root(first)); };
  const source = documents[0].segments;
  for (const target of documents.slice(1)) {
    const targetIds = new Set(target.segments.map(segment => segment.id));
    const pair = alignments.filter(alignment => targetIds.has(alignment.target_segment_ids[0]));
    // Preserve the existing two-column treatment of unmatched rows as well.
    for (const block of buildAlignmentBlocks(source, target.segments, pair.map(alignment => ({ id: alignment.alignment_id, sourceIds: alignment.source_segment_ids, targetIds: alignment.target_segment_ids, status: alignment.producer })))) {
      unite([...block.sourceSegments, ...block.targetSegments].map(segment => segment.id));
    }
  }
  const bands = new Map<string, ComparisonBand>();
  for (const { document, segments } of documents) for (const segment of segments) {
    const key = root(segment.id);
    let band = bands.get(key);
    if (!band) { band = { key, cells: new Map(), relations: [] }; bands.set(key, band); }
    const cell = band.cells.get(document.document_id) ?? [];
    cell.push(segment); band.cells.set(document.document_id, cell);
  }
  for (const relation of alignments) bands.get(root(relation.source_segment_ids[0]))?.relations.push(relation);
  const edges = new Map([...bands.keys()].map(key => [key, new Set<string>()]));
  const degrees = new Map([...bands.keys()].map(key => [key, 0]));
  for (const { segments } of documents) {
    let previous: string | undefined;
    for (const segment of segments) {
      const key = root(segment.id);
      if (previous && previous !== key && !edges.get(previous)!.has(key)) { edges.get(previous)!.add(key); degrees.set(key, degrees.get(key)! + 1); }
      previous = key;
    }
  }
  const score = (key: string) => Math.min(...[...bands.get(key)!.cells.values()].flat().map(segment => segment.order));
  const compare = (a: string, b: string) => score(a) - score(b) || a.localeCompare(b);
  const ready = [...degrees.keys()].filter(key => degrees.get(key) === 0).sort(compare);
  const ordered: string[] = [];
  while (ready.length) { const key = ready.shift()!; ordered.push(key); for (const next of edges.get(key)!) { degrees.set(next, degrees.get(next)! - 1); if (!degrees.get(next)) { ready.push(next); ready.sort(compare); } } }
  const emitted = new Set(ordered);
  ordered.push(...[...bands.keys()].filter(key => !emitted.has(key)).sort(compare));
  return ordered.map(key => bands.get(key)!);
}

export interface ComparisonSegmentPosition { top: number; height: number }
export interface ComparisonBandLayout { height: number; segments: Map<string, ComparisonSegmentPosition>; crossed: boolean }

/** Align relation starts, while each document contributes its own measured-height constraints.
 * Overlapping n:m relations share a band but must not flatten its internal anchors.
 * Cyclic (crossed) constraints retain document order and expose the conflict instead of changing links.
 */
export function layoutComparisonBand(band: ComparisonBand, heights: ReadonlyMap<string, number>): ComparisonBandLayout {
  const all = [...band.cells.values()].flat();
  const byId = new Map(all.map(segment => [segment.id, segment]));
  const parents = new Map(all.map(segment => [segment.id, segment.id]));
  const root = (id: string): string => {
    let current = id;
    while (parents.get(current) && parents.get(current) !== current) current = parents.get(current)!;
    while (parents.has(id) && parents.get(id) !== current) { const next = parents.get(id)!; parents.set(id, current); id = next; }
    return current;
  };
  const first = (ids: string[]) => ids.filter(id => byId.has(id)).sort((a, b) => byId.get(a)!.order - byId.get(b)!.order)[0];
  for (const relation of band.relations) {
    const source = first(relation.source_segment_ids), target = first(relation.target_segment_ids);
    if (source && target) parents.set(root(target), root(source));
  }
  const keys = new Set(all.map(segment => root(segment.id)));
  const edges = new Map([...keys].map(key => [key, new Map<string, number>()]));
  const degrees = new Map([...keys].map(key => [key, 0]));
  let selfCycle = false;
  for (const segments of band.cells.values()) for (let index = 1; index < segments.length; index++) {
    const previous = segments[index - 1], next = segments[index];
    const from = root(previous.id), to = root(next.id);
    if (from === to) { selfCycle = true; continue; }
    const outgoing = edges.get(from)!;
    if (!outgoing.has(to)) degrees.set(to, degrees.get(to)! + 1);
    outgoing.set(to, Math.max(outgoing.get(to) ?? 0, heights.get(previous.id) ?? 114));
  }
  const ready = [...keys].filter(key => !degrees.get(key));
  const offsets = new Map([...keys].map(key => [key, 0]));
  let processed = 0;
  for (let index = 0; index < ready.length; index++) {
    const key = ready[index]; processed++;
    for (const [next, distance] of edges.get(key)!) {
      offsets.set(next, Math.max(offsets.get(next)!, offsets.get(key)! + distance));
      degrees.set(next, degrees.get(next)! - 1);
      if (!degrees.get(next)) ready.push(next);
    }
  }
  const crossed = selfCycle || processed !== keys.size;
  const segments = new Map<string, ComparisonSegmentPosition>();
  let height = 0;
  for (const cell of band.cells.values()) {
    let fallbackTop = 0;
    for (const segment of cell) {
      const segmentHeight = heights.get(segment.id) ?? 114;
      const top = crossed ? fallbackTop : offsets.get(root(segment.id))!;
      segments.set(segment.id, { top, height: segmentHeight });
      fallbackTop = top + segmentHeight; height = Math.max(height, fallbackTop);
    }
  }
  return { height, segments, crossed };
}
