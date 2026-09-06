//! Content edits and lossless segment merge/split.

use crate::projection::alignment_by_segment;
use crate::revision::{advance_revision, next_revision_id};
use crate::sidecar::{migrate_annotation_links_after_split, migrate_segment_metadata_after_merge};
use crate::validation::{has_duplicate_ids, validate_snapshot};
use crate::{KernelError, KernelService};
use jueming_core::{
    ContentRef, OrderedSegmentRef, PositionKey, RevisionId, Segment, SegmentId, SegmentOrder,
};
use jueming_protocol::ProjectSnapshot;
use std::collections::HashSet;
use std::path::Path;

impl KernelService {
    pub fn update_segment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        segment_id: SegmentId,
        content: &str,
    ) -> Result<ProjectSnapshot, KernelError> {
        let mut next = snapshot.clone();
        let revision_id = advance_revision(
            &mut next,
            "update_segment",
            1,
            format!("Edited segment {segment_id}"),
        );
        let segment = next
            .segments
            .iter_mut()
            .find(|segment| segment.segment_id == segment_id)
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        segment.update_content(content, revision_id);
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Merge consecutive segments from one document into the first segment in
    /// display order. The operation is deliberately content-only: it is only
    /// valid when every selected segment is unlinked, or every one belongs to
    /// the same active alignment.
    pub fn merge_segments(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        segment_ids: Vec<SegmentId>,
        merged_content: &str,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        if segment_ids.len() < 2 || has_duplicate_ids(&segment_ids) {
            return Err(KernelError::MergeSegmentsRequiresMultiple);
        }
        let first_segment = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_ids[0])
            .ok_or(KernelError::SegmentNotFound(segment_ids[0]))?;
        let document_id = first_segment.document_id;
        for segment_id in &segment_ids {
            let segment = snapshot
                .segments
                .iter()
                .find(|segment| segment.segment_id == *segment_id)
                .ok_or(KernelError::SegmentNotFound(*segment_id))?;
            if segment.document_id != document_id {
                return Err(KernelError::MergeSegmentsDifferentDocuments);
            }
        }
        let order = snapshot
            .segment_orders
            .iter()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let selected: HashSet<_> = segment_ids.iter().copied().collect();
        let selected_positions = order
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| selected.contains(&entry.segment_id).then_some(index))
            .collect::<Vec<_>>();
        if selected_positions.len() != segment_ids.len()
            || selected_positions.last().copied()
                != selected_positions
                    .first()
                    .map(|start| start + segment_ids.len() - 1)
        {
            return Err(KernelError::MergeSegmentsNotConsecutive);
        }
        let ordered_selected = selected_positions
            .iter()
            .map(|index| order.entries[*index].segment_id)
            .collect::<Vec<_>>();
        let survivor = ordered_selected[0];
        let memberships = alignment_by_segment(snapshot);
        let selected_alignment_ids = ordered_selected
            .iter()
            .filter_map(|id| memberships.get(id).copied())
            .collect::<HashSet<_>>();
        if !(selected_alignment_ids.is_empty()
            || (selected_alignment_ids.len() == 1
                && ordered_selected
                    .iter()
                    .all(|id| memberships.contains_key(id))))
        {
            return Err(KernelError::MergeSegmentsAlignmentConflict);
        }

        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "merge_segments",
            ordered_selected.len() as u64,
            format!("Merged {} segments into {survivor}", ordered_selected.len()),
        );
        next.segments
            .iter_mut()
            .find(|segment| segment.segment_id == survivor)
            .ok_or(KernelError::SegmentNotFound(survivor))?
            .update_content(merged_content, revision_id);
        next.segments.retain(|segment| {
            !selected.contains(&segment.segment_id) || segment.segment_id == survivor
        });
        let next_order = next
            .segment_orders
            .iter_mut()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        next_order
            .entries
            .retain(|entry| !selected.contains(&entry.segment_id) || entry.segment_id == survivor);
        rekey_order(next_order, revision_id);
        if let Some(alignment_id) = selected_alignment_ids.iter().next().copied() {
            let alignment = next
                .alignments
                .iter_mut()
                .find(|alignment| alignment.alignment_id == alignment_id)
                .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
            replace_alignment_segment_refs(alignment, &selected, survivor, revision_id);
        }
        migrate_segment_metadata_after_merge(&mut next, &selected, survivor);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Split one segment into lossless, non-empty content parts. The original
    /// ID remains anchored to the first part; additional parts receive new
    /// stable IDs and inherit the original alignment when one exists.
    pub fn split_segment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        segment_id: SegmentId,
        parts: Vec<String>,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let original = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .cloned()
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        if parts.len() < 2 || parts.iter().any(String::is_empty) {
            return Err(KernelError::InvalidSplitParts);
        }
        if parts.concat() != original.content {
            return Err(KernelError::SplitContentMismatch);
        }
        let original_alignment_id = alignment_by_segment(snapshot).get(&segment_id).copied();
        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "split_segment",
            parts.len() as u64,
            format!("Split segment {segment_id} into {} parts", parts.len()),
        );
        next.segments
            .iter_mut()
            .find(|segment| segment.segment_id == segment_id)
            .ok_or(KernelError::SegmentNotFound(segment_id))?
            .update_content(parts[0].clone(), revision_id);
        let inserted_segments = parts
            .into_iter()
            .enumerate()
            .skip(1)
            .map(|(index, content)| {
                let mut segment = Segment::new(original.document_id, content, revision_id);
                segment.content_ref = ContentRef {
                    key: format!("{}#split-{index}", original.content_ref.key),
                };
                segment
            })
            .collect::<Vec<_>>();
        let inserted_ids = inserted_segments
            .iter()
            .map(|segment| segment.segment_id)
            .collect::<Vec<_>>();
        next.segments.extend(inserted_segments);
        let order = next
            .segment_orders
            .iter_mut()
            .find(|order| order.document_id == original.document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let insertion_index = order
            .entries
            .iter()
            .position(|entry| entry.segment_id == segment_id)
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        for (offset, inserted_id) in inserted_ids.iter().enumerate() {
            order.entries.insert(
                insertion_index + 1 + offset,
                OrderedSegmentRef {
                    segment_id: *inserted_id,
                    position_key: PositionKey(String::new()),
                },
            );
        }
        rekey_order(order, revision_id);
        if let Some(alignment_id) = original_alignment_id {
            let alignment = next
                .alignments
                .iter_mut()
                .find(|alignment| alignment.alignment_id == alignment_id)
                .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
            insert_alignment_segment_refs(alignment, segment_id, &inserted_ids, revision_id);
        }
        migrate_annotation_links_after_split(&mut next, segment_id, &inserted_ids);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }
}

fn rekey_order(order: &mut SegmentOrder, revision_id: RevisionId) {
    for (index, entry) in order.entries.iter_mut().enumerate() {
        entry.position_key = PositionKey(format!("{index:020}"));
    }
    order.updated_revision_id = revision_id;
}

fn replace_alignment_segment_refs(
    alignment: &mut jueming_core::Alignment,
    replaced: &HashSet<SegmentId>,
    survivor: SegmentId,
    revision_id: RevisionId,
) {
    let replace = |ids: &mut Vec<SegmentId>| {
        let mut seen = HashSet::new();
        ids.retain(|id| !replaced.contains(id) || *id == survivor);
        for id in ids.iter_mut() {
            if replaced.contains(id) {
                *id = survivor;
            }
        }
        ids.retain(|id| seen.insert(*id));
    };
    replace(&mut alignment.source_segment_ids);
    replace(&mut alignment.target_segment_ids);
    alignment.cardinality = jueming_core::Cardinality::of(
        alignment.source_segment_ids.len(),
        alignment.target_segment_ids.len(),
    )
    .expect("merge always retains one segment on each alignment side");
    alignment.updated_revision_id = revision_id;
}

fn insert_alignment_segment_refs(
    alignment: &mut jueming_core::Alignment,
    original: SegmentId,
    inserted: &[SegmentId],
    revision_id: RevisionId,
) {
    let insert_after = |ids: &mut Vec<SegmentId>| {
        if let Some(index) = ids.iter().position(|id| *id == original) {
            ids.splice(index + 1..index + 1, inserted.iter().copied());
        }
    };
    insert_after(&mut alignment.source_segment_ids);
    insert_after(&mut alignment.target_segment_ids);
    alignment.cardinality = jueming_core::Cardinality::of(
        alignment.source_segment_ids.len(),
        alignment.target_segment_ids.len(),
    )
    .expect("split preserves both alignment sides");
    alignment.updated_revision_id = revision_id;
}
