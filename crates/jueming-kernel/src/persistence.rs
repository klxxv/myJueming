//! Revision-first persistence and validated historical reads.

use crate::KernelError;
use crate::validation::validate_snapshot;
use jueming_core::RevisionId;
use jueming_protocol::ProjectSnapshot;
use jueming_storage::ProjectLayout;

pub(crate) fn persist_snapshot(
    layout: &ProjectLayout,
    snapshot: &ProjectSnapshot,
) -> Result<(), KernelError> {
    let _commit_lock = layout.acquire_commit_lock()?;
    let revision_id = snapshot.project.current_revision_id;
    if layout.snapshot_path().exists() {
        let current = layout.read_snapshot::<ProjectSnapshot>()?;
        if current == *snapshot {
            return Ok(());
        }
        if current.project.project_id != snapshot.project.project_id {
            return Err(KernelError::ProjectMismatch);
        }
        let parent = snapshot
            .revisions
            .last()
            .and_then(|revision| revision.parent_revision_id);
        if parent != Some(current.project.current_revision_id)
            || revision_id.value() != current.project.current_revision_id.value() + 1
        {
            return Err(KernelError::StaleRevision {
                expected: current.project.current_revision_id,
                provided: parent.unwrap_or(revision_id),
            });
        }
    }
    // Write the immutable revision first. If the main pointer write fails, a
    // harmless orphan remains; the previous project.json is still valid and no
    // history entry points at a partial file.
    layout.write_revision_snapshot(revision_id, snapshot)?;
    layout.write_snapshot(snapshot)?;
    Ok(())
}

pub(crate) fn read_revision_snapshot(
    layout: &ProjectLayout,
    current: &ProjectSnapshot,
    revision_id: RevisionId,
) -> Result<ProjectSnapshot, KernelError> {
    match layout.read_revision_snapshot(revision_id) {
        Ok(snapshot) => {
            validate_snapshot(&snapshot)?;
            Ok(snapshot)
        }
        Err(_error) if revision_id == current.project.current_revision_id => Ok(current.clone()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{KernelService, revision::advance_revision, test_support::government_request};
    use jueming_core::RevisionId;
    use jueming_storage::ProjectLayout;

    #[test]
    fn orphan_revision_file_is_invisible_until_the_current_snapshot_advances() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("orphan-revision.jm");
        let service = KernelService;
        let initial = service
            .create_project(&government_request(&path))
            .expect("create project");
        let layout = ProjectLayout::new(&path).expect("layout");
        let mut orphan = initial.clone();
        advance_revision(
            &mut orphan,
            "simulated_interrupted_write",
            1,
            "revision file reached disk before project.json".into(),
        );
        layout
            .write_revision_snapshot(RevisionId::new(2), &orphan)
            .expect("write the first half of persist_snapshot");

        assert_eq!(service.open_project(&path).unwrap(), initial);
        let visible = service.list_revisions(&path).expect("visible history");
        assert_eq!(visible.current_revision_id, RevisionId::new(1));
        assert_eq!(visible.revisions.len(), 1);

        let segment_id = initial.segments[0].segment_id;
        let committed = service
            .update_segment(&path, &initial, segment_id, "committed after recovery")
            .expect("overwrite orphan with a complete commit");
        assert_eq!(committed.project.current_revision_id, RevisionId::new(2));
        assert_eq!(service.open_project(&path).unwrap(), committed);
    }
}
