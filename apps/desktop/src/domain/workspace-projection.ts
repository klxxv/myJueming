import type { AlignmentDto, LanguageSide, SegmentDto } from "./kernel-client";

export interface OrderSelection {
  side: LanguageSide;
  segmentId: string;
}

export interface AlignmentBlockView {
  sourceSegments: SegmentDto[];
  targetSegments: SegmentDto[];
  alignmentId: string;
  index: number;
  linked: boolean;
  sourceFragmented: boolean;
  targetFragmented: boolean;
}

const sideIsFragmented = (segmentIds: string[], segments: SegmentDto[]) => {
  const orderById = new Map(segments.map((segment) => [segment.id, segment.order]));
  const orders = segmentIds
    .map((segmentId) => orderById.get(segmentId))
    .filter((order): order is number => order !== undefined)
    .sort((a, b) => a - b);
  return orders.length > 1 && orders[orders.length - 1] - orders[0] + 1 !== orders.length;
};

const orderAlignmentBlocks = (
  unorderedBlocks: AlignmentBlockView[],
  sourceSegments: SegmentDto[],
  targetSegments: SegmentDto[],
) => {
  const blockIndexBySegmentId = new Map<string, number>();
  unorderedBlocks.forEach((block, blockIndex) => {
    [...block.sourceSegments, ...block.targetSegments]
      .forEach((segment) => blockIndexBySegmentId.set(segment.id, blockIndex));
  });

  const outgoing = unorderedBlocks.map(() => new Set<number>());
  const indegree = unorderedBlocks.map(() => 0);
  const addSideConstraints = (segments: SegmentDto[]) => {
    const ordered = [...segments].sort((a, b) => a.order - b.order);
    let previousBlockIndex: number | undefined;
    for (const segment of ordered) {
      const blockIndex = blockIndexBySegmentId.get(segment.id);
      if (blockIndex === undefined) continue;
      if (previousBlockIndex !== undefined
        && previousBlockIndex !== blockIndex
        && !outgoing[previousBlockIndex].has(blockIndex)) {
        outgoing[previousBlockIndex].add(blockIndex);
        indegree[blockIndex] += 1;
      }
      previousBlockIndex = blockIndex;
    }
  };
  addSideConstraints(sourceSegments);
  addSideConstraints(targetSegments);

  const blockScore = (blockIndex: number) => {
    const block = unorderedBlocks[blockIndex];
    const ranks = [...block.sourceSegments, ...block.targetSegments].map((segment) => segment.order);
    return ranks.length ? Math.min(...ranks) : Number.MAX_SAFE_INTEGER;
  };
  const compareBlocks = (a: number, b: number) => blockScore(a) - blockScore(b)
    || Number(unorderedBlocks[b].linked) - Number(unorderedBlocks[a].linked)
    || unorderedBlocks[a].alignmentId.localeCompare(unorderedBlocks[b].alignmentId);
  const ready = indegree.map((degree, blockIndex) => ({ degree, blockIndex }))
    .filter(({ degree }) => degree === 0)
    .map(({ blockIndex }) => blockIndex)
    .sort(compareBlocks);
  const orderedIndexes: number[] = [];
  while (ready.length) {
    const blockIndex = ready.shift()!;
    orderedIndexes.push(blockIndex);
    for (const nextIndex of outgoing[blockIndex]) {
      indegree[nextIndex] -= 1;
      if (indegree[nextIndex] === 0) {
        ready.push(nextIndex);
        ready.sort(compareBlocks);
      }
    }
  }

  // A cross-block Segment reorder can temporarily make one Alignment span
  // another. Keep every canonical relation intact and fall back to the stable
  // block score for the cyclic remainder instead of splitting the Alignment.
  if (orderedIndexes.length !== unorderedBlocks.length) {
    const emitted = new Set(orderedIndexes);
    orderedIndexes.push(...unorderedBlocks
      .map((_, index) => index)
      .filter((index) => !emitted.has(index))
      .sort(compareBlocks));
  }

  return orderedIndexes.map((blockIndex, index) => {
    const block = unorderedBlocks[blockIndex];
    return {
      ...block,
      sourceSegments: [...block.sourceSegments].sort((a, b) => a.order - b.order),
      targetSegments: [...block.targetSegments].sort((a, b) => a.order - b.order),
      index,
    };
  });
};

export const buildAlignmentBlocks = (
  sourceSegments: SegmentDto[],
  targetSegments: SegmentDto[],
  alignments: AlignmentDto[],
): AlignmentBlockView[] => {
  const sourceById = new Map(sourceSegments.map((segment) => [segment.id, segment]));
  const targetById = new Map(targetSegments.map((segment) => [segment.id, segment]));
  const usedSources = new Set<string>();
  const usedTargets = new Set<string>();
  const blocks: AlignmentBlockView[] = [];

  for (const alignment of alignments) {
    const blockSources = alignment.sourceIds
      .map((id) => sourceById.get(id))
      .filter((segment): segment is SegmentDto => segment !== undefined);
    const blockTargets = alignment.targetIds
      .map((id) => targetById.get(id))
      .filter((segment): segment is SegmentDto => segment !== undefined);
    blockSources.forEach((segment) => usedSources.add(segment.id));
    blockTargets.forEach((segment) => usedTargets.add(segment.id));
    blocks.push({
      sourceSegments: blockSources,
      targetSegments: blockTargets,
      alignmentId: alignment.id,
      index: 0,
      linked: true,
      sourceFragmented: sideIsFragmented(alignment.sourceIds, sourceSegments),
      targetFragmented: sideIsFragmented(alignment.targetIds, targetSegments),
    });
  }

  // Unlink removes the canonical relation, but same-position text should stay
  // visually adjacent so the user can immediately create a new Link.
  const unlinkedTargets = targetSegments
    .filter((segment) => !usedTargets.has(segment.id))
    .sort((a, b) => a.order - b.order);
  const targetByOrder = new Map(unlinkedTargets.map((segment) => [segment.order, segment]));
  const pairedTargets = new Set<string>();
  for (const source of sourceSegments.filter((segment) => !usedSources.has(segment.id)).sort((a, b) => a.order - b.order)) {
    const target = targetByOrder.get(source.order);
    if (target) pairedTargets.add(target.id);
    blocks.push({
      sourceSegments: [source],
      targetSegments: target ? [target] : [],
      alignmentId: `unlinked-${source.id}-${target?.id ?? "empty"}`,
      index: 0,
      linked: false,
      sourceFragmented: false,
      targetFragmented: false,
    });
  }
  for (const target of unlinkedTargets.filter((segment) => !pairedTargets.has(segment.id))) {
    blocks.push({
      sourceSegments: [],
      targetSegments: [target],
      alignmentId: `unlinked-empty-${target.id}`,
      index: 0,
      linked: false,
      sourceFragmented: false,
      targetFragmented: false,
    });
  }

  return orderAlignmentBlocks(blocks, sourceSegments, targetSegments);
};
