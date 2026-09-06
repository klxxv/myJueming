//! Manual/provisional relations and cardinality validation.

use crate::error::CoreError;
use crate::ids::{AlignmentId, ProjectId, RevisionId, SegmentId};
use crate::project::Document;
use crate::segment::Segment;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentProducer {
    Provisional,
    Manual,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Cardinality {
    #[serde(rename = "1:1")]
    OneToOne,
    #[serde(rename = "1:n")]
    OneToMany,
    #[serde(rename = "n:1")]
    ManyToOne,
    #[serde(rename = "n:m")]
    ManyToMany,
}
impl Cardinality {
    pub fn of(source: usize, target: usize) -> Result<Self, CoreError> {
        match (source, target) {
            (0, _) | (_, 0) => Err(CoreError::EmptyAlignment),
            (1, 1) => Ok(Self::OneToOne),
            (1, _) => Ok(Self::OneToMany),
            (_, 1) => Ok(Self::ManyToOne),
            _ => Ok(Self::ManyToMany),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Alignment {
    pub alignment_id: AlignmentId,
    pub project_id: ProjectId,
    pub source_segment_ids: Vec<SegmentId>,
    pub target_segment_ids: Vec<SegmentId>,
    pub producer: AlignmentProducer,
    pub cardinality: Cardinality,
    pub created_revision_id: RevisionId,
    pub updated_revision_id: RevisionId,
}
impl Alignment {
    pub fn new(
        project_id: ProjectId,
        source_segment_ids: Vec<SegmentId>,
        target_segment_ids: Vec<SegmentId>,
        revision: RevisionId,
    ) -> Result<Self, CoreError> {
        let cardinality = Cardinality::of(source_segment_ids.len(), target_segment_ids.len())?;
        if has_duplicates(&source_segment_ids) || has_duplicates(&target_segment_ids) {
            return Err(CoreError::DuplicateAlignmentReference);
        }
        Ok(Self {
            alignment_id: AlignmentId::new(),
            project_id,
            source_segment_ids,
            target_segment_ids,
            producer: AlignmentProducer::Manual,
            cardinality,
            created_revision_id: revision,
            updated_revision_id: revision,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InitialLayout {
    pub alignments: Vec<Alignment>,
    pub source_unlinked: Vec<SegmentId>,
    pub target_unlinked: Vec<SegmentId>,
}
pub fn build_provisional_layout(
    project_id: ProjectId,
    source: &[Segment],
    target: &[Segment],
    revision: RevisionId,
) -> Result<InitialLayout, CoreError> {
    let count = source.len().min(target.len());
    let mut alignments = Vec::with_capacity(count);
    for index in 0..count {
        let mut alignment = Alignment::new(
            project_id,
            vec![source[index].segment_id],
            vec![target[index].segment_id],
            revision,
        )?;
        alignment.producer = AlignmentProducer::Provisional;
        alignments.push(alignment);
    }
    Ok(InitialLayout {
        alignments,
        source_unlinked: source[count..].iter().map(|s| s.segment_id).collect(),
        target_unlinked: target[count..].iter().map(|s| s.segment_id).collect(),
    })
}
pub fn provisional_alignment_layout(
    project_id: ProjectId,
    source: &[Segment],
    target: &[Segment],
    revision: RevisionId,
) -> Result<InitialLayout, CoreError> {
    build_provisional_layout(project_id, source, target, revision)
}
fn has_duplicates(values: &[SegmentId]) -> bool {
    let mut seen = HashSet::new();
    values.iter().any(|value| !seen.insert(*value))
}

pub fn validate_alignment(
    alignment: &Alignment,
    project_id: ProjectId,
    source_document: &Document,
    target_document: &Document,
    segments: &HashMap<SegmentId, Segment>,
) -> Result<(), CoreError> {
    if alignment.project_id != project_id
        || alignment.cardinality
            != Cardinality::of(
                alignment.source_segment_ids.len(),
                alignment.target_segment_ids.len(),
            )?
        || has_duplicates(&alignment.source_segment_ids)
        || has_duplicates(&alignment.target_segment_ids)
    {
        return Err(CoreError::InvalidAlignmentReference);
    }
    for id in &alignment.source_segment_ids {
        if segments
            .get(id)
            .map(|s| s.document_id != source_document.document_id)
            .unwrap_or(true)
        {
            return Err(CoreError::InvalidAlignmentReference);
        }
    }
    for id in &alignment.target_segment_ids {
        if segments
            .get(id)
            .map(|s| s.document_id != target_document.document_id)
            .unwrap_or(true)
        {
            return Err(CoreError::InvalidAlignmentReference);
        }
    }
    Ok(())
}
