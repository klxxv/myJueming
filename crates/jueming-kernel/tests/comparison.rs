mod common;
use jueming_core::{Alignment, RevisionId};
use jueming_kernel::{KernelService, validate_snapshot};
use jueming_protocol::{AlignmentGapEdge, ExportFormat, ExportRequest, TextInput};

#[test]
fn three_translations_round_trip_and_pair_commands_preserve_other_relations() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("three.jm");
    let kernel = KernelService;
    let mut request = common::government_request(&path);
    request.additional_targets = vec![request.target.clone(), request.target.clone()];
    request.additional_targets[0].title = "Second translation".into();
    request.additional_targets[1].title = "Third translation".into();
    let initial = kernel.create_project(&request).unwrap();
    assert_eq!(initial.contract_version, "2.0");
    assert_eq!(initial.documents.len(), 4);
    assert_eq!(initial.source_assets.len(), 4);
    assert!(
        initial
            .source_assets
            .iter()
            .all(|asset| asset.import_profile.is_some())
    );
    assert_eq!(
        initial
            .project
            .comparison
            .as_ref()
            .unwrap()
            .target_document_ids
            .len(),
        3
    );
    assert_eq!(kernel.open_project(&path).unwrap(), initial);
    kernel.summarize(&initial).unwrap();
    let selected = initial.alignments[8].clone();
    let other = initial
        .alignments
        .iter()
        .filter(|alignment| alignment.alignment_id != selected.alignment_id)
        .cloned()
        .collect::<Vec<_>>();
    let unlinked = kernel
        .unlink_alignment(&path, &initial, selected.alignment_id)
        .unwrap();
    assert_eq!(unlinked.alignments, other);
    assert_eq!(
        kernel.research_context_fingerprint(&initial, selected.source_segment_ids[0]),
        kernel.research_context_fingerprint(&unlinked, selected.source_segment_ids[0]),
        "other translations do not change primary research evidence"
    );
    let linked = kernel
        .link_segments(
            &path,
            &unlinked,
            selected.source_segment_ids.clone(),
            selected.target_segment_ids.clone(),
            false,
        )
        .unwrap();
    for relation in &other {
        assert!(linked.alignments.contains(relation));
    }
    // Source Split inherits every translation membership and lossless Merge restores all refs.
    let source = linked
        .segments
        .iter()
        .find(|segment| segment.segment_id == selected.source_segment_ids[0])
        .unwrap();
    let boundary = source.content.char_indices().nth(1).unwrap().0;
    let split = kernel
        .split_segment(
            &path,
            &linked,
            source.segment_id,
            vec![
                source.content[..boundary].into(),
                source.content[boundary..].into(),
            ],
        )
        .unwrap();
    let inherited = split
        .alignments
        .iter()
        .filter(|relation| relation.source_segment_ids.contains(&source.segment_id))
        .collect::<Vec<_>>();
    assert_eq!(inherited.len(), 3);
    assert!(
        inherited
            .iter()
            .all(|relation| relation.source_segment_ids.len() == 2)
    );
    let merged = kernel
        .merge_segments(
            &path,
            &split,
            inherited[0].source_segment_ids.clone(),
            &source.content,
        )
        .unwrap();
    assert_eq!(
        merged
            .alignments
            .iter()
            .map(|relation| (&relation.source_segment_ids, &relation.target_segment_ids))
            .collect::<Vec<_>>(),
        linked
            .alignments
            .iter()
            .map(|relation| (&relation.source_segment_ids, &relation.target_segment_ids))
            .collect::<Vec<_>>()
    );
    let third = initial.documents[3].document_id;
    let preserved = merged
        .alignments
        .iter()
        .filter(|relation| {
            !initial.segments.iter().any(|segment| {
                segment.document_id == third
                    && relation.target_segment_ids.contains(&segment.segment_id)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let gap = kernel
        .insert_alignment_gap_for_document(
            &path,
            &merged,
            initial.segment_orders[0].entries[2].segment_id,
            AlignmentGapEdge::Before,
            Some(third),
        )
        .unwrap();
    for relation in preserved {
        assert!(gap.alignments.contains(&relation));
    }
    let restored = kernel
        .restore_revision(&path, &gap, RevisionId::new(1))
        .unwrap();
    assert_eq!(restored.documents, initial.documents);
    assert_eq!(restored.alignments, initial.alignments);
    assert_eq!(restored.contract_version, "2.0");
    assert_eq!(kernel.open_project(&path).unwrap(), restored);
    for format in [ExportFormat::Txt, ExportFormat::Xml, ExportFormat::Json] {
        let bytes = kernel
            .export(
                &restored,
                &ExportRequest {
                    format,
                    output_path: String::new(),
                    include_unlinked: true,
                    side_separator: " ".into(),
                },
            )
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Second translation"));
        assert!(text.contains("Third translation"));
    }
}

#[test]
fn multi_pair_validation_and_preview_digest_fail_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("invalid.jm");
    let kernel = KernelService;
    let mut request = common::government_request(&path);
    request.additional_targets.push(request.target.clone());
    request.additional_targets[0].expected_sha256 = Some("outdated".into());
    assert!(kernel.create_project(&request).is_err());
    assert!(!path.exists());
    let preview = kernel
        .preview_import(
            &request.additional_targets[0].input,
            &request.additional_targets[0].profile,
        )
        .unwrap();
    request.additional_targets[0].expected_sha256 = Some(preview.sha256);
    let initial = kernel.create_project(&request).unwrap();
    let mut duplicated = initial.clone();
    let relation = &initial.alignments[0];
    duplicated.alignments.push(
        Alignment::new(
            initial.project.project_id,
            relation.source_segment_ids.clone(),
            relation.target_segment_ids.clone(),
            RevisionId::new(1),
        )
        .unwrap(),
    );
    assert!(validate_snapshot(&duplicated).is_err());
    let cross_pair_targets = vec![
        initial.alignments[0].target_segment_ids[0],
        initial.alignments[8].target_segment_ids[0],
    ];
    assert!(
        kernel
            .link_segments(
                &path,
                &initial,
                relation.source_segment_ids.clone(),
                cross_pair_targets,
                true
            )
            .is_err()
    );
    // A source Merge that crosses a relation in any translation must fail.
    assert!(
        kernel
            .merge_segments(
                &path,
                &initial,
                vec![
                    initial.alignments[0].source_segment_ids[0],
                    initial.alignments[1].source_segment_ids[0]
                ],
                "合并"
            )
            .is_err()
    );
    assert_eq!(kernel.open_project(&path).unwrap(), initial);
    let blank_path = temp.path().join("blank.jm");
    request.project_path = blank_path.to_string_lossy().into();
    request.additional_targets[0].expected_sha256 = None;
    request.additional_targets[0].input = TextInput::Paste {
        label: "empty".into(),
        text: "\n".into(),
    };
    assert!(kernel.create_project(&request).is_err());
    assert!(!blank_path.exists());
}

#[test]
fn same_language_search_and_replace_remain_scoped_to_one_translation() {
    use jueming_protocol::{ReplaceApplyRequest, ReplacePreviewRequest, SearchSegmentsRequest};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("search.jm");
    let kernel = KernelService;
    let mut request = common::government_request(&path);
    request.additional_targets = vec![request.target.clone(), request.target.clone()];
    let initial = kernel.create_project(&request).unwrap();
    let document_id = initial.documents[2].document_id;
    let search = kernel
        .search_segments(
            &initial,
            &SearchSegmentsRequest {
                document_ids: Some(vec![document_id]),
                project_id: initial.project.project_id,
                query: ".".into(),
                regex: true,
                case_sensitive: false,
                language_id: None,
                base_revision_id: initial.project.current_revision_id,
            },
        )
        .unwrap();
    assert_eq!(search.hits.len(), 8);
    assert!(search.hits.iter().all(|hit| hit.document_id == document_id));
    let replaced = kernel
        .apply_replace(
            &path,
            &initial,
            &ReplaceApplyRequest {
                preview: ReplacePreviewRequest {
                    document_ids: Some(vec![document_id]),
                    project_id: initial.project.project_id,
                    query: ".".into(),
                    replacement: "x".into(),
                    regex: true,
                    case_sensitive: false,
                    language_id: None,
                    base_revision_id: initial.project.current_revision_id,
                },
                selected_segment_ids: vec![],
            },
        )
        .unwrap();
    for segment in initial
        .segments
        .iter()
        .filter(|segment| segment.document_id != document_id)
    {
        assert!(replaced.segments.contains(segment));
    }
    assert_eq!(replaced.alignments, initial.alignments);
}

#[test]
fn changed_file_after_preview_is_rejected_before_project_creation() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("changed.jm");
    let file = temp.path().join("third.txt");
    std::fs::write(&file, "First version.").unwrap();
    let kernel = KernelService;
    let mut request = common::government_request(&path);
    let mut extra = request.target.clone();
    extra.input = TextInput::File {
        path: file.to_string_lossy().into(),
    };
    extra.expected_sha256 = Some(
        kernel
            .preview_import(&extra.input, &extra.profile)
            .unwrap()
            .sha256,
    );
    request.additional_targets.push(extra);
    std::fs::write(&file, "New version.").unwrap();
    assert!(kernel.create_project(&request).is_err());
    assert!(!path.exists());
    request.additional_targets[0].expected_sha256 = Some(
        kernel
            .preview_import(
                &request.additional_targets[0].input,
                &request.additional_targets[0].profile,
            )
            .unwrap()
            .sha256,
    );
    let created = kernel.create_project(&request).unwrap();
    assert!(
        created
            .segments
            .iter()
            .any(|segment| segment.content == "New version.")
    );
}
