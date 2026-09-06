//! Project lifecycle and summary queries through the storage boundary.

use crate::persistence::persist_snapshot;
use crate::validation::validate_snapshot;
use crate::{KernelError, KernelService};
use jueming_protocol::{ProjectSnapshot, ProjectSummary};
use jueming_storage::ProjectLayout;
use std::collections::HashSet;
use std::path::Path;

impl KernelService {
    pub fn open_project(
        &self,
        project_path: impl AsRef<Path>,
    ) -> Result<ProjectSnapshot, KernelError> {
        let layout = ProjectLayout::new(project_path.as_ref())?;
        let snapshot = layout.read_snapshot::<ProjectSnapshot>()?;
        validate_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub fn save_project(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
    ) -> Result<(), KernelError> {
        validate_snapshot(snapshot)?;
        let layout = ProjectLayout::new(project_path.as_ref())?;
        persist_snapshot(&layout, snapshot)?;
        Ok(())
    }

    pub fn clear_cache(&self, project_path: impl AsRef<Path>) -> Result<u64, KernelError> {
        let layout = ProjectLayout::new(project_path.as_ref())?;
        layout.clear_cache().map_err(KernelError::from)
    }

    pub fn summarize(&self, snapshot: &ProjectSnapshot) -> Result<ProjectSummary, KernelError> {
        validate_snapshot(snapshot)?;
        let source_document = &snapshot.documents[0];
        let target_document = &snapshot.documents[1];
        let source_count = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == source_document.document_id)
            .count() as u64;
        let target_count = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == target_document.document_id)
            .count() as u64;
        let source_linked: HashSet<_> = snapshot
            .alignments
            .iter()
            .flat_map(|alignment| alignment.source_segment_ids.iter().copied())
            .collect();
        let target_linked: HashSet<_> = snapshot
            .alignments
            .iter()
            .flat_map(|alignment| alignment.target_segment_ids.iter().copied())
            .collect();
        Ok(ProjectSummary {
            project_id: snapshot.project.project_id,
            name: snapshot.project.name.clone(),
            source_label: snapshot
                .source_assets
                .iter()
                .find(|asset| asset.asset_id == source_document.source_asset_id)
                .map(|asset| asset.label.clone())
                .unwrap_or_else(|| source_document.title.clone()),
            target_label: snapshot
                .source_assets
                .iter()
                .find(|asset| asset.asset_id == target_document.source_asset_id)
                .map(|asset| asset.label.clone())
                .unwrap_or_else(|| target_document.title.clone()),
            source_count,
            target_count,
            alignment_count: snapshot.alignments.len() as u64,
            source_unlinked_count: source_count - source_linked.len() as u64,
            target_unlinked_count: target_count - target_linked.len() as u64,
            revision_id: snapshot.project.current_revision_id,
        })
    }
}
