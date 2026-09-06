//! Document-order mutations that preserve segment and alignment identity.

use crate::revision::advance_revision;
use crate::{KernelError, KernelService};
use jueming_core::{OrderedSegmentRef, PositionKey, SegmentId};
use jueming_protocol::ProjectSnapshot;
use std::collections::HashSet;
use std::path::Path;

impl KernelService {
    pub fn move_segment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        segment_id: SegmentId,
        before_segment_id: Option<SegmentId>,
        after_segment_id: Option<SegmentId>,
    ) -> Result<ProjectSnapshot, KernelError> {
        let mut next = snapshot.clone();
        let document_id = next
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .map(|segment| segment.document_id)
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        let revision_id = advance_revision(
            &mut next,
            "move_segment",
            1,
            format!("Moved segment {segment_id}"),
        );
        let order = next
            .segment_orders
            .iter_mut()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let moved = order
            .entries
            .iter()
            .position(|entry| entry.segment_id == segment_id)
            .map(|index| order.entries.remove(index))
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        let insertion_index = if let Some(before) = before_segment_id {
            order
                .entries
                .iter()
                .position(|entry| entry.segment_id == before)
                .map(|index| index + 1)
                .ok_or(KernelError::InvalidMoveAnchor)?
        } else if let Some(after) = after_segment_id {
            order
                .entries
                .iter()
                .position(|entry| entry.segment_id == after)
                .ok_or(KernelError::InvalidMoveAnchor)?
        } else {
            0
        };
        order.entries.insert(insertion_index, moved);
        for (index, entry) in order.entries.iter_mut().enumerate() {
            entry.position_key = PositionKey(format!("{index:020}"));
        }
        order.updated_revision_id = revision_id;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Replace one document's display order in a single durable operation.
    /// The supplied IDs must be an exact permutation of the current order so
    /// reset/reorder cannot accidentally drop or introduce segments.
    pub fn reorder_segments(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        ordered_segment_ids: Vec<SegmentId>,
    ) -> Result<ProjectSnapshot, KernelError> {
        let first_segment_id = ordered_segment_ids
            .first()
            .copied()
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let document_id = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == first_segment_id)
            .map(|segment| segment.document_id)
            .ok_or(KernelError::SegmentNotFound(first_segment_id))?;
        let current_order = snapshot
            .segment_orders
            .iter()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let current_ids: HashSet<_> = current_order
            .entries
            .iter()
            .map(|entry| entry.segment_id)
            .collect();
        let requested_ids: HashSet<_> = ordered_segment_ids.iter().copied().collect();
        if requested_ids.len() != ordered_segment_ids.len() || requested_ids != current_ids {
            return Err(KernelError::InvalidMoveAnchor);
        }

        let mut next = snapshot.clone();
        let revision_id = advance_revision(
            &mut next,
            "reorder_segments",
            ordered_segment_ids.len() as u64,
            format!("Reordered {} segments", ordered_segment_ids.len()),
        );
        let order = next
            .segment_orders
            .iter_mut()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        order.entries = ordered_segment_ids
            .into_iter()
            .enumerate()
            .map(|(index, segment_id)| OrderedSegmentRef {
                segment_id,
                position_key: PositionKey(format!("{index:020}")),
            })
            .collect();
        order.updated_revision_id = revision_id;
        self.save_project(project_path, &next)?;
        Ok(next)
    }
}
