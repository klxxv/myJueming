//! Alignment regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_core::RevisionId;
use jueming_kernel::{KernelError, KernelService};

#[test]
fn link_supports_all_cardinalities_and_unlinks_existing_occupants() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("cardinality.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source: Vec<_> = snapshot
        .segments
        .iter()
        .filter(|segment| segment.document_id == snapshot.documents[0].document_id)
        .map(|segment| segment.segment_id)
        .collect();
    let target: Vec<_> = snapshot
        .segments
        .iter()
        .filter(|segment| segment.document_id == snapshot.documents[1].document_id)
        .map(|segment| segment.segment_id)
        .collect();

    let conflict = service
        .link_segments(
            &path,
            &snapshot,
            vec![source[0]],
            vec![target[0], target[1]],
            false,
        )
        .expect_err("occupied link requires explicit replacement");
    assert!(matches!(conflict, KernelError::AlignmentSelectionConflict));
    let one_to_many = service
        .link_segments(
            &path,
            &snapshot,
            vec![source[0]],
            vec![target[0], target[1]],
            true,
        )
        .expect("1:n link");
    assert_eq!(
        one_to_many
            .alignments
            .iter()
            .find(|alignment| alignment.source_segment_ids == vec![source[0]])
            .unwrap()
            .cardinality,
        jueming_core::Cardinality::OneToMany
    );
    let many_to_one = service
        .link_segments(
            &path,
            &one_to_many,
            vec![source[1], source[2]],
            vec![target[1]],
            true,
        )
        .expect("n:1 link");
    assert_eq!(
        many_to_one
            .alignments
            .iter()
            .find(|alignment| alignment.target_segment_ids == vec![target[1]])
            .unwrap()
            .cardinality,
        jueming_core::Cardinality::ManyToOne
    );
    let many_to_many = service
        .link_segments(
            &path,
            &many_to_one,
            vec![source[3], source[4]],
            vec![target[2], target[3]],
            true,
        )
        .expect("n:m link");
    assert!(
        many_to_many
            .alignments
            .iter()
            .any(|alignment| alignment.cardinality == jueming_core::Cardinality::ManyToMany)
    );
    assert_eq!(many_to_many.project.current_revision_id, RevisionId::new(4));
}

#[test]
fn unlink_merge_split_and_reopen_preserve_ids_and_revisions() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("relations.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source: Vec<_> = initial
        .segments
        .iter()
        .filter(|segment| segment.document_id == initial.documents[0].document_id)
        .map(|segment| segment.segment_id)
        .collect();
    let target: Vec<_> = initial
        .segments
        .iter()
        .filter(|segment| segment.document_id == initial.documents[1].document_id)
        .map(|segment| segment.segment_id)
        .collect();
    let linked = service
        .link_segments(
            &path,
            &initial,
            vec![source[0], source[1]],
            vec![target[0], target[1]],
            true,
        )
        .expect("n:m");
    let created_id = linked
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids.len() == 2)
        .unwrap()
        .alignment_id;
    let merged = service
        .merge_alignments(
            &path,
            &linked,
            vec![created_id, linked.alignments[2].alignment_id],
            Vec::new(),
        )
        .expect("merge");
    assert_eq!(merged.project.current_revision_id, RevisionId::new(3));
    assert_eq!(merged.segments, linked.segments);
    assert_eq!(
        merged.revisions.last().unwrap().change_set.operation,
        "group_alignment"
    );
    let merged_id = merged
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids.len() >= 2)
        .unwrap()
        .alignment_id;
    let split = service
        .split_alignment(&path, &merged, merged_id, Vec::new(), Vec::new())
        .expect("split into pairs");
    assert_eq!(split.segments, merged.segments);
    assert_eq!(
        split.revisions.last().unwrap().change_set.operation,
        "ungroup_alignment"
    );
    assert!(
        split
            .alignments
            .iter()
            .all(|alignment| alignment.cardinality == jueming_core::Cardinality::OneToOne)
    );
    let removed = split.alignments[0].alignment_id;
    let unlinked = service
        .unlink_alignment(&path, &split, removed)
        .expect("unlink");
    assert!(
        !unlinked
            .alignments
            .iter()
            .any(|alignment| alignment.alignment_id == removed)
    );
    assert_eq!(service.open_project(&path).expect("reopen"), unlinked);
}

#[test]
fn merge_combines_an_active_alignment_with_unlinked_segments() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("merge-unlinked.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source: Vec<_> = initial
        .segments
        .iter()
        .filter(|segment| segment.document_id == initial.documents[0].document_id)
        .map(|segment| segment.segment_id)
        .collect();
    let target: Vec<_> = initial
        .segments
        .iter()
        .filter(|segment| segment.document_id == initial.documents[1].document_id)
        .map(|segment| segment.segment_id)
        .collect();
    let kept_alignment_id = initial
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids.contains(&source[0]))
        .expect("first alignment")
        .alignment_id;
    let detached_alignment_id = initial
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids.contains(&source[1]))
        .expect("second alignment")
        .alignment_id;
    let unlinked = service
        .unlink_alignment(&path, &initial, detached_alignment_id)
        .expect("unlink second alignment");
    let merged = service
        .merge_alignments(
            &path,
            &unlinked,
            vec![kept_alignment_id],
            vec![source[1], target[1]],
        )
        .expect("merge active alignment with unlinked segments");

    let replacement = merged
        .alignments
        .iter()
        .find(|alignment| {
            alignment.source_segment_ids == vec![source[0], source[1]]
                && alignment.target_segment_ids == vec![target[0], target[1]]
        })
        .expect("merged n:m alignment");
    assert_eq!(
        replacement.cardinality,
        jueming_core::Cardinality::ManyToMany
    );
    assert_ne!(replacement.alignment_id, kept_alignment_id);
    assert!(
        !merged
            .alignments
            .iter()
            .any(|alignment| alignment.alignment_id == kept_alignment_id)
    );
    assert_eq!(merged.project.current_revision_id, RevisionId::new(3));
    assert_eq!(service.open_project(&path).expect("reopen"), merged);
}

#[test]
fn merge_rejects_a_segment_that_is_still_aligned_without_writing() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("merge-conflict.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let first_alignment = initial.alignments[0].alignment_id;
    let occupied_segment = initial.alignments[1].source_segment_ids[0];
    let error = service
        .merge_alignments(
            &path,
            &initial,
            vec![first_alignment],
            vec![occupied_segment],
        )
        .expect_err("occupied segment must not be accepted as unlinked");

    assert!(matches!(
        error,
        KernelError::MergeSegmentAlreadyAligned(id) if id == occupied_segment
    ));
    assert_eq!(service.open_project(&path).expect("reopen"), initial);
}

#[test]
fn invalid_duplicate_selection_is_rejected_without_revision_or_disk_change() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("rollback.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source_id = initial.segments[0].segment_id;
    let target_id = initial.segments[8].segment_id;
    let error = service
        .link_segments(
            &path,
            &initial,
            vec![source_id, source_id],
            vec![target_id],
            false,
        )
        .expect_err("duplicate selection must fail");
    assert!(matches!(error, KernelError::InvalidAlignmentSelection));
    assert_eq!(service.open_project(&path).expect("reopen"), initial);
}

#[test]
fn explicit_split_groups_must_partition_original_alignment() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("split-validation.jm");
    let service = KernelService;
    let initial = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source: Vec<_> = initial
        .segments
        .iter()
        .filter(|segment| segment.document_id == initial.documents[0].document_id)
        .map(|segment| segment.segment_id)
        .collect();
    let target: Vec<_> = initial
        .segments
        .iter()
        .filter(|segment| segment.document_id == initial.documents[1].document_id)
        .map(|segment| segment.segment_id)
        .collect();
    let linked = service
        .link_segments(
            &path,
            &initial,
            vec![source[0], source[1]],
            vec![target[0], target[1]],
            true,
        )
        .expect("n:m");
    let alignment_id = linked
        .alignments
        .iter()
        .find(|alignment| alignment.source_segment_ids.len() == 2)
        .unwrap()
        .alignment_id;
    let error = service
        .split_alignment(
            &path,
            &linked,
            alignment_id,
            vec![vec![source[0]]],
            vec![vec![target[0]]],
        )
        .expect_err("incomplete partition must fail");
    assert!(matches!(error, KernelError::SplitGroupsMismatch));
    let no_op = service
        .ungroup_alignment(
            &path,
            &linked,
            alignment_id,
            vec![vec![source[0], source[1]]],
            vec![vec![target[0], target[1]]],
        )
        .expect_err("ungroup must create at least two relations");
    assert!(matches!(no_op, KernelError::SplitGroupsMismatch));
    assert_eq!(service.open_project(&path).expect("reopen"), linked);
}
