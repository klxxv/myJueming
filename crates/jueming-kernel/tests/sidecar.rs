//! Sidecar regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_core::RevisionId;
use jueming_kernel::KernelService;
use jueming_protocol::{
    AnnotationCreateRequest, AnnotationStatus, AnnotationUpdateRequest, BookmarkCreateRequest,
    BookmarkUpdateRequest,
};

#[test]
fn bookmarks_and_annotations_follow_stable_segment_anchors() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("sidecars.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let segment_id = snapshot.segments[0].segment_id;
    let alignment_id = snapshot.alignments[0].alignment_id;
    let bookmarked = service
        .create_bookmark(
            &path,
            &snapshot,
            BookmarkCreateRequest {
                segment_id,
                alignment_id: Some(alignment_id),
                label: "review".into(),
            },
        )
        .expect("bookmark");
    let bookmark_id = bookmarked.bookmarks[0].bookmark_id;
    let moved = service
        .move_segment(&path, &bookmarked, segment_id, None, None)
        .expect("reorder");
    let updated = service
        .update_bookmark(
            &path,
            &moved,
            BookmarkUpdateRequest {
                bookmark_id,
                segment_id,
                alignment_id: Some(alignment_id),
                label: "still review".into(),
            },
        )
        .expect("bookmark survives reorder");
    assert_eq!(updated.bookmarks[0].segment_id, segment_id);
    let annotated = service
        .create_annotation(
            &path,
            &updated,
            AnnotationCreateRequest {
                title: "Terminology".into(),
                body: "Check this phrase".into(),
                status: AnnotationStatus::Draft,
                linked_segment_ids: vec![segment_id],
                alignment_id: Some(alignment_id),
                local_author_label: "Local".into(),
            },
        )
        .expect("annotation");
    let annotation_id = annotated.annotations[0].annotation_id;
    let in_progress = service
        .update_annotation(
            &path,
            &annotated,
            AnnotationUpdateRequest {
                annotation_id,
                title: "Terminology".into(),
                body: "Checked".into(),
                status: AnnotationStatus::InProgress,
                linked_segment_ids: vec![segment_id],
                alignment_id: Some(alignment_id),
            },
        )
        .expect("annotation update");
    let resolved = service
        .resolve_annotation(&path, &in_progress, annotation_id)
        .expect("resolve");
    assert_eq!(resolved.annotations[0].status, AnnotationStatus::Resolved);
    assert_eq!(resolved.project.current_revision_id, RevisionId::new(7));
    assert_eq!(resolved.revisions.len(), 7);
}
