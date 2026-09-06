//! Public API regression tests.

#[cfg(not(windows))]
use jueming_core::CoreError;
use jueming_core::{
    ContentRef, DocumentId, Encoding, ImportProfile, ProjectId, RevisionId, Segment, SegmentId,
    SegmentKind, SegmentPreview, SegmentationMode, build_provisional_layout, decode_bytes,
    segment_text,
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
    #[cfg(windows)]
    assert_eq!(
        decode_bytes(&bytes, Encoding::Gb18030).unwrap().text,
        "中文"
    );
    #[cfg(not(windows))]
    assert!(matches!(
        decode_bytes(&bytes, Encoding::Gb18030),
        Err(CoreError::UnsupportedGb18030)
    ));
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
