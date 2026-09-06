//! Visual alignment-gap insertion without creating fake segments.

use crate::projection::ordered_segment_ids;
use crate::revision::{advance_revision, next_revision_id};
use crate::sidecar::refresh_alignment_metadata;
use crate::validation::validate_snapshot;
use crate::{KernelError, KernelService};
use jueming_core::SegmentId;
use jueming_protocol::{AlignmentGapEdge, ProjectSnapshot};
use std::collections::{HashMap, HashSet};
use std::path::Path;

impl KernelService {
    /// Insert a visual gap on the selected segment's side and rebuild the
    /// remaining suffix as ordered 1:1 manual alignments. The gap itself is
    /// represented canonically by the opposite segment becoming unlinked; no
    /// fake or empty Segment is introduced.
    pub fn insert_alignment_gap(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        segment_id: SegmentId,
        edge: AlignmentGapEdge,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let source_document_id = snapshot
            .documents
            .first()
            .ok_or_else(|| KernelError::InvalidSnapshot("missing source document".into()))?
            .document_id;
        let target_document_id = snapshot
            .documents
            .get(1)
            .ok_or_else(|| KernelError::InvalidSnapshot("missing target document".into()))?
            .document_id;
        let selected_segment = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        let selected_alignment = snapshot
            .alignments
            .iter()
            .find(|alignment| {
                alignment.source_segment_ids.contains(&segment_id)
                    || alignment.target_segment_ids.contains(&segment_id)
            })
            .ok_or(KernelError::AlignmentGapRequiresLinkedSegment(segment_id))?;
        let source_order = ordered_segment_ids(snapshot, source_document_id)?;
        let target_order = ordered_segment_ids(snapshot, target_document_id)?;
        let source_ranks: HashMap<_, _> = source_order
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect();
        let target_ranks: HashMap<_, _> = target_order
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect();

        let (source_start, target_start, source_pair_start, target_pair_start) =
            if selected_segment.document_id == source_document_id {
                let selected_rank = *source_ranks
                    .get(&segment_id)
                    .ok_or(KernelError::InvalidMoveAnchor)?;
                let opposite_ranks = selected_alignment
                    .target_segment_ids
                    .iter()
                    .filter_map(|id| target_ranks.get(id).copied())
                    .collect::<Vec<_>>();
                let opposite_start = match edge {
                    AlignmentGapEdge::Before => opposite_ranks.iter().min().copied(),
                    AlignmentGapEdge::After => {
                        opposite_ranks.iter().max().copied().map(|rank| rank + 1)
                    }
                }
                .ok_or(KernelError::AlignmentGapOutOfRange)?;
                let selected_start =
                    selected_rank + usize::from(matches!(edge, AlignmentGapEdge::After));
                (
                    selected_start,
                    opposite_start,
                    selected_start,
                    opposite_start + 1,
                )
            } else if selected_segment.document_id == target_document_id {
                let selected_rank = *target_ranks
                    .get(&segment_id)
                    .ok_or(KernelError::InvalidMoveAnchor)?;
                let opposite_ranks = selected_alignment
                    .source_segment_ids
                    .iter()
                    .filter_map(|id| source_ranks.get(id).copied())
                    .collect::<Vec<_>>();
                let opposite_start = match edge {
                    AlignmentGapEdge::Before => opposite_ranks.iter().min().copied(),
                    AlignmentGapEdge::After => {
                        opposite_ranks.iter().max().copied().map(|rank| rank + 1)
                    }
                }
                .ok_or(KernelError::AlignmentGapOutOfRange)?;
                let selected_start =
                    selected_rank + usize::from(matches!(edge, AlignmentGapEdge::After));
                (
                    opposite_start,
                    selected_start,
                    opposite_start + 1,
                    selected_start,
                )
            } else {
                return Err(KernelError::WrongAlignmentSide(segment_id));
            };

        if source_start >= source_order.len()
            || target_start >= target_order.len()
            || source_pair_start >= source_order.len()
            || target_pair_start >= target_order.len()
        {
            return Err(KernelError::AlignmentGapOutOfRange);
        }
        let pair_count =
            (source_order.len() - source_pair_start).min(target_order.len() - target_pair_start);
        if pair_count == 0 {
            return Err(KernelError::AlignmentGapOutOfRange);
        }

        let affected_segment_ids: HashSet<_> = source_order[source_start..]
            .iter()
            .chain(target_order[target_start..].iter())
            .copied()
            .collect();
        let removed_alignment_count = snapshot
            .alignments
            .iter()
            .filter(|alignment| {
                alignment
                    .source_segment_ids
                    .iter()
                    .chain(alignment.target_segment_ids.iter())
                    .any(|id| affected_segment_ids.contains(id))
            })
            .count();
        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "insert_alignment_gap",
            (removed_alignment_count + pair_count) as u64,
            format!(
                "Inserted alignment gap {:?} segment {segment_id} and realigned {pair_count} pairs",
                edge
            ),
        );
        next.alignments.retain(|alignment| {
            !alignment
                .source_segment_ids
                .iter()
                .chain(alignment.target_segment_ids.iter())
                .any(|id| affected_segment_ids.contains(id))
        });
        for offset in 0..pair_count {
            next.alignments.push(jueming_core::Alignment::new(
                snapshot.project.project_id,
                vec![source_order[source_pair_start + offset]],
                vec![target_order[target_pair_start + offset]],
                revision_id,
            )?);
        }
        refresh_alignment_metadata(&mut next);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }
}
