//! Human-annotation sidecar commands and queries.

use crate::revision::{advance_revision, timestamp};
use crate::validation::{ensure_annotation_anchors, validate_snapshot};
use crate::{KernelError, KernelService};
use jueming_protocol::{
    AnnotationCreateRequest, AnnotationStatus, AnnotationUpdateRequest, HumanAnnotation,
    ProjectSnapshot,
};
use std::path::Path;

impl KernelService {
    pub fn create_annotation(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        request: AnnotationCreateRequest,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_annotation_anchors(snapshot, &request.linked_segment_ids, request.alignment_id)?;
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "create_annotation",
            1,
            format!("Created annotation {}", request.title),
        );
        let now = timestamp();
        next.annotations.push(HumanAnnotation {
            annotation_id: jueming_core::AnnotationId::new(),
            project_id: snapshot.project.project_id,
            title: request.title,
            body: request.body,
            status: request.status,
            linked_segment_ids: request.linked_segment_ids,
            alignment_id: request.alignment_id,
            local_author_label: request.local_author_label,
            created_at: now.clone(),
            updated_at: now,
        });
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn list_annotations(
        &self,
        snapshot: &ProjectSnapshot,
    ) -> Result<Vec<HumanAnnotation>, KernelError> {
        validate_snapshot(snapshot)?;
        Ok(snapshot.annotations.clone())
    }

    pub fn update_annotation(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        request: AnnotationUpdateRequest,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_annotation_anchors(snapshot, &request.linked_segment_ids, request.alignment_id)?;
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "update_annotation",
            1,
            format!("Updated annotation {}", request.annotation_id),
        );
        let annotation = next
            .annotations
            .iter_mut()
            .find(|annotation| annotation.annotation_id == request.annotation_id)
            .ok_or(KernelError::AnnotationNotFound(request.annotation_id))?;
        annotation.title = request.title;
        annotation.body = request.body;
        annotation.status = request.status;
        annotation.linked_segment_ids = request.linked_segment_ids;
        annotation.alignment_id = request.alignment_id;
        annotation.updated_at = timestamp();
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn delete_annotation(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        annotation_id: jueming_core::AnnotationId,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let mut next = snapshot.clone();
        let old = next.annotations.len();
        next.annotations
            .retain(|annotation| annotation.annotation_id != annotation_id);
        if old == next.annotations.len() {
            return Err(KernelError::AnnotationNotFound(annotation_id));
        }
        advance_revision(
            &mut next,
            "delete_annotation",
            1,
            format!("Deleted annotation {annotation_id}"),
        );
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn resolve_annotation(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        annotation_id: jueming_core::AnnotationId,
    ) -> Result<ProjectSnapshot, KernelError> {
        let annotation = snapshot
            .annotations
            .iter()
            .find(|annotation| annotation.annotation_id == annotation_id)
            .ok_or(KernelError::AnnotationNotFound(annotation_id))?;
        self.update_annotation(
            project_path,
            snapshot,
            AnnotationUpdateRequest {
                annotation_id,
                title: annotation.title.clone(),
                body: annotation.body.clone(),
                status: AnnotationStatus::Resolved,
                linked_segment_ids: annotation.linked_segment_ids.clone(),
                alignment_id: annotation.alignment_id,
            },
        )
    }
}
