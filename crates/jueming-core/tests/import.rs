//! Public API regression tests.

use jueming_core::{
    ContentRef, DocumentId, Encoding, ImportCleanup, ImportProfile, ProjectId, RevisionId, Segment,
    SegmentBoundary, SegmentId, SegmentKind, SegmentPreview, SegmentationMode,
    build_provisional_layout, decode_bytes, segment_text,
};

#[test]
fn government_fixture_lines_make_provisional_layout() {
    let source = segment_text(
        "甲\n乙\n丙\n",
        &ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
    );
    let target = segment_text(
        "A\nB\nC\n",
        &ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
    );
    let project = ProjectId::new();
    let make = |items: &[SegmentPreview]| {
        items
            .iter()
            .enumerate()
            .map(|(i, item)| Segment {
                segment_id: SegmentId::new(),
                document_id: DocumentId::new(),
                kind: SegmentKind::Sentence,
                content_ref: ContentRef { key: i.to_string() },
                content: item.content.clone(),
                content_hash: String::new(),
                created_revision_id: RevisionId::new(1),
                updated_revision_id: RevisionId::new(1),
            })
            .collect::<Vec<_>>()
    };
    let layout = build_provisional_layout(
        project,
        &make(&source.segments),
        &make(&target.segments),
        RevisionId::new(1),
    )
    .unwrap();
    assert_eq!(layout.alignments.len(), 3);
    assert!(layout.source_unlinked.is_empty() && layout.target_unlinked.is_empty());
}

#[test]
fn bundled_government_fixture_is_eight_by_eight() {
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
    let source = segment_text(
        include_str!("../../../tests/fixtures/government-report/report_zh.txt"),
        &profile,
    );
    let target = segment_text(
        include_str!("../../../tests/fixtures/government-report/report_en.txt"),
        &profile,
    );
    assert_eq!(source.segments.len(), 8);
    assert_eq!(target.segments.len(), 8);
    let make = |items: &[SegmentPreview]| {
        items
            .iter()
            .map(|item| Segment {
                segment_id: SegmentId::new(),
                document_id: DocumentId::new(),
                kind: SegmentKind::Sentence,
                content_ref: ContentRef {
                    key: item.ordinal.to_string(),
                },
                content: item.content.clone(),
                content_hash: String::new(),
                created_revision_id: RevisionId::new(1),
                updated_revision_id: RevisionId::new(1),
            })
            .collect::<Vec<_>>()
    };
    let layout = build_provisional_layout(
        ProjectId::new(),
        &make(&source.segments),
        &make(&target.segments),
        RevisionId::new(1),
    )
    .unwrap();
    assert_eq!(layout.alignments.len(), 8);
    assert!(layout.source_unlinked.is_empty());
    assert!(layout.target_unlinked.is_empty());
}

#[test]
fn gb18030_is_strict() {
    let bytes = [0xd6, 0xd0, 0xce, 0xc4];
    assert_eq!(
        decode_bytes(&bytes, Encoding::Gb18030).unwrap().text,
        "中文"
    );
    assert!(decode_bytes(&[0x81], Encoding::Gb18030).is_err());
    assert!(decode_bytes(&[0xff], Encoding::Utf8).is_err());
}

#[test]
fn legacy_preview_removes_wrappers_pos_and_cjk_spaces() {
    let p = ImportProfile::new(Encoding::Utf8, SegmentationMode::LegacyTaggedLine);
    let preview = segment_text("<seg>中 文_NN ，_, test_VB</seg>\n<seg>第二 句。</seg>", &p);
    assert_eq!(
        preview
            .segments
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>(),
        vec!["中文， test", "第二句。"]
    );
}

#[test]
fn legacy_preview_preserves_physical_lines_with_malformed_wrappers() {
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::LegacyTaggedLine);
    let source = segment_text(
        "<seg> 阿 古 顿 巴\n<seg> 产 生 故 事 , 牲 牛 .</seg>\n<seg>\n",
        &profile,
    );
    let target = segment_text(
        "<seg>Akhu_FW Tenpa_NP1\n<seg> yaks_NN2 ,_ horses_NN2 .</seg>\n<seg>\n",
        &profile,
    );
    assert_eq!(
        source
            .segments
            .iter()
            .map(|segment| segment.content.as_str())
            .collect::<Vec<_>>(),
        vec!["阿古顿巴", "产生故事,牲牛."]
    );
    assert_eq!(
        target
            .segments
            .iter()
            .map(|segment| segment.content.as_str())
            .collect::<Vec<_>>(),
        vec!["Akhu Tenpa", "yaks, horses."]
    );
}

#[test]
fn sentence_preview_explains_actual_boundaries_without_losing_original_fragments() {
    let preview = segment_text(
        "第一句。Second!\n没有末尾标点",
        &ImportProfile::new(Encoding::Utf8, SegmentationMode::SentenceRules),
    );
    assert_eq!(preview.segments.len(), 3);
    for (segment, marker) in preview.segments[..2].iter().zip(['。', '!']) {
        assert_eq!(segment.boundary, SegmentBoundary::SentencePunctuation);
        assert_eq!(segment.boundary_marker, Some(marker));
        assert_eq!(segment.original_text, segment.content);
        assert!(segment.cleanups.is_empty());
    }
    assert_eq!(preview.segments[2].boundary, SegmentBoundary::TextEnd);
    assert_eq!(preview.segments[2].boundary_marker, None);
    assert_eq!(preview.segments[2].original_text, "没有末尾标点");
}

#[test]
fn seg_preview_reports_only_cleanup_stages_that_changed_the_fragment() {
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::LegacyTaggedLine);
    let preview = segment_text("<seg>中 文_NN ，_, test_VB</seg>\n\nplain\n<seg>", &profile);
    assert_eq!(preview.segments.len(), 2);
    let first = &preview.segments[0];
    assert_eq!(first.boundary, SegmentBoundary::NonEmptyLine);
    assert_eq!(first.original_text, "<seg>中 文_NN ，_, test_VB</seg>");
    assert_eq!(first.content, "中文， test");
    assert_eq!(
        first.cleanups,
        vec![
            ImportCleanup::SegWrappers,
            ImportCleanup::PosSuffixesAndWhitespace,
            ImportCleanup::CjkSpaces
        ]
    );
    assert!(preview.segments[1].cleanups.is_empty());
    assert_eq!(preview.segments[1].ordinal, 1);
}

#[test]
fn non_empty_line_preview_keeps_punctuation_and_seg_markers() {
    let preview = segment_text(
        "\n<seg>One. Two!</seg>\r\n\n第三句",
        &ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
    );
    assert_eq!(preview.segments.len(), 2);
    assert_eq!(preview.segments[0].content, "<seg>One. Two!</seg>");
    assert!(
        preview
            .segments
            .iter()
            .all(|segment| segment.boundary == SegmentBoundary::NonEmptyLine
                && segment.cleanups.is_empty())
    );
}
