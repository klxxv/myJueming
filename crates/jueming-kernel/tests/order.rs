//! Order regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_core::RevisionId;
use jueming_kernel::{KernelError, KernelService};

#[test]
fn cross_block_reorder_changes_only_document_order_and_keeps_alignments() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("ordered.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source_order = &snapshot.segment_orders[0];
    let first = source_order.entries[0].segment_id;
    let second = source_order.entries[1].segment_id;
    let third = source_order.entries[2].segment_id;
    let alignment_ids: Vec<_> = snapshot
        .alignments
        .iter()
        .map(|alignment| alignment.alignment_id)
        .collect();
    let moved = service
        .move_segment(&path, &snapshot, first, Some(second), Some(third))
        .expect("move segment");
    let moved_order = &moved.segment_orders[0];
    assert_eq!(moved_order.entries[0].segment_id, second);
    assert_eq!(moved_order.entries[1].segment_id, first);
    assert_eq!(
        moved
            .alignments
            .iter()
            .map(|alignment| alignment.alignment_id)
            .collect::<Vec<_>>(),
        alignment_ids
    );
    assert_eq!(moved.alignments, snapshot.alignments);
    assert_eq!(moved.project.current_revision_id, RevisionId::new(2));
}

#[test]
fn batch_reorder_requires_an_exact_permutation_and_survives_reopen() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("batch-ordered.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let mut requested: Vec<_> = snapshot.segment_orders[0]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect();
    requested.rotate_left(2);
    let reordered = service
        .reorder_segments(&path, &snapshot, requested.clone())
        .expect("batch reorder");
    assert_eq!(
        reordered.segment_orders[0]
            .entries
            .iter()
            .map(|entry| entry.segment_id)
            .collect::<Vec<_>>(),
        requested
    );
    assert_eq!(service.open_project(&path).expect("reopen"), reordered);

    let invalid = service
        .reorder_segments(&path, &reordered, vec![requested[0], requested[0]])
        .expect_err("duplicate IDs are rejected");
    assert!(matches!(invalid, KernelError::InvalidMoveAnchor));
}
