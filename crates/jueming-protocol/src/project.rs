//! Project DTOs for the versioned Kernel boundary.

use crate::import::SourceAssetRecord;
use crate::sidecar::{Bookmark, HumanAnnotation};
use jueming_core::{
    Alignment, Document, ImportProfile, Project, ProjectId, Revision, RevisionId, Segment,
    SegmentOrder,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub research_records: Vec<crate::ResearchRecord>,
    /// Durable command receipts, committed atomically with canonical history.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_receipts: Vec<CommandReceipt>,
    pub contract_version: String,
    pub project: Project,
    pub documents: Vec<Document>,
    pub source_assets: Vec<SourceAssetRecord>,
    pub segments: Vec<Segment>,
    pub segment_orders: Vec<SegmentOrder>,
    pub alignments: Vec<Alignment>,
    pub revisions: Vec<Revision>,
    pub source_profile: ImportProfile,
    pub target_profile: ImportProfile,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub annotations: Vec<HumanAnnotation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub command_id: jueming_core::CommandId,
    pub fingerprint: String,
    pub committed_revision_id: RevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_id: ProjectId,
    pub name: String,
    pub source_label: String,
    pub target_label: String,
    pub source_count: u64,
    pub target_count: u64,
    pub alignment_count: u64,
    pub source_unlinked_count: u64,
    pub target_unlinked_count: u64,
    pub revision_id: RevisionId,
}
