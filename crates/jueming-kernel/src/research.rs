//! Human research facts share canonical revision and command transaction semantics.
use crate::{KernelError, KernelService, revision::advance_revision, validate_snapshot};
use jueming_protocol::{JudgementKind, ProjectSnapshot, ResearchRecord};
use sha2::{Digest, Sha256};
use std::path::Path;

impl KernelService {
    /// Includes all n:m context members, their original contents and relation identities.
    pub fn research_context_fingerprint(
        &self,
        snapshot: &ProjectSnapshot,
        source: jueming_protocol::SegmentId,
    ) -> String {
        let primary = crate::projection::alignment_ids_for_document(
            snapshot,
            snapshot.documents[1].document_id,
        );
        let contexts: Vec<_> = snapshot
            .alignments
            .iter()
            .filter(|a| a.source_segment_ids.contains(&source) && primary.contains(&a.alignment_id))
            .map(|alignment| {
                let mut ids: Vec<_> = alignment
                    .source_segment_ids
                    .iter()
                    .chain(&alignment.target_segment_ids)
                    .copied()
                    .collect();
                ids.sort();
                let contents: Vec<_> = ids
                    .iter()
                    .map(|id| {
                        (
                            id,
                            snapshot
                                .segments
                                .iter()
                                .find(|s| s.segment_id == *id)
                                .map(|s| &s.content),
                        )
                    })
                    .collect();
                serde_json::json!({"alignment":alignment,"contents":contents})
            })
            .collect();
        let bytes = serde_json::to_vec(&contexts).expect("research context serializes");
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    pub fn merge_research_groups(
        &self,
        path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        request: jueming_protocol::ResearchGroupMergeRequest,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        if request.from_group == request.to_group
            || request.to_group.trim().is_empty()
            || request.to_group.len() > 512
        {
            return Err(KernelError::InvalidSnapshot(
                "invalid research group merge".into(),
            ));
        }
        let mut next = snapshot.clone();
        let mut count = 0;
        let source_kinds: Vec<_> = snapshot
            .research_records
            .iter()
            .filter(|r| r.run_id == request.run_id && r.judgement.group_name == request.from_group)
            .map(|r| r.judgement.kind)
            .collect();
        if snapshot.research_records.iter().any(|r| {
            r.run_id == request.run_id
                && r.judgement.group_name == request.to_group
                && !source_kinds.contains(&r.judgement.kind)
        }) {
            return Err(KernelError::InvalidSnapshot(
                "translation, paraphrase and omission groups cannot be merged across kinds".into(),
            ));
        }
        for record in &mut next.research_records {
            if record.run_id == request.run_id && record.judgement.group_name == request.from_group
            {
                record.judgement.group_name = request.to_group.clone();
                count += 1;
            }
        }
        if count == 0 {
            return Err(KernelError::InvalidSnapshot(
                "research group is empty".into(),
            ));
        }
        advance_revision(
            &mut next,
            "merge_research_groups",
            count,
            format!("Merged {} into {}", request.from_group, request.to_group),
        );
        self.save_project(path, &next)?;
        Ok(next)
    }
    pub fn confirm_research(
        &self,
        path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        record: ResearchRecord,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let invalid = |message: &str| KernelError::InvalidSnapshot(message.into());
        if record.source_ranges.is_empty()
            || record.record_id.is_empty()
            || record.run_id.is_empty()
            || record.occurrence_id.is_empty()
        {
            return Err(invalid("missing research identity/source"));
        }
        if record.input_revision_id.value() > snapshot.project.current_revision_id.value() {
            return Err(invalid("future research input revision"));
        }
        if record.context_fingerprint
            != self.research_context_fingerprint(snapshot, record.source_ranges[0].segment_id)
        {
            return Err(invalid(
                "research alignment context changed; rerun before confirming",
            ));
        }
        if record.judgement.group_name.len() > 512 || record.judgement.strategy.len() > 512 {
            return Err(invalid("research label too long"));
        }
        match record.judgement.kind {
            JudgementKind::Omission if !record.judgement.target_ranges.is_empty() => {
                return Err(invalid("omission cannot have target ranges"));
            }
            JudgementKind::Translation if record.judgement.target_ranges.is_empty() => {
                return Err(invalid("translation requires target evidence"));
            }
            _ => {}
        }
        let source_document = snapshot.documents[0].document_id;
        let target_document = snapshot.documents[1].document_id;
        for (ranges, document) in [
            (&record.source_ranges, source_document),
            (&record.judgement.target_ranges, target_document),
        ] {
            for range in ranges {
                let segment = snapshot
                    .segments
                    .iter()
                    .find(|s| s.segment_id == range.segment_id)
                    .ok_or_else(|| invalid("research anchor no longer exists"))?;
                let hash: String = Sha256::digest(segment.content.as_bytes())
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                if segment.document_id != document
                    || record.content_hashes.get(&range.segment_id) != Some(&hash)
                {
                    return Err(invalid(
                        "research input changed; refresh evidence before confirming",
                    ));
                }
                let start =
                    usize::try_from(range.start_utf8).map_err(|_| invalid("invalid offset"))?;
                let end = usize::try_from(range.end_utf8).map_err(|_| invalid("invalid offset"))?;
                if start >= end || segment.content.get(start..end).is_none() {
                    return Err(invalid("invalid original UTF-8 range"));
                }
            }
        }
        let mut next = snapshot.clone();
        if next.contract_version != "2.0" {
            next.contract_version = "1.1".into();
        }
        next.research_records
            .retain(|r| r.run_id != record.run_id || r.occurrence_id != record.occurrence_id);
        next.research_records.push(record);
        advance_revision(
            &mut next,
            "confirm_research",
            1,
            "Confirmed translation evidence".into(),
        );
        self.save_project(path, &next)?;
        Ok(next)
    }
}
