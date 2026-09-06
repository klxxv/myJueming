//! Append-only revision records; command orchestration lives in the Kernel.

use crate::ids::{OperationId, ProjectId, RevisionId};
use crate::project::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangeSetSummary {
    pub operation: String,
    pub affected_count: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionState {
    Complete,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    pub revision_id: RevisionId,
    pub project_id: ProjectId,
    pub parent_revision_id: Option<RevisionId>,
    pub operation_id: OperationId,
    pub change_set: ChangeSetSummary,
    pub author_label: String,
    pub created_at: Timestamp,
    pub summary: String,
    pub state: RevisionState,
}
