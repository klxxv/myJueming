//! Read-only stable-ID membership and display-order projections.

use crate::KernelError;
use jueming_core::{SegmentId, SegmentOrder};
use jueming_protocol::ProjectSnapshot;
use std::collections::HashMap;

pub(crate) fn alignment_by_segment(
    snapshot: &ProjectSnapshot,
) -> HashMap<SegmentId, jueming_core::AlignmentId> {
    let mut membership = HashMap::new();
    let primary = alignment_ids_for_document(snapshot, snapshot.documents[1].document_id);
    for alignment in snapshot
        .alignments
        .iter()
        .filter(|alignment| primary.contains(&alignment.alignment_id))
        .chain(
            snapshot
                .alignments
                .iter()
                .filter(|alignment| !primary.contains(&alignment.alignment_id)),
        )
    {
        for id in alignment
            .source_segment_ids
            .iter()
            .chain(&alignment.target_segment_ids)
        {
            membership.entry(*id).or_insert(alignment.alignment_id);
        }
    }
    membership
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
        .find(|order| Some(order.document_id) == target_document_id(snapshot, &target_ids).ok());
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

/// Infer a relation's document pair from stable canonical references, never language or UI columns.
pub(crate) fn target_document_id(
    snapshot: &ProjectSnapshot,
    ids: &[SegmentId],
) -> Result<jueming_core::DocumentId, KernelError> {
    let first = ids.first().ok_or(KernelError::InvalidAlignmentSelection)?;
    let document_id = snapshot
        .segments
        .iter()
        .find(|segment| segment.segment_id == *first)
        .ok_or(KernelError::SegmentNotFound(*first))?
        .document_id;
    for id in ids {
        let segment = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == *id)
            .ok_or(KernelError::SegmentNotFound(*id))?;
        if segment.document_id != document_id {
            return Err(KernelError::WrongAlignmentSide(*id));
        }
    }
    Ok(document_id)
}

/// Build pair membership once for command/projection loops (linear in project size).
pub(crate) fn alignment_ids_for_document(
    snapshot: &ProjectSnapshot,
    document_id: jueming_core::DocumentId,
) -> std::collections::HashSet<jueming_core::AlignmentId> {
    let ids: std::collections::HashSet<_> = snapshot
        .segments
        .iter()
        .filter(|segment| segment.document_id == document_id)
        .map(|segment| segment.segment_id)
        .collect();
    snapshot
        .alignments
        .iter()
        .filter(|alignment| {
            alignment
                .target_segment_ids
                .first()
                .is_some_and(|id| ids.contains(id))
        })
        .map(|alignment| alignment.alignment_id)
        .collect()
}
