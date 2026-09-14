mod common;
use jueming_kernel::{KernelService, validate_snapshot};
use jueming_protocol::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[test]
fn research_facts_round_trip_history_and_reject_stale_or_invalid_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("research.jm");
    let kernel = KernelService;
    let original = kernel
        .create_project(&common::government_request(&path))
        .unwrap();
    let segment = |side: usize| {
        original
            .segments
            .iter()
            .find(|s| s.document_id == original.documents[side].document_id)
            .unwrap()
    };
    let source = segment(0);
    let target = segment(1);
    let hash = |s: &str| {
        Sha256::digest(s.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let range = |s: &Segment| TextRange {
        segment_id: s.segment_id,
        start_utf8: 0,
        end_utf8: s.content.chars().next().unwrap().len_utf8() as u64,
    };
    let record = ResearchRecord {
        context_fingerprint: kernel.research_context_fingerprint(&original, source.segment_id),
        record_id: CommandId::new().to_string(),
        run_id: CommandId::new().to_string(),
        occurrence_id: CommandId::new().to_string(),
        input_revision_id: original.project.current_revision_id,
        source_ranges: vec![range(source)],
        content_hashes: BTreeMap::from([
            (source.segment_id, hash(&source.content)),
            (target.segment_id, hash(&target.content)),
        ]),
        judgement: ResearchJudgement {
            kind: JudgementKind::Translation,
            group_name: "译法".into(),
            strategy: "策略".into(),
            target_ranges: vec![range(target)],
        },
    };
    let mut invalid = record.clone();
    invalid.source_ranges[0].end_utf8 = 1;
    assert!(kernel.confirm_research(&path, &original, invalid).is_err());
    let mut stale = record.clone();
    stale
        .content_hashes
        .insert(source.segment_id, "0".repeat(64));
    assert!(kernel.confirm_research(&path, &original, stale).is_err());
    let confirmed = kernel
        .confirm_research(&path, &original, record.clone())
        .unwrap();
    assert_eq!(confirmed.contract_version, "1.1");
    assert_eq!(
        kernel.open_project(&path).unwrap().research_records,
        vec![record.clone()]
    );
    let undone = kernel.undo(&path, &confirmed).unwrap();
    assert!(undone.research_records.is_empty());
    assert_eq!(undone.contract_version, "1.1");
    let redone = kernel.redo(&path, &undone).unwrap();
    assert_eq!(redone.research_records, vec![record.clone()]);
    let merged = kernel
        .merge_research_groups(
            &path,
            &redone,
            ResearchGroupMergeRequest {
                run_id: record.run_id.clone(),
                from_group: "译法".into(),
                to_group: "归并后".into(),
            },
        )
        .unwrap();
    assert_eq!(merged.research_records[0].judgement.group_name, "归并后");
    assert_eq!(
        merged.research_records[0].judgement.target_ranges,
        record.judgement.target_ranges
    );
    let restored = kernel
        .restore_revision(&path, &merged, original.project.current_revision_id)
        .unwrap();
    assert!(restored.research_records.is_empty());
    assert_eq!(restored.contract_version, "1.1");
    assert!(restored.revisions.len() > merged.revisions.len());
    let mut duplicate = confirmed;
    duplicate.research_records.push(record);
    assert!(validate_snapshot(&duplicate).is_err());
}
