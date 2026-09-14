//! Bounded body queries over a revision-bound, text-free structure projection.
use crate::{KernelError, KernelService};
use jueming_protocol::{
    CONTRACT_VERSION, ParallelSlice, ParallelSliceRequest, ProjectSnapshot, SegmentDescriptor,
    SegmentText, WorkspaceProject,
};
use std::collections::HashSet;

impl KernelService {
    pub fn workspace_project(
        &self,
        snapshot: &ProjectSnapshot,
    ) -> Result<WorkspaceProject, KernelError> {
        Ok(WorkspaceProject {
            contract_version: CONTRACT_VERSION.into(),
            project: snapshot.project.clone(),
            documents: snapshot.documents.clone(),
            segments: snapshot
                .segments
                .iter()
                .map(|s| SegmentDescriptor {
                    segment_id: s.segment_id,
                    document_id: s.document_id,
                    content_hash: s.content_hash.clone(),
                    content_length: s.content.encode_utf16().count(),
                })
                .collect(),
            segment_orders: snapshot.segment_orders.clone(),
            alignments: snapshot.alignments.clone(),
            revisions: snapshot.revisions.clone(),
            bookmarks: snapshot.bookmarks.clone(),
            annotations: snapshot.annotations.clone(),
            summary: self.summarize(snapshot)?,
        })
    }

    pub fn load_parallel_slice(
        &self,
        snapshot: &ProjectSnapshot,
        request: &ParallelSliceRequest,
    ) -> Result<ParallelSlice, KernelError> {
        if request.project_id != snapshot.project.project_id {
            return Err(KernelError::ProjectMismatch);
        }
        if request.revision_id != snapshot.project.current_revision_id {
            return Err(KernelError::StaleRevision {
                expected: snapshot.project.current_revision_id,
                provided: request.revision_id,
            });
        }
        if request.source_document_id != snapshot.documents[0].document_id
            || !snapshot
                .documents
                .iter()
                .skip(1)
                .any(|document| document.document_id == request.target_document_id)
            || request.halo > 50
            || request.segment_ids.len() > 200
        {
            return Err(KernelError::InvalidSnapshot(
                "Invalid slice range; at most 200 segment IDs or halo 50 are allowed".into(),
            ));
        }
        let mut ids: HashSet<_> = request.segment_ids.iter().copied().collect();
        if ids.is_empty() {
            let anchor = request.anchor_segment_id.or_else(|| {
                snapshot
                    .alignments
                    .iter()
                    .find(|a| Some(a.alignment_id) == request.anchor_alignment_id)
                    .and_then(|a| {
                        a.source_segment_ids
                            .first()
                            .or(a.target_segment_ids.first())
                            .copied()
                    })
            });
            let anchor = anchor.ok_or_else(|| {
                KernelError::InvalidSnapshot(
                    "A slice requires a stable anchor or segment IDs".into(),
                )
            })?;
            if !snapshot.segments.iter().any(|s| s.segment_id == anchor) {
                return Err(KernelError::InvalidSnapshot(
                    "Slice anchor is unknown".into(),
                ));
            }
            let relation = snapshot.alignments.iter().find(|a| {
                a.source_segment_ids.contains(&anchor) || a.target_segment_ids.contains(&anchor)
            });
            for order in &snapshot.segment_orders {
                let index = order.entries.iter().position(|entry| {
                    entry.segment_id == anchor
                        || relation.is_some_and(|a| {
                            a.source_segment_ids.contains(&entry.segment_id)
                                || a.target_segment_ids.contains(&entry.segment_id)
                        })
                });
                if let Some(index) = index {
                    let start = index.saturating_sub(request.halo as usize);
                    let end = (index + request.halo as usize + 1).min(order.entries.len());
                    ids.extend(
                        order.entries[start..end]
                            .iter()
                            .map(|entry| entry.segment_id),
                    );
                }
            }
        }
        let segments: Vec<_> = snapshot
            .segments
            .iter()
            .filter(|s| ids.contains(&s.segment_id))
            .map(|s| SegmentText {
                segment_id: s.segment_id,
                content: s.content.clone(),
                content_hash: s.content_hash.clone(),
            })
            .collect();
        if segments.len() != ids.len() {
            return Err(KernelError::InvalidSnapshot(
                "Slice contains unknown segment IDs".into(),
            ));
        }
        Ok(ParallelSlice {
            project_id: request.project_id,
            revision_id: request.revision_id,
            segments,
        })
    }
}
