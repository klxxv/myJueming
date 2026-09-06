//! Shared revision advancement and timestamp generation.

use chrono::{SecondsFormat, Utc};
use jueming_core::{ChangeSetSummary, OperationId, Revision, RevisionId, RevisionState};
use jueming_protocol::ProjectSnapshot;

pub(crate) fn advance_revision(
    snapshot: &mut ProjectSnapshot,
    operation: &str,
    affected_count: u64,
    summary: String,
) -> RevisionId {
    let parent = snapshot.project.current_revision_id;
    let revision_id = RevisionId::new(parent.value() + 1);
    advance_revision_with_operation(
        snapshot,
        operation.into(),
        affected_count,
        summary,
        revision_id,
    );
    revision_id
}

pub(crate) fn advance_revision_with_operation(
    snapshot: &mut ProjectSnapshot,
    operation: String,
    affected_count: u64,
    summary: String,
    revision_id: RevisionId,
) {
    let parent = snapshot.project.current_revision_id;
    let now = timestamp();
    snapshot.project.current_revision_id = revision_id;
    snapshot.project.updated_at = now.clone();
    snapshot.revisions.push(Revision {
        revision_id,
        project_id: snapshot.project.project_id,
        parent_revision_id: Some(parent),
        operation_id: OperationId::new(),
        change_set: ChangeSetSummary {
            operation,
            affected_count,
        },
        author_label: "Local user".into(),
        created_at: now,
        summary,
        state: RevisionState::Complete,
    });
}

pub(crate) fn next_revision_id(snapshot: &ProjectSnapshot) -> RevisionId {
    RevisionId::new(snapshot.project.current_revision_id.value() + 1)
}

pub(crate) fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
