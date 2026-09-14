import assert from 'node:assert/strict';
import { buildAlignmentBlocks } from '../../apps/desktop/src/domain/workspace-projection';
import { buildComparisonBands, layoutComparisonBand, type ComparisonDocument } from '../../apps/desktop/src/domain/comparison-projection';
import { snapshotToWorkspace, type CoreAlignment, type WorkspaceProject } from '../../apps/desktop/src/domain/kernel-client';
const documents = Array.from({ length: 4 }, (_, col): ComparisonDocument => ({
  document: { document_id: `doc-${col}`, title: `Translation ${col}`, language_id: 'en', project_id: 'project', source_asset_id: `asset-${col}`, segment_order_id: `order-${col}`, created_revision_id: '1' },
  segments: Array.from({ length: 12 }, (_, order) => ({ id: `${col}:${order}`, order, side: col ? 'target' : 'source', text: `${col}:${order}` })),
}));
const relation = (id: string, target: number, source: number[], refs: number[]): CoreAlignment => ({ alignment_id: id, source_segment_ids: source.map(index => `0:${index}`), target_segment_ids: refs.map(index => `${target}:${index}`), project_id: 'project', producer: 'manual', cardinality: 'n:m', created_revision_id: '1', updated_revision_id: '1' });
const pair = [relation('one', 1, [0], [0]), relation('one-many', 1, [1], [1, 2]), relation('many-one', 1, [2, 3], [3]), relation('many-many', 1, [4, 5], [4, 5]), relation('crossed', 1, [8, 10], [7, 9])];
const dual = buildAlignmentBlocks(documents[0].segments, documents[1].segments, pair.map(item => ({ id: item.alignment_id, sourceIds: item.source_segment_ids, targetIds: item.target_segment_ids, status: item.producer })));
const projected = buildComparisonBands(documents.slice(0, 2), pair);
assert.deepEqual(projected.map(band => [...band.cells.values()].map(cell => cell.map(segment => segment.id))), dual.map(block => [block.sourceSegments.map(segment => segment.id), block.targetSegments.map(segment => segment.id)]), 'all cardinalities and unmatched/crossed rows exactly preserve the two-column projection');
const all = [...pair, relation('second-overlap', 2, [5, 6], [2, 3, 4]), relation('third', 3, [6], [9])];
const multi = buildComparisonBands(documents, all);
assert.equal(new Set(multi.flatMap(band => [...band.cells.values()].flat().map(segment => segment.id))).size, 48);
assert.equal(multi.flatMap(band => [...band.cells.values()].flat()).length, 48, 'a shared source segment is rendered once');
assert.deepEqual(multi.flatMap(band => band.relations.map(item => item.alignment_id)).sort(), all.map(item => item.alignment_id).sort());
const overlapping = multi.find(band => band.relations.some(item => item.alignment_id === 'second-overlap'))!;
assert.ok(overlapping.relations.some(item => item.alignment_id === 'many-many'));
assert.ok(overlapping.relations.some(item => item.alignment_id === 'third'));
const snapshot = { documents: documents.map(doc => doc.document), segments: documents.flatMap(doc => doc.segments.map(segment => ({ segment_id: segment.id, document_id: doc.document.document_id, content_length: 3, content_hash: segment.id }))), segment_orders: documents.map(doc => ({ document_id: doc.document.document_id, entries: doc.segments.map(segment => ({ segment_id: segment.id, position_key: String(segment.order) })) })), alignments: all } as WorkspaceProject;
const selected = snapshotToWorkspace(snapshot, new Map(), 'doc-2');
assert.deepEqual(selected.alignments.map(item => item.id), ['second-overlap']);
assert.ok(selected.targetSegments.every(segment => segment.id.startsWith('2:')), 'same-language translations are selected by DocumentId');
console.log('comparison: exact two-column projection, overlapping n:m, stable memberships, and same-language document selection passed');

const heights = new Map(documents.flatMap(doc => doc.segments.map(segment => [segment.id, segment.id === '3:9' ? 900 : 114] as const)));
const layout = layoutComparisonBand(overlapping, heights);
assert.equal(layout.crossed, false);
for (const item of overlapping.relations) {
  const first = (ids: string[]) => [...ids].sort((a, b) => Number(a.split(':')[1]) - Number(b.split(':')[1]))[0];
  assert.equal(layout.segments.get(first(item.source_segment_ids))!.top, layout.segments.get(first(item.target_segment_ids))!.top, 'each overlapping relation retains its own start anchor');
}
assert.ok(layout.segments.get('0:5')!.top > layout.segments.get('0:4')!.top, 'overlapping groups are not flattened into a common start');
for (const cell of overlapping.cells.values()) for (let index = 1; index < cell.length; index++) {
  const previous = layout.segments.get(cell[index - 1].id)!;
  assert.ok(layout.segments.get(cell[index].id)!.top >= previous.top + previous.height, 'measured long text never overlaps the next segment');
}
console.log('comparison: independent overlapping relation anchors and measured-height constraints passed');
