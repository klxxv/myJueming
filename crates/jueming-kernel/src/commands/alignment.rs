//! Manual relation commands and legacy entry-point compatibility.

use crate::projection::{alignment_ids_for_document, ordered_alignment_refs, target_document_id};
use crate::revision::{advance_revision, next_revision_id};
use crate::sidecar::refresh_alignment_metadata;
use crate::validation::{
    ensure_selection, has_duplicate_ids, validate_snapshot, validate_split_groups,
};
use crate::{KernelError, KernelService};
use jueming_core::SegmentId;
use jueming_protocol::ProjectSnapshot;
use std::collections::HashSet;
use std::path::Path;

impl KernelService {
    /// Link the selected segments into one manual alignment. Existing active
    /// alignments are replaced only when the caller explicitly opts in.
    pub fn link_segments(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        source_segment_ids: Vec<SegmentId>,
        target_segment_ids: Vec<SegmentId>,
        replace_existing: bool,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_selection(snapshot, &source_segment_ids, &target_segment_ids)?;
        let target_document_id = target_document_id(snapshot, &target_segment_ids)?;
        let pair_alignment_ids = alignment_ids_for_document(snapshot, target_document_id);
        let mut next = snapshot.clone();
        let selected: HashSet<_> = source_segment_ids
            .iter()
            .chain(target_segment_ids.iter())
            .copied()
            .collect();
        let removed = next
            .alignments
            .iter()
            .filter(|alignment| {
                pair_alignment_ids.contains(&alignment.alignment_id)
                    && alignment
                        .source_segment_ids
                        .iter()
                        .chain(alignment.target_segment_ids.iter())
                        .any(|id| selected.contains(id))
            })
            .count();
        if removed > 0 && !replace_existing {
            return Err(KernelError::AlignmentSelectionConflict);
        }
        let revision_id = next_revision_id(&next);
        advance_revision(
            &mut next,
            "link_alignment",
            (removed + 1) as u64,
            format!(
                "Linked {} source and {} target segments",
                source_segment_ids.len(),
                target_segment_ids.len()
            ),
        );
        next.alignments.retain(|alignment| {
            !pair_alignment_ids.contains(&alignment.alignment_id)
                || !alignment
                    .source_segment_ids
                    .iter()
                    .chain(alignment.target_segment_ids.iter())
                    .any(|id| selected.contains(id))
        });
        next.alignments.push(jueming_core::Alignment::new(
            snapshot.project.project_id,
            source_segment_ids,
            target_segment_ids,
            revision_id,
        )?);
        refresh_alignment_metadata(&mut next);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn unlink_alignment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_id: jueming_core::AlignmentId,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        if !snapshot
            .alignments
            .iter()
            .any(|alignment| alignment.alignment_id == alignment_id)
        {
            return Err(KernelError::AlignmentNotFound(alignment_id));
        }
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "unlink_alignment",
            1,
            format!("Unlinked alignment {alignment_id}"),
        );
        next.alignments
            .retain(|alignment| alignment.alignment_id != alignment_id);
        refresh_alignment_metadata(&mut next);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Combine complete alignment relations (and optionally unlinked segments)
    /// into one new relation.  This never changes Segment content.
    pub fn group_alignment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_ids: Vec<jueming_core::AlignmentId>,
        unlinked_segment_ids: Vec<SegmentId>,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        if alignment_ids.is_empty()
            || alignment_ids.len() + unlinked_segment_ids.len() < 2
            || has_duplicate_ids(&alignment_ids)
            || has_duplicate_ids(&unlinked_segment_ids)
        {
            return Err(KernelError::MergeRequiresMultiple);
        }
        let selected: Vec<_> = alignment_ids
            .iter()
            .filter_map(|id| {
                snapshot
                    .alignments
                    .iter()
                    .find(|alignment| alignment.alignment_id == *id)
            })
            .cloned()
            .collect();
        if selected.len() != alignment_ids.len() {
            return Err(KernelError::AlignmentNotFound(
                *alignment_ids
                    .iter()
                    .find(|id| {
                        !snapshot
                            .alignments
                            .iter()
                            .any(|alignment| alignment.alignment_id == **id)
                    })
                    .unwrap(),
            ));
        }
        let mut source_ids = selected
            .iter()
            .flat_map(|alignment| alignment.source_segment_ids.iter().copied())
            .collect::<Vec<_>>();
        let mut target_ids = selected
            .iter()
            .flat_map(|alignment| alignment.target_segment_ids.iter().copied())
            .collect::<Vec<_>>();
        let target_document_id = target_document_id(snapshot, &target_ids)?;
        let pair_alignment_ids = alignment_ids_for_document(snapshot, target_document_id);
        let aligned_segment_ids: HashSet<_> = snapshot
            .alignments
            .iter()
            .filter(|alignment| pair_alignment_ids.contains(&alignment.alignment_id))
            .flat_map(|alignment| {
                alignment
                    .source_segment_ids
                    .iter()
                    .chain(alignment.target_segment_ids.iter())
                    .copied()
            })
            .collect();
        let source_document_id = snapshot.documents[0].document_id;

        for segment_id in &unlinked_segment_ids {
            if aligned_segment_ids.contains(segment_id) {
                return Err(KernelError::MergeSegmentAlreadyAligned(*segment_id));
            }
            let segment = snapshot
                .segments
                .iter()
                .find(|segment| segment.segment_id == *segment_id)
                .ok_or(KernelError::SegmentNotFound(*segment_id))?;
            if segment.document_id == source_document_id {
                source_ids.push(*segment_id);
            } else if segment.document_id == target_document_id {
                target_ids.push(*segment_id);
            } else {
                return Err(KernelError::WrongAlignmentSide(*segment_id));
            }
        }
        if has_duplicate_ids(&source_ids) || has_duplicate_ids(&target_ids) {
            return Err(KernelError::DuplicateActiveAlignment(
                source_ids
                    .iter()
                    .chain(target_ids.iter())
                    .find(|id| {
                        source_ids.iter().filter(|other| *other == *id).count() > 1
                            || target_ids.iter().filter(|other| *other == *id).count() > 1
                    })
                    .copied()
                    .unwrap_or(source_ids[0]),
            ));
        }
        ensure_selection(snapshot, &source_ids, &target_ids)?;
        let (source_ids, target_ids) = ordered_alignment_refs(snapshot, source_ids, target_ids);
        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "group_alignment",
            (alignment_ids.len() + unlinked_segment_ids.len()) as u64,
            format!(
                "Grouped {} alignments and {} unlinked segments",
                alignment_ids.len(),
                unlinked_segment_ids.len()
            ),
        );
        next.alignments
            .retain(|alignment| !alignment_ids.contains(&alignment.alignment_id));
        next.alignments.push(jueming_core::Alignment::new(
            snapshot.project.project_id,
            source_ids,
            target_ids,
            revision_id,
        )?);
        refresh_alignment_metadata(&mut next);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Compatibility entry point for older IPC callers. New revisions still
    /// record the canonical `group_alignment` operation.
    pub fn merge_alignments(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_ids: Vec<jueming_core::AlignmentId>,
        unlinked_segment_ids: Vec<SegmentId>,
    ) -> Result<ProjectSnapshot, KernelError> {
        self.group_alignment(project_path, snapshot, alignment_ids, unlinked_segment_ids)
    }

    /// Partition one complex alignment into explicitly chosen relations. This
    /// changes relations only, never Segment text.
    pub fn ungroup_alignment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_id: jueming_core::AlignmentId,
        mut source_groups: Vec<Vec<SegmentId>>,
        mut target_groups: Vec<Vec<SegmentId>>,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let original = snapshot
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
        if source_groups.is_empty() && target_groups.is_empty() {
            if original.source_segment_ids.len() != original.target_segment_ids.len() {
                return Err(KernelError::SplitGroupsMismatch);
            }
            source_groups = original
                .source_segment_ids
                .iter()
                .map(|id| vec![*id])
                .collect();
            target_groups = original
                .target_segment_ids
                .iter()
                .map(|id| vec![*id])
                .collect();
        }
        validate_split_groups(original, &source_groups, &target_groups)?;
        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "ungroup_alignment",
            source_groups.len() as u64,
            format!(
                "Ungrouped alignment {alignment_id} into {} groups",
                source_groups.len()
            ),
        );
        next.alignments
            .retain(|alignment| alignment.alignment_id != alignment_id);
        for (source, target) in source_groups.into_iter().zip(target_groups) {
            next.alignments.push(jueming_core::Alignment::new(
                snapshot.project.project_id,
                source,
                target,
                revision_id,
            )?);
        }
        refresh_alignment_metadata(&mut next);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Compatibility entry point for older IPC callers. New revisions still
    /// record the canonical `ungroup_alignment` operation.
    pub fn split_alignment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_id: jueming_core::AlignmentId,
        source_groups: Vec<Vec<SegmentId>>,
        target_groups: Vec<Vec<SegmentId>>,
    ) -> Result<ProjectSnapshot, KernelError> {
        self.ungroup_alignment(
            project_path,
            snapshot,
            alignment_id,
            source_groups,
            target_groups,
        )
    }
}
