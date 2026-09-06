//! Snapshot, selection and anchor validation without storage side effects.

use crate::KernelError;
use jueming_core::{
    RevisionId, SegmentId, validate_alignment, validate_project, validate_segment_order,
};
use jueming_protocol::{CONTRACT_VERSION, ProjectSnapshot, SupportedLanguageId};
use std::collections::{HashMap, HashSet};

pub fn validate_snapshot(snapshot: &ProjectSnapshot) -> Result<(), KernelError> {
    if snapshot.contract_version != CONTRACT_VERSION || snapshot.documents.len() != 2 {
        return Err(KernelError::InvalidSnapshot(
            "contract version or bilingual documents are invalid".into(),
        ));
    }
    validate_project(&snapshot.project, &snapshot.documents, &snapshot.revisions)?;
    canonical_language(&snapshot.project.source_language)?;
    canonical_language(&snapshot.project.target_language)?;
    for document in &snapshot.documents {
        canonical_language(&document.language_id)?;
    }
    let source_document = &snapshot.documents[0];
    let target_document = &snapshot.documents[1];
    let segment_map: HashMap<_, _> = snapshot
        .segments
        .iter()
        .cloned()
        .map(|segment| (segment.segment_id, segment))
        .collect();
    if segment_map.len() != snapshot.segments.len() {
        return Err(KernelError::InvalidSnapshot("duplicate segment IDs".into()));
    }
    for document in &snapshot.documents {
        let segments: Vec<_> = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == document.document_id)
            .cloned()
            .collect();
        let order = snapshot
            .segment_orders
            .iter()
            .find(|order| order.segment_order_id == document.segment_order_id)
            .ok_or_else(|| KernelError::InvalidSnapshot("missing segment order".into()))?;
        validate_segment_order(order, &segments)?;
    }
    let mut occupied = HashSet::new();
    for alignment in &snapshot.alignments {
        validate_alignment(
            alignment,
            snapshot.project.project_id,
            source_document,
            target_document,
            &segment_map,
        )?;
        for segment_id in alignment
            .source_segment_ids
            .iter()
            .chain(alignment.target_segment_ids.iter())
        {
            if !occupied.insert(*segment_id) {
                return Err(KernelError::DuplicateActiveAlignment(*segment_id));
            }
        }
    }
    if snapshot
        .bookmarks
        .iter()
        .any(|bookmark| bookmark.project_id != snapshot.project.project_id)
    {
        return Err(KernelError::InvalidSnapshot(
            "bookmark belongs to another project".into(),
        ));
    }
    for bookmark in &snapshot.bookmarks {
        ensure_anchor(snapshot, bookmark.segment_id, bookmark.alignment_id)?;
    }
    for annotation in &snapshot.annotations {
        if annotation.project_id != snapshot.project.project_id {
            return Err(KernelError::InvalidSnapshot(
                "annotation belongs to another project".into(),
            ));
        }
        ensure_annotation_anchors(
            snapshot,
            &annotation.linked_segment_ids,
            annotation.alignment_id,
        )?;
    }
    Ok(())
}

pub(crate) fn canonical_language(value: &str) -> Result<String, KernelError> {
    SupportedLanguageId::parse_compatible(value)
        .map(|language| language.as_str().to_owned())
        .ok_or_else(|| KernelError::UnsupportedLanguage(value.to_owned()))
}

pub(crate) fn ensure_revision(
    snapshot: &ProjectSnapshot,
    project_id: jueming_core::ProjectId,
    revision_id: RevisionId,
) -> Result<(), KernelError> {
    if snapshot.project.project_id != project_id {
        return Err(KernelError::ProjectMismatch);
    }
    if snapshot.project.current_revision_id != revision_id {
        return Err(KernelError::StaleRevision {
            expected: snapshot.project.current_revision_id,
            provided: revision_id,
        });
    }
    Ok(())
}

pub(crate) fn ensure_anchor(
    snapshot: &ProjectSnapshot,
    segment_id: SegmentId,
    alignment_id: Option<jueming_core::AlignmentId>,
) -> Result<(), KernelError> {
    if !snapshot
        .segments
        .iter()
        .any(|segment| segment.segment_id == segment_id)
    {
        return Err(KernelError::SegmentNotFound(segment_id));
    }
    if let Some(alignment_id) = alignment_id {
        let alignment = snapshot
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
        if !alignment.source_segment_ids.contains(&segment_id)
            && !alignment.target_segment_ids.contains(&segment_id)
        {
            return Err(KernelError::AnchorMismatch);
        }
    }
    Ok(())
}

pub(crate) fn ensure_annotation_anchors(
    snapshot: &ProjectSnapshot,
    segment_ids: &[SegmentId],
    alignment_id: Option<jueming_core::AlignmentId>,
) -> Result<(), KernelError> {
    if segment_ids.is_empty() || has_duplicate_ids(segment_ids) {
        return Err(KernelError::InvalidAnnotationLinks);
    }
    for id in segment_ids {
        if !snapshot
            .segments
            .iter()
            .any(|segment| segment.segment_id == *id)
        {
            return Err(KernelError::SegmentNotFound(*id));
        }
    }
    if let Some(alignment_id) = alignment_id {
        let alignment = snapshot
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
        if segment_ids.iter().any(|id| {
            !alignment.source_segment_ids.contains(id) && !alignment.target_segment_ids.contains(id)
        }) {
            return Err(KernelError::AnchorMismatch);
        }
    }
    Ok(())
}

pub(crate) fn has_duplicate_ids<T: Eq + std::hash::Hash>(values: &[T]) -> bool {
    let mut seen = HashSet::new();
    values.iter().any(|value| !seen.insert(value))
}

pub(crate) fn ensure_selection(
    snapshot: &ProjectSnapshot,
    source_ids: &[SegmentId],
    target_ids: &[SegmentId],
) -> Result<(), KernelError> {
    if source_ids.is_empty()
        || target_ids.is_empty()
        || has_duplicate_ids(source_ids)
        || has_duplicate_ids(target_ids)
    {
        return Err(KernelError::InvalidAlignmentSelection);
    }
    let source_document = snapshot
        .documents
        .first()
        .ok_or_else(|| KernelError::InvalidSnapshot("missing source document".into()))?;
    let target_document = snapshot
        .documents
        .get(1)
        .ok_or_else(|| KernelError::InvalidSnapshot("missing target document".into()))?;
    for id in source_ids {
        let segment = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == *id)
            .ok_or(KernelError::SegmentNotFound(*id))?;
        if segment.document_id != source_document.document_id {
            return Err(KernelError::WrongAlignmentSide(*id));
        }
    }
    for id in target_ids {
        let segment = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == *id)
            .ok_or(KernelError::SegmentNotFound(*id))?;
        if segment.document_id != target_document.document_id {
            return Err(KernelError::WrongAlignmentSide(*id));
        }
    }
    Ok(())
}

pub(crate) fn validate_split_groups(
    original: &jueming_core::Alignment,
    source_groups: &[Vec<SegmentId>],
    target_groups: &[Vec<SegmentId>],
) -> Result<(), KernelError> {
    if source_groups.len() < 2
        || source_groups.len() != target_groups.len()
        || source_groups.iter().any(Vec::is_empty)
        || target_groups.iter().any(Vec::is_empty)
    {
        return Err(KernelError::SplitGroupsMismatch);
    }
    let flatten = |groups: &[Vec<SegmentId>]| groups.iter().flatten().copied().collect::<Vec<_>>();
    let source = flatten(source_groups);
    let target = flatten(target_groups);
    if has_duplicate_ids(&source)
        || has_duplicate_ids(&target)
        || {
            let mut left = source.clone();
            let mut right = original.source_segment_ids.clone();
            left.sort();
            right.sort();
            left != right
        }
        || {
            let mut left = target.clone();
            let mut right = original.target_segment_ids.clone();
            left.sort();
            right.sort();
            left != right
        }
    {
        return Err(KernelError::SplitGroupsMismatch);
    }
    Ok(())
}
