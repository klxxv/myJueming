//! Search regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_core::RevisionId;
use jueming_kernel::{KernelError, KernelService};
use jueming_protocol::{ReplaceApplyRequest, ReplacePreviewRequest, SearchSegmentsRequest};

#[test]
fn search_supports_literal_regex_language_and_invalid_regex_errors() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("search.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let request = SearchSegmentsRequest {
        project_id: snapshot.project.project_id,
        query: "government".into(),
        regex: false,
        case_sensitive: false,
        language_id: Some("en".into()),
        base_revision_id: snapshot.project.current_revision_id,
    };
    let hits = service
        .search_segments(&snapshot, &request)
        .expect("literal search");
    assert!(!hits.hits.is_empty());
    assert!(hits.hits.iter().all(|hit| hit.language_id == "en"));
    let regex_hits = service
        .search_segments(
            &snapshot,
            &SearchSegmentsRequest {
                query: "^Employment.*stable".into(),
                regex: true,
                ..request.clone()
            },
        )
        .expect("regex search");
    assert_eq!(regex_hits.hits.len(), 1);
    let invalid = service
        .search_segments(
            &snapshot,
            &SearchSegmentsRequest {
                query: "[".into(),
                regex: true,
                ..request
            },
        )
        .expect_err("invalid regex");
    assert!(matches!(invalid, KernelError::InvalidSearchRegex(_)));
}

#[test]
fn replace_is_one_revision_stale_safe_and_undoable() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("replace.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let request = ReplacePreviewRequest {
        project_id: snapshot.project.project_id,
        query: "stable".into(),
        replacement: "steady".into(),
        regex: false,
        case_sensitive: false,
        language_id: Some("en".into()),
        base_revision_id: snapshot.project.current_revision_id,
    };
    let preview = service
        .preview_replace(&snapshot, &request)
        .expect("preview");
    assert!(!preview.selected_segment_ids.is_empty());
    let replaced = service
        .apply_replace(
            &path,
            &snapshot,
            &ReplaceApplyRequest {
                preview: request.clone(),
                selected_segment_ids: preview.selected_segment_ids.clone(),
            },
        )
        .expect("apply replace");
    assert_eq!(replaced.project.current_revision_id, RevisionId::new(2));
    assert_eq!(replaced.revisions.len(), 2);
    let stale = service
        .apply_replace(
            &path,
            &replaced,
            &ReplaceApplyRequest {
                preview: request,
                selected_segment_ids: preview.selected_segment_ids,
            },
        )
        .expect_err("stale replace");
    assert!(matches!(stale, KernelError::StaleRevision { .. }));
    let undone = service.undo(&path, &replaced).expect("undo replace");
    assert_eq!(undone.project.current_revision_id, RevisionId::new(3));
    assert_eq!(undone.segments, snapshot.segments);
}
