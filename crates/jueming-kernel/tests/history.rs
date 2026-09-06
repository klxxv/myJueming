//! History regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_core::RevisionId;
use jueming_kernel::KernelService;

#[test]
fn history_snapshots_support_edit_move_alignment_undo_and_redo_after_reopen() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("history.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let first = initial.segments[0].segment_id;
    let second = initial.segments[1].segment_id;
    let target = initial.segments[8].segment_id;
    let edited = service
        .update_segment(&path, &initial, first, "历史编辑后的句子")
        .expect("edit");
    let moved = service
        .move_segment(&path, &edited, first, Some(second), None)
        .expect("move");
    let changed = service
        .link_segments(&path, &moved, vec![first], vec![target], true)
        .expect("alignment");
    assert_eq!(changed.project.current_revision_id, RevisionId::new(4));

    let undo_one = service.undo(&path, &changed).expect("undo alignment");
    let reopened = service.open_project(&path).expect("reopen after undo");
    assert_eq!(reopened, undo_one);
    let undo_two = service.undo(&path, &reopened).expect("undo move");
    let redo_one = service.redo(&path, &undo_two).expect("redo move");
    let redo_two = service.redo(&path, &redo_one).expect("redo alignment");
    assert_eq!(redo_two.project.current_revision_id, RevisionId::new(8));
    assert_eq!(
        redo_two
            .segments
            .iter()
            .find(|segment| segment.segment_id == first)
            .unwrap()
            .content,
        "历史编辑后的句子"
    );
    assert!(
        redo_two
            .alignments
            .iter()
            .any(|alignment| alignment.source_segment_ids == vec![first]
                && alignment.target_segment_ids == vec![target])
    );
    assert_eq!(service.list_revisions(&path).unwrap().revisions.len(), 8);
}

#[test]
fn compare_and_restore_are_structured_and_append_only() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("restore.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let segment_id = initial.segments[0].segment_id;
    let edited = service
        .update_segment(&path, &initial, segment_id, "new text")
        .expect("edit");
    let comparison = service
        .compare_revision(&path, RevisionId::new(1), RevisionId::new(2))
        .expect("compare");
    assert_eq!(comparison.from_revision_id, RevisionId::new(1));
    assert_eq!(comparison.to_revision_id, RevisionId::new(2));
    assert!(
        comparison
            .segment_changes
            .iter()
            .any(|change| change.segment_id == segment_id)
    );
    let restored = service
        .restore_revision(&path, &edited, RevisionId::new(1))
        .expect("restore");
    assert_eq!(restored.project.current_revision_id, RevisionId::new(3));
    assert_eq!(restored.revisions.len(), 3);
    assert_eq!(
        restored
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .unwrap()
            .content,
        initial
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .unwrap()
            .content
    );
    assert_eq!(
        service.open_project(&path).expect("reopen restored"),
        restored
    );
}
