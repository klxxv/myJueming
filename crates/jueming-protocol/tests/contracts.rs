use jueming_protocol::*;

#[test]
fn export_defaults_survive_the_dto_module_boundary() {
    let request: ExportRequest = serde_json::from_value(serde_json::json!({
        "format": "txt",
        "output_path": "parallel.txt"
    }))
    .unwrap();
    assert_eq!(request.format, ExportFormat::Txt);
    assert!(request.include_unlinked);
    assert_eq!(request.side_separator, " ");
}

#[test]
fn alignment_payload_defaults_and_legacy_alias_remain_compatible() {
    let source = SegmentId::new();
    let target = SegmentId::new();
    let alignment = AlignmentId::new();
    let link: LinkSegmentsPayload = serde_json::from_value(serde_json::json!({
        "source_segment_ids": [source],
        "target_segment_ids": [target]
    }))
    .unwrap();
    assert!(!link.replace_existing);
    let group: GroupAlignmentPayload = serde_json::from_value(serde_json::json!({
        "alignment_ids": [alignment]
    }))
    .unwrap();
    assert!(group.unlinked_segment_ids.is_empty());

    let payload: SplitAlignmentPayload = UngroupAlignmentPayload {
        alignment_id: alignment,
        source_groups: vec![vec![source]],
        target_groups: vec![vec![target]],
    };
    let encoded = serde_json::to_value(&payload).unwrap();
    let decoded: UngroupAlignmentPayload = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn legacy_and_canonical_command_wire_names_are_unchanged() {
    for (kind, wire) in [
        (CommandKind::MergeAlignments, "merge_alignments"),
        (CommandKind::MergeAlignment, "merge_alignment"),
        (CommandKind::SplitAlignment, "split_alignment"),
        (CommandKind::GroupAlignment, "group_alignment"),
        (CommandKind::UngroupAlignment, "ungroup_alignment"),
        (CommandKind::MergeSegments, "merge_segments"),
        (CommandKind::SplitSegment, "split_segment"),
    ] {
        let encoded = serde_json::to_value(kind).unwrap();
        assert_eq!(encoded, wire);
        assert_eq!(
            serde_json::from_value::<CommandKind>(encoded).unwrap(),
            kind
        );
    }
}

#[test]
fn envelope_uses_contract_version_and_stable_ids() {
    let project = ProjectId::new();
    let envelope = CommandEnvelope::new(
        CommandId::new(),
        project,
        RevisionId::new(0),
        CommandKind::ApplySegmentation,
        ApplySegmentationPayload {
            document_id: DocumentId::new(),
            encoding: Encoding::Utf8,
            mode: SegmentationMode::NonEmptyLine,
            profile: ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
        },
    );
    let value = serde_json::to_value(&envelope).unwrap();
    assert_eq!(value["contract_version"], "1.0");
    assert!(value["command_id"].as_str().unwrap().contains('-'));
    assert_eq!(
        serde_json::from_value::<CommandEnvelope<ApplySegmentationPayload>>(value)
            .unwrap()
            .project_id,
        project
    );
}

#[test]
fn language_catalogue_uses_ten_ltr_base_codes_and_reads_legacy_chinese() {
    assert_eq!(COMMON_LTR_LANGUAGES.len(), 10);
    assert_eq!(
        SupportedLanguageId::parse_compatible("zh-CN"),
        Some(SupportedLanguageId::Zh)
    );
    assert_eq!(SupportedLanguageId::parse_compatible("ar"), None);
    assert_eq!(SupportedLanguageId::Ru.as_str(), "ru");
}

#[test]
fn bookmark_preview_serializes_as_a_flat_read_model() {
    let preview = BookmarkPreview {
        bookmark: Bookmark {
            bookmark_id: BookmarkId::new(),
            project_id: ProjectId::new(),
            segment_id: SegmentId::new(),
            alignment_id: Some(AlignmentId::new()),
            label: "Review this passage".into(),
            created_at: "2026-08-29T00:00:00Z".into(),
            updated_at: "2026-08-29T00:00:00Z".into(),
        },
        document_title: "Source".into(),
        language_id: "zh".into(),
        segment_content: "当前正文".into(),
        before_context: Some("上文".into()),
        after_context: Some("下文".into()),
    };

    let value = serde_json::to_value(preview).unwrap();
    assert!(value.get("bookmark").is_none());
    assert!(value["bookmark_id"].as_str().is_some());
    assert!(value["segment_id"].as_str().is_some());
    assert_eq!(value["segment_content"], "当前正文");
    assert_eq!(value["language_id"], "zh");
}

#[test]
fn automatic_detection_is_a_preview_option_not_a_stored_encoding() {
    let old_request = serde_json::json!({
        "input": {"kind":"paste", "label":"text", "text":"中文"},
        "profile": ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine)
    });
    let request: jueming_protocol::PreviewImportRequest =
        serde_json::from_value(old_request.clone()).unwrap();
    assert!(!request.auto_detect_encoding);
    let mut automatic = old_request;
    automatic["auto_detect_encoding"] = true.into();
    let request: jueming_protocol::PreviewImportRequest =
        serde_json::from_value(automatic).unwrap();
    assert!(request.auto_detect_encoding);
    assert!(serde_json::from_str::<Encoding>("\"auto\"").is_err());
    for (value, expected) in [
        ("utf8", Encoding::Utf8),
        ("utf8-bom", Encoding::Utf8Bom),
        ("gb18030", Encoding::Gb18030),
        ("utf-16le", Encoding::Utf16Le),
        ("shift-jis", Encoding::ShiftJis),
    ] {
        assert_eq!(serde_json::to_value(expected).unwrap(), value);
        assert_eq!(
            serde_json::from_value::<Encoding>(value.into()).unwrap(),
            expected
        );
    }
}
