//! Segment regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_kernel::{KernelError, KernelService};
use jueming_protocol::{AnnotationCreateRequest, AnnotationStatus, BookmarkCreateRequest};

#[test]
fn segment_content_operations_migrate_sidecars_and_survive_history_round_trip() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("segment-content-history.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source = initial.segment_orders[0]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect::<Vec<_>>();
    let target = initial.segment_orders[1]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect::<Vec<_>>();
    let first_content = initial
        .segments
        .iter()
        .find(|segment| segment.segment_id == source[0])
        .unwrap()
        .content
        .clone();
    let second_content = initial
        .segments
        .iter()
        .find(|segment| segment.segment_id == source[1])
        .unwrap()
        .content
        .clone();
    let linked = service
        .link_segments(
            &path,
            &initial,
            vec![source[0], source[1]],
            vec![target[0], target[1]],
            true,
        )
        .expect("make a complex alignment");
    let alignment_id = linked
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids == vec![source[0], source[1]])
        .unwrap()
        .alignment_id;
    let bookmarked = service
        .create_bookmark(
            &path,
            &linked,
            BookmarkCreateRequest {
                segment_id: source[1],
                alignment_id: Some(alignment_id),
                label: "absorbed anchor".into(),
            },
        )
        .expect("bookmark absorbed segment");
    let annotated = service
        .create_annotation(
            &path,
            &bookmarked,
            AnnotationCreateRequest {
                title: "split later".into(),
                body: "keep both references".into(),
                status: AnnotationStatus::Draft,
                linked_segment_ids: vec![source[0], source[1]],
                alignment_id: Some(alignment_id),
                local_author_label: "Local".into(),
            },
        )
        .expect("annotation");
    let combined = format!("{first_content}{second_content}");
    let merged = service
        .merge_segments(&path, &annotated, vec![source[1], source[0]], &combined)
        .expect("merge consecutive aligned segments");
    assert_eq!(
        merged.revisions.last().unwrap().change_set.operation,
        "merge_segments"
    );
    assert!(
        merged
            .segments
            .iter()
            .all(|segment| segment.segment_id != source[1])
    );
    assert_eq!(merged.bookmarks[0].segment_id, source[0]);
    assert_eq!(merged.bookmarks[0].alignment_id, Some(alignment_id));
    assert_eq!(merged.annotations[0].linked_segment_ids, vec![source[0]]);
    assert_eq!(
        merged
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .unwrap()
            .source_segment_ids,
        vec![source[0]]
    );
    let previews = service.list_bookmarks(&merged).expect("bookmark preview");
    assert_eq!(previews[0].segment_content, combined);
    assert!(previews[0].after_context.is_some());
    assert_eq!(service.open_project(&path).unwrap(), merged);

    let split = service
        .split_segment(
            &path,
            &merged,
            source[0],
            vec![first_content.clone(), second_content.clone()],
        )
        .expect("lossless split");
    let split_id = split
        .segment_orders
        .iter()
        .find(|order| order.document_id == initial.documents[0].document_id)
        .unwrap()
        .entries[1]
        .segment_id;
    assert_ne!(split_id, source[1]);
    assert_eq!(split.bookmarks[0].segment_id, source[0]);
    assert_eq!(
        split.annotations[0].linked_segment_ids,
        vec![source[0], split_id]
    );
    assert_eq!(
        split
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .unwrap()
            .source_segment_ids,
        vec![source[0], split_id]
    );
    assert_eq!(service.open_project(&path).unwrap(), split);

    let undone = service.undo(&path, &split).expect("undo split");
    assert_eq!(undone.segments, merged.segments);
    let redone = service.redo(&path, &undone).expect("redo split");
    assert_eq!(redone.segments, split.segments);
    assert_eq!(service.open_project(&path).unwrap(), redone);
}

#[test]
fn segment_conflicts_are_atomic_and_relation_changes_migrate_sidecar_hints() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("segment-content-conflicts.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source = initial.segment_orders[0]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect::<Vec<_>>();
    let target = initial.segment_orders[1]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect::<Vec<_>>();
    let rejected = service
        .merge_segments(&path, &initial, vec![source[0], source[2]], "invalid")
        .expect_err("nonconsecutive selections cannot merge");
    assert!(matches!(rejected, KernelError::MergeSegmentsNotConsecutive));
    let rejected = service
        .split_segment(
            &path,
            &initial,
            source[0],
            vec!["different".into(), "text".into()],
        )
        .expect_err("non-lossless split cannot write");
    assert!(matches!(rejected, KernelError::SplitContentMismatch));
    assert_eq!(service.open_project(&path).unwrap(), initial);

    let old_alignment_id = initial
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids == vec![source[0]])
        .unwrap()
        .alignment_id;
    let bookmarked = service
        .create_bookmark(
            &path,
            &initial,
            BookmarkCreateRequest {
                segment_id: source[0],
                alignment_id: Some(old_alignment_id),
                label: "group relation".into(),
            },
        )
        .unwrap();
    let grouped = service
        .group_alignment(
            &path,
            &bookmarked,
            vec![old_alignment_id, initial.alignments[1].alignment_id],
            Vec::new(),
        )
        .expect("group full relations");
    let new_alignment_id = grouped
        .alignments
        .iter()
        .find(|alignment| {
            alignment.source_segment_ids == vec![source[0], source[1]]
                && alignment.target_segment_ids == vec![target[0], target[1]]
        })
        .unwrap()
        .alignment_id;
    assert_ne!(new_alignment_id, old_alignment_id);
    assert_eq!(grouped.bookmarks[0].alignment_id, Some(new_alignment_id));
    assert_eq!(
        grouped.revisions.last().unwrap().change_set.operation,
        "group_alignment"
    );
    assert_eq!(service.open_project(&path).unwrap(), grouped);

    let annotated = service
        .create_annotation(
            &path,
            &grouped,
            AnnotationCreateRequest {
                title: "spans the grouped relation".into(),
                body: "the hint must clear when the links split across children".into(),
                status: AnnotationStatus::Draft,
                linked_segment_ids: vec![source[0], source[1]],
                alignment_id: Some(new_alignment_id),
                local_author_label: "Local".into(),
            },
        )
        .expect("annotation on grouped relation");
    let ungrouped = service
        .ungroup_alignment(
            &path,
            &annotated,
            new_alignment_id,
            vec![vec![source[0]], vec![source[1]]],
            vec![vec![target[0]], vec![target[1]]],
        )
        .expect("explicitly ungroup relation");
    let first_child = ungrouped
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids == vec![source[0]])
        .expect("first child relation")
        .alignment_id;
    assert_eq!(ungrouped.bookmarks[0].alignment_id, Some(first_child));
    assert_eq!(ungrouped.annotations[0].alignment_id, None);
    assert_eq!(
        ungrouped.revisions.last().unwrap().change_set.operation,
        "ungroup_alignment"
    );

    let unlinked = service
        .unlink_alignment(&path, &ungrouped, first_child)
        .expect("unlink child relation");
    assert_eq!(unlinked.bookmarks[0].segment_id, source[0]);
    assert_eq!(unlinked.bookmarks[0].alignment_id, None);
    assert_eq!(
        unlinked.annotations[0].linked_segment_ids,
        vec![source[0], source[1]]
    );
    let undone = service.undo(&path, &unlinked).expect("undo unlink");
    assert_eq!(undone.bookmarks[0].alignment_id, Some(first_child));
    let redone = service.redo(&path, &undone).expect("redo unlink");
    assert_eq!(redone.bookmarks[0].alignment_id, None);
    assert_eq!(service.open_project(&path).unwrap(), redone);
}
