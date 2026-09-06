//! Project/document ownership, independent of physical storage.

use crate::error::CoreError;
use crate::ids::{AssetId, DocumentId, ProjectId, RevisionId, SegmentOrderId};
use crate::revision::Revision;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub type LanguageId = String;
pub type Timestamp = String;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub project_id: ProjectId,
    pub name: String,
    pub source_language: LanguageId,
    pub target_language: LanguageId,
    pub document_ids: Vec<DocumentId>,
    pub current_revision_id: RevisionId,
    pub format_version: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Project {
    pub fn new(
        name: impl Into<String>,
        source_language: impl Into<String>,
        target_language: impl Into<String>,
        revision: RevisionId,
        now: impl Into<String>,
    ) -> Self {
        let now = now.into();
        Self {
            project_id: ProjectId::new(),
            name: name.into(),
            source_language: source_language.into(),
            target_language: target_language.into(),
            document_ids: Vec::new(),
            current_revision_id: revision,
            format_version: "1.0".into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub document_id: DocumentId,
    pub project_id: ProjectId,
    pub language_id: LanguageId,
    pub title: String,
    pub source_asset_id: AssetId,
    pub segment_order_id: SegmentOrderId,
    pub created_revision_id: RevisionId,
}

impl Document {
    pub fn new(
        project_id: ProjectId,
        language_id: impl Into<String>,
        title: impl Into<String>,
        revision: RevisionId,
    ) -> Self {
        Self {
            document_id: DocumentId::new(),
            project_id,
            language_id: language_id.into(),
            title: title.into(),
            source_asset_id: AssetId::new(),
            segment_order_id: SegmentOrderId::new(),
            created_revision_id: revision,
        }
    }
}

/// Validate the project directory references without knowing its physical store.
pub fn validate_project(
    project: &Project,
    documents: &[Document],
    revisions: &[Revision],
) -> Result<(), CoreError> {
    let document_ids: HashSet<_> = documents
        .iter()
        .map(|document| document.document_id)
        .collect();
    if document_ids.len() != documents.len()
        || project.document_ids.len() != documents.len()
        || project.document_ids.iter().collect::<HashSet<_>>()
            != document_ids.iter().collect::<HashSet<_>>()
        || documents
            .iter()
            .any(|document| document.project_id != project.project_id)
        || !revisions.iter().any(|revision| {
            revision.project_id == project.project_id
                && revision.revision_id == project.current_revision_id
        })
    {
        return Err(CoreError::InvalidProject);
    }
    Ok(())
}
