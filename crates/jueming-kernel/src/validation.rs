//! Snapshot, selection and anchor validation without storage side effects.

use crate::KernelError;
use jueming_core::{
    RevisionId, SegmentId, validate_alignment, validate_project, validate_segment_order,
};
use jueming_protocol::{CONTRACT_VERSION, ProjectSnapshot, SupportedLanguageId};
use std::collections::{HashMap, HashSet};

pub fn validate_snapshot(snapshot: &ProjectSnapshot) -> Result<(), KernelError> {
    if ![CONTRACT_VERSION, "1.1", "2.0"].contains(&snapshot.contract_version.as_str())
        || snapshot.documents.len() < 2
        || (snapshot.contract_version != "2.0" && snapshot.documents.len() != 2)
    {
        return Err(KernelError::InvalidSnapshot(
            "contract version or bilingual documents are invalid".into(),
        ));
    }
    if !snapshot.research_records.is_empty()
        && !matches!(snapshot.contract_version.as_str(), "1.1" | "2.0")
    {
        return Err(KernelError::InvalidSnapshot(
            "research sidecar requires project contract 1.1".into(),
        ));
    }
    validate_project(&snapshot.project, &snapshot.documents, &snapshot.revisions)?;
    let mut research_ids = HashSet::new();
    let mut research_occurrences = HashSet::new();
    for record in &snapshot.research_records {
        if record.record_id.is_empty()
            || record.run_id.is_empty()
            || record.occurrence_id.is_empty()
            || !research_ids.insert(&record.record_id)
            || !research_occurrences.insert((&record.run_id, &record.occurrence_id))
            || record.source_ranges.is_empty()
            || record.input_revision_id.value() > snapshot.project.current_revision_id.value()
            || record.judgement.group_name.len() > 512
            || record.judgement.strategy.len() > 512
            || (record.judgement.kind == jueming_protocol::JudgementKind::Omission
                && !record.judgement.target_ranges.is_empty())
            || (record.judgement.kind == jueming_protocol::JudgementKind::Translation
                && record.judgement.target_ranges.is_empty())
        {
            return Err(KernelError::InvalidSnapshot(
                "invalid research sidecar identity or judgement".into(),
            ));
        }
        // Anchors may be stale after later edits. Their original evidence must remain readable.
        for range in record
            .source_ranges
            .iter()
            .chain(&record.judgement.target_ranges)
        {
            if range.start_utf8 >= range.end_utf8
                || !record
                    .content_hashes
                    .get(&range.segment_id)
                    .is_some_and(|hash| {
                        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                    })
            {
                return Err(KernelError::InvalidSnapshot(
                    "invalid research evidence coordinates or digest".into(),
                ));
            }
        }
    }
    canonical_language(&snapshot.project.source_language)?;
    canonical_language(&snapshot.project.target_language)?;
    for document in &snapshot.documents {
        canonical_language(&document.language_id)?;
    }
    let source_document = &snapshot.documents[0];
    if snapshot.contract_version == "2.0" {
        let comparison = snapshot.project.comparison.as_ref().ok_or_else(|| {
            KernelError::InvalidSnapshot("missing comparison document roles".into())
        })?;
        if snapshot.project.format_version != "2.0"
            || comparison.source_document_id != source_document.document_id
            || comparison.target_document_ids
                != snapshot.documents[1..]
                    .iter()
                    .map(|doc| doc.document_id)
                    .collect::<Vec<_>>()
        {
            return Err(KernelError::InvalidSnapshot(
                "comparison roles do not match document catalogue".into(),
            ));
        }
    } else if snapshot.project.comparison.is_some() {
        return Err(KernelError::InvalidSnapshot(
            "comparison requires contract 2.0".into(),
        ));
    }
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
    if snapshot.segments.iter().any(|segment| {
        !snapshot
            .documents
            .iter()
            .any(|doc| doc.document_id == segment.document_id)
    }) {
        return Err(KernelError::InvalidSnapshot(
            "segment belongs to unknown document".into(),
        ));
    }
    let mut alignment_ids = HashSet::new();
    let mut occupied = HashSet::new();
    for alignment in &snapshot.alignments {
        if !alignment_ids.insert(alignment.alignment_id) {
            return Err(KernelError::InvalidSnapshot(
                "duplicate alignment IDs".into(),
            ));
        }
        let target_id = alignment
            .target_segment_ids
            .first()
            .and_then(|id| segment_map.get(id))
            .ok_or(KernelError::InvalidAlignmentSelection)?
            .document_id;
        let target_document = snapshot
            .documents
            .iter()
            .skip(1)
            .find(|doc| doc.document_id == target_id)
            .ok_or(KernelError::InvalidAlignmentSelection)?;
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
            if !occupied.insert((target_id, *segment_id)) {
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
    let target_id = crate::projection::target_document_id(snapshot, target_ids)?;
    let target_document = snapshot
        .documents
        .iter()
        .skip(1)
        .find(|doc| doc.document_id == target_id)
        .ok_or(KernelError::InvalidAlignmentSelection)?;
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
