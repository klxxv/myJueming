//! History queries, structured diffs and append-only undo/redo/restore.

use crate::persistence::{persist_snapshot, read_revision_snapshot};
use crate::revision::{advance_revision_with_operation, next_revision_id};
use crate::validation::validate_snapshot;
use crate::{KernelError, KernelService};
use jueming_core::{Revision, RevisionId};
use jueming_protocol::{
    AlignmentRevisionDiff, OrderRevisionDiff, ProjectSnapshot, RevisionComparison,
    RevisionListResponse, SegmentRevisionDiff,
};
use jueming_storage::ProjectLayout;
use std::collections::{HashMap, HashSet};
use std::path::Path;

impl KernelService {
    pub fn list_revisions(
        &self,
        project_path: impl AsRef<Path>,
    ) -> Result<RevisionListResponse, KernelError> {
        let snapshot = self.open_project(project_path)?;
        Ok(RevisionListResponse {
            project_id: snapshot.project.project_id,
            current_revision_id: snapshot.project.current_revision_id,
            revisions: snapshot.revisions,
        })
    }

    pub fn compare_revision(
        &self,
        project_path: impl AsRef<Path>,
        from_revision_id: RevisionId,
        to_revision_id: RevisionId,
    ) -> Result<RevisionComparison, KernelError> {
        let layout = ProjectLayout::new(project_path.as_ref())?;
        let current = layout.read_snapshot::<ProjectSnapshot>()?;
        validate_snapshot(&current)?;
        let from = read_revision_snapshot(&layout, &current, from_revision_id)?;
        let to = read_revision_snapshot(&layout, &current, to_revision_id)?;
        if from.project.project_id != to.project.project_id {
            return Err(KernelError::RevisionProjectMismatch);
        }
        Ok(compare_snapshots(&from, &to))
    }

    pub fn undo(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let current_id = snapshot.project.current_revision_id;
        let current_revision = revision(snapshot, current_id)?;
        let target_id = match parse_operation(&current_revision.change_set.operation) {
            Some(("undo", _from, to)) => revision(snapshot, to)?
                .parent_revision_id
                .ok_or(KernelError::NoUndoAvailable)?,
            Some(("redo", from, _)) => from,
            _ => current_revision
                .parent_revision_id
                .ok_or(KernelError::NoUndoAvailable)?,
        };
        let layout = ProjectLayout::new(project_path.as_ref())?;
        let target = read_revision_snapshot(&layout, snapshot, target_id)?;
        self.commit_history_transition(
            &layout,
            snapshot,
            target,
            format!("undo:{current_id}:{target_id}"),
            format!("Undo to revision {target_id}"),
        )
    }

    pub fn redo(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let current_id = snapshot.project.current_revision_id;
        let current_revision = revision(snapshot, current_id)?;
        let (_, from, target_id) = parse_operation(&current_revision.change_set.operation)
            .filter(|(operation, _, _)| *operation == "undo" || *operation == "redo")
            .ok_or(KernelError::NoRedoAvailable)?;
        let target_id = if current_revision.change_set.operation.starts_with("undo:") {
            from
        } else {
            let target_revision = revision(snapshot, target_id)?;
            match parse_operation(&target_revision.change_set.operation) {
                Some(("undo", source, _)) => source,
                _ => return Err(KernelError::NoRedoAvailable),
            }
        };
        let layout = ProjectLayout::new(project_path.as_ref())?;
        let target = read_revision_snapshot(&layout, snapshot, target_id)?;
        self.commit_history_transition(
            &layout,
            snapshot,
            target,
            format!("redo:{current_id}:{target_id}"),
            format!("Redo to revision {target_id}"),
        )
    }

    pub fn restore_revision(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        target_revision_id: RevisionId,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let layout = ProjectLayout::new(project_path.as_ref())?;
        let target = read_revision_snapshot(&layout, snapshot, target_revision_id)?;
        self.commit_history_transition(
            &layout,
            snapshot,
            target,
            format!("restore:{target_revision_id}"),
            format!("Restored revision {target_revision_id}"),
        )
    }

    fn commit_history_transition(
        &self,
        layout: &ProjectLayout,
        current: &ProjectSnapshot,
        mut target: ProjectSnapshot,
        operation: String,
        summary: String,
    ) -> Result<ProjectSnapshot, KernelError> {
        if target.project.project_id != current.project.project_id {
            return Err(KernelError::RevisionProjectMismatch);
        }
        target.revisions = current.revisions.clone();
        let revision_id = next_revision_id(current);
        advance_revision_with_operation(&mut target, operation, 1, summary, revision_id);
        validate_snapshot(&target)?;
        persist_snapshot(layout, &target)?;
        Ok(target)
    }
}

fn revision(snapshot: &ProjectSnapshot, revision_id: RevisionId) -> Result<&Revision, KernelError> {
    snapshot
        .revisions
        .iter()
        .find(|revision| revision.revision_id == revision_id)
        .ok_or(KernelError::RevisionNotFound(revision_id))
}

fn parse_operation(operation: &str) -> Option<(&str, RevisionId, RevisionId)> {
    let mut parts = operation.split(':');
    let kind = parts.next()?;
    let first = parts.next()?.parse::<u64>().ok()?;
    let second = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((kind, RevisionId::new(first), RevisionId::new(second)))
}

fn compare_snapshots(from: &ProjectSnapshot, to: &ProjectSnapshot) -> RevisionComparison {
    let before_segments: HashMap<_, _> = from
        .segments
        .iter()
        .map(|segment| (segment.segment_id, segment))
        .collect();
    let after_segments: HashMap<_, _> = to
        .segments
        .iter()
        .map(|segment| (segment.segment_id, segment))
        .collect();
    let mut segment_ids: HashSet<_> = before_segments.keys().copied().collect();
    segment_ids.extend(after_segments.keys().copied());
    let mut segment_changes = segment_ids
        .into_iter()
        .filter_map(|id| {
            let before = before_segments.get(&id).copied();
            let after = after_segments.get(&id).copied();
            (before != after).then(|| SegmentRevisionDiff {
                segment_id: id,
                before: before.cloned(),
                after: after.cloned(),
            })
        })
        .collect::<Vec<_>>();
    segment_changes.sort_by_key(|diff| diff.segment_id);

    let before_orders: HashMap<_, _> = from
        .segment_orders
        .iter()
        .map(|order| (order.document_id, order))
        .collect();
    let after_orders: HashMap<_, _> = to
        .segment_orders
        .iter()
        .map(|order| (order.document_id, order))
        .collect();
    let mut document_ids: HashSet<_> = before_orders.keys().copied().collect();
    document_ids.extend(after_orders.keys().copied());
    let mut order_changes = document_ids
        .into_iter()
        .filter_map(|id| {
            let before = before_orders.get(&id).copied();
            let after = after_orders.get(&id).copied();
            (before != after).then(|| OrderRevisionDiff {
                document_id: id,
                before: before.cloned(),
                after: after.cloned(),
            })
        })
        .collect::<Vec<_>>();
    order_changes.sort_by_key(|diff| diff.document_id);

    let before_alignments: HashMap<_, _> = from
        .alignments
        .iter()
        .map(|alignment| (alignment.alignment_id, alignment))
        .collect();
    let after_alignments: HashMap<_, _> = to
        .alignments
        .iter()
        .map(|alignment| (alignment.alignment_id, alignment))
        .collect();
    let mut alignment_ids: HashSet<_> = before_alignments.keys().copied().collect();
    alignment_ids.extend(after_alignments.keys().copied());
    let mut alignment_changes = alignment_ids
        .into_iter()
        .filter_map(|id| {
            let before = before_alignments.get(&id).copied();
            let after = after_alignments.get(&id).copied();
            (before != after).then(|| AlignmentRevisionDiff {
                alignment_id: id,
                before: before.cloned(),
                after: after.cloned(),
            })
        })
        .collect::<Vec<_>>();
    alignment_changes.sort_by_key(|diff| diff.alignment_id);

    RevisionComparison {
        project_id: from.project.project_id,
        from_revision_id: from.project.current_revision_id,
        to_revision_id: to.project.current_revision_id,
        segment_changes,
        order_changes,
        alignment_changes,
    }
}
