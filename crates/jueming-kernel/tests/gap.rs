//! Gap regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_core::RevisionId;
use jueming_kernel::{KernelError, KernelService};
use jueming_protocol::AlignmentGapEdge;

#[test]
fn source_gap_before_unlinks_the_opposite_segment_and_realigns_the_suffix() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("source-gap.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source: Vec<_> = initial.segment_orders[0]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect();
    let target: Vec<_> = initial.segment_orders[1]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect();
    let preserved_prefix_ids: Vec<_> = initial.alignments[..2]
        .iter()
        .map(|alignment| alignment.alignment_id)
        .collect();

    let shifted = service
        .insert_alignment_gap(&path, &initial, source[2], AlignmentGapEdge::Before)
        .expect("insert source gap before the third source segment");

    assert!(preserved_prefix_ids.iter().all(|id| {
        shifted
            .alignments
            .iter()
            .any(|alignment| alignment.alignment_id == *id)
    }));
    assert!(
        !shifted
            .alignments
            .iter()
            .any(|alignment| alignment.target_segment_ids.contains(&target[2]))
    );
    for offset in 0..5 {
        assert!(shifted.alignments.iter().any(|alignment| {
            alignment.source_segment_ids == vec![source[2 + offset]]
                && alignment.target_segment_ids == vec![target[3 + offset]]
        }));
    }
    assert!(
        !shifted
            .alignments
            .iter()
            .any(|alignment| alignment.source_segment_ids.contains(&source[7]))
    );
    assert_eq!(shifted.project.current_revision_id, RevisionId::new(2));
    assert_eq!(
        shifted.revisions.last().unwrap().change_set.operation,
        "insert_alignment_gap"
    );
    assert_eq!(service.open_project(&path).expect("reopen"), shifted);
}

#[test]
fn target_gap_after_preserves_the_selected_pair_and_realigns_following_pairs() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("target-gap.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source: Vec<_> = initial.segment_orders[0]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect();
    let target: Vec<_> = initial.segment_orders[1]
        .entries
        .iter()
        .map(|entry| entry.segment_id)
        .collect();
    let selected_alignment_id = initial
        .alignments
        .iter()
        .find(|alignment| alignment.target_segment_ids == vec![target[2]])
        .unwrap()
        .alignment_id;

    let shifted = service
        .insert_alignment_gap(&path, &initial, target[2], AlignmentGapEdge::After)
        .expect("insert target gap below the third target segment");

    assert!(
        shifted
            .alignments
            .iter()
            .any(|alignment| alignment.alignment_id == selected_alignment_id)
    );
    assert!(
        !shifted
            .alignments
            .iter()
            .any(|alignment| alignment.source_segment_ids.contains(&source[3]))
    );
    for offset in 0..4 {
        assert!(shifted.alignments.iter().any(|alignment| {
            alignment.source_segment_ids == vec![source[4 + offset]]
                && alignment.target_segment_ids == vec![target[3 + offset]]
        }));
    }
    assert!(
        !shifted
            .alignments
            .iter()
            .any(|alignment| alignment.target_segment_ids.contains(&target[7]))
    );
    assert_eq!(service.open_project(&path).expect("reopen"), shifted);
}

#[test]
fn alignment_gap_rejects_a_tail_without_writing() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("invalid-gap.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let last_source = initial.segment_orders[0].entries.last().unwrap().segment_id;

    let error = service
        .insert_alignment_gap(&path, &initial, last_source, AlignmentGapEdge::After)
        .expect_err("a gap without a later pair must be rejected");

    assert!(matches!(error, KernelError::AlignmentGapOutOfRange));
    assert_eq!(service.open_project(&path).expect("reopen"), initial);
}
