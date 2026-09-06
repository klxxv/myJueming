//! Read-only stable-ID membership and display-order projections.

use crate::KernelError;
use jueming_core::{SegmentId, SegmentOrder};
use jueming_protocol::ProjectSnapshot;
use std::collections::HashMap;

pub(crate) fn alignment_by_segment(
    snapshot: &ProjectSnapshot,
) -> HashMap<SegmentId, jueming_core::AlignmentId> {
    snapshot
        .alignments
        .iter()
        .flat_map(|alignment| {
            alignment
                .source_segment_ids
                .iter()
                .chain(alignment.target_segment_ids.iter())
                .map(move |id| (*id, alignment.alignment_id))
        })
        .collect()
}

pub(crate) fn ordered_segment_ids(
    snapshot: &ProjectSnapshot,
    document_id: jueming_core::DocumentId,
) -> Result<Vec<SegmentId>, KernelError> {
    snapshot
        .segment_orders
        .iter()
        .find(|order| order.document_id == document_id)
        .map(|order| order.entries.iter().map(|entry| entry.segment_id).collect())
        .ok_or(KernelError::InvalidMoveAnchor)
}

pub(crate) fn ordered_alignment_refs(
    snapshot: &ProjectSnapshot,
    mut source_ids: Vec<SegmentId>,
    mut target_ids: Vec<SegmentId>,
) -> (Vec<SegmentId>, Vec<SegmentId>) {
    let source_order = snapshot
        .segment_orders
        .iter()
        .find(|order| order.document_id == snapshot.documents[0].document_id);
    let target_order = snapshot
        .segment_orders
        .iter()
        .find(|order| order.document_id == snapshot.documents[1].document_id);
    let rank = |ids: &mut Vec<SegmentId>, order: Option<&SegmentOrder>| {
        if let Some(order) = order {
            let ranks: HashMap<_, _> = order
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| (entry.segment_id, index))
                .collect();
            ids.sort_by_key(|id| ranks.get(id).copied().unwrap_or(usize::MAX));
        }
    };
    rank(&mut source_ids, source_order);
    rank(&mut target_ids, target_order);
    (source_ids, target_ids)
}
