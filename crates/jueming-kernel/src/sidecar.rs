//! Shared bookmark/annotation anchor migrations for structural commands.

use crate::projection::alignment_by_segment;
use crate::revision::timestamp;
use jueming_core::SegmentId;
use jueming_protocol::ProjectSnapshot;
use std::collections::HashSet;

pub(crate) fn migrate_segment_metadata_after_merge(
    snapshot: &mut ProjectSnapshot,
    replaced: &HashSet<SegmentId>,
    survivor: SegmentId,
) {
    let now = timestamp();
    for bookmark in &mut snapshot.bookmarks {
        if replaced.contains(&bookmark.segment_id) {
            bookmark.segment_id = survivor;
            bookmark.updated_at = now.clone();
        }
    }
    for annotation in &mut snapshot.annotations {
        let previous = annotation.linked_segment_ids.clone();
        let mut seen = HashSet::new();
        annotation.linked_segment_ids = annotation
            .linked_segment_ids
            .iter()
            .map(|id| if replaced.contains(id) { survivor } else { *id })
            .filter(|id| seen.insert(*id))
            .collect();
        if annotation.linked_segment_ids != previous {
            annotation.updated_at = now.clone();
        }
    }
}

pub(crate) fn migrate_annotation_links_after_split(
    snapshot: &mut ProjectSnapshot,
    original: SegmentId,
    inserted: &[SegmentId],
) {
    let now = timestamp();
    for annotation in &mut snapshot.annotations {
        let previous = annotation.linked_segment_ids.clone();
        let mut links = Vec::with_capacity(annotation.linked_segment_ids.len() + inserted.len());
        for id in &annotation.linked_segment_ids {
            links.push(*id);
            if *id == original {
                links.extend(inserted.iter().copied());
            }
        }
        let mut seen = HashSet::new();
        links.retain(|id| seen.insert(*id));
        if links != previous {
            annotation.linked_segment_ids = links;
            annotation.updated_at = now.clone();
        }
    }
}

/// Relationship IDs are ephemeral navigation hints in sidecars. Whenever a
/// relation is replaced, derive a still-active hint from the canonical segment
/// links; when no single relation covers an annotation, clear the hint.
pub(crate) fn refresh_alignment_metadata(snapshot: &mut ProjectSnapshot) {
    let memberships = alignment_by_segment(snapshot);
    let now = timestamp();
    for bookmark in &mut snapshot.bookmarks {
        let next_alignment = memberships.get(&bookmark.segment_id).copied();
        if bookmark.alignment_id != next_alignment {
            bookmark.alignment_id = next_alignment;
            bookmark.updated_at = now.clone();
        }
    }
    for annotation in &mut snapshot.annotations {
        let linked_alignments = annotation
            .linked_segment_ids
            .iter()
            .map(|segment_id| memberships.get(segment_id).copied())
            .collect::<Vec<_>>();
        let next_alignment = linked_alignments
            .first()
            .copied()
            .flatten()
            .filter(|candidate| {
                linked_alignments
                    .iter()
                    .all(|alignment_id| *alignment_id == Some(*candidate))
            });
        if annotation.alignment_id != next_alignment {
            annotation.alignment_id = next_alignment;
            annotation.updated_at = now.clone();
        }
    }
}
