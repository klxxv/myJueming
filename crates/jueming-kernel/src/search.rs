//! Revision-bound search and atomic replacement.

use crate::projection::alignment_by_segment;
use crate::revision::{advance_revision, next_revision_id};
use crate::validation::{ensure_revision, validate_snapshot};
use crate::{KernelError, KernelService};
use jueming_core::OperationId;
use jueming_protocol::{
    ProjectSnapshot, ReplaceApplyRequest, ReplacePreviewItem, ReplacePreviewRequest,
    ReplacePreviewResponse, SearchSegmentsRequest, SearchSegmentsResponse, SegmentSearchHit,
};
use regex::RegexBuilder;
use std::collections::HashSet;
use std::path::Path;

impl KernelService {
    pub fn search_segments(
        &self,
        snapshot: &ProjectSnapshot,
        request: &SearchSegmentsRequest,
    ) -> Result<SearchSegmentsResponse, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_revision(snapshot, request.project_id, request.base_revision_id)?;
        let matcher = SearchMatcher::new(&request.query, request.regex, request.case_sensitive)?;
        let alignment_by_segment = alignment_by_segment(snapshot);
        let hits = snapshot
            .segments
            .iter()
            .filter(|segment| {
                request
                    .document_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&segment.document_id))
            })
            .filter(|segment| {
                request.language_id.as_ref().is_none_or(|language| {
                    snapshot.documents.iter().any(|document| {
                        document.document_id == segment.document_id
                            && &document.language_id == language
                    })
                })
            })
            .filter(|segment| matcher.is_match(&segment.content))
            .map(|segment| SegmentSearchHit {
                segment_id: segment.segment_id,
                document_id: segment.document_id,
                language_id: snapshot
                    .documents
                    .iter()
                    .find(|document| document.document_id == segment.document_id)
                    .map(|document| document.language_id.clone())
                    .unwrap_or_default(),
                content: segment.content.clone(),
                alignment_id: alignment_by_segment.get(&segment.segment_id).copied(),
                revision_id: snapshot.project.current_revision_id,
            })
            .collect();
        Ok(SearchSegmentsResponse {
            revision_id: snapshot.project.current_revision_id,
            hits,
        })
    }

    pub fn preview_replace(
        &self,
        snapshot: &ProjectSnapshot,
        request: &ReplacePreviewRequest,
    ) -> Result<ReplacePreviewResponse, KernelError> {
        let hits = self.search_segments(
            snapshot,
            &SearchSegmentsRequest {
                document_ids: request.document_ids.clone(),
                project_id: request.project_id,
                query: request.query.clone(),
                regex: request.regex,
                case_sensitive: request.case_sensitive,
                language_id: request.language_id.clone(),
                base_revision_id: request.base_revision_id,
            },
        )?;
        let matcher = SearchMatcher::new(&request.query, request.regex, request.case_sensitive)?;
        let items = hits
            .hits
            .iter()
            .map(|hit| ReplacePreviewItem {
                segment_id: hit.segment_id,
                before: hit.content.clone(),
                after: matcher.replace(&hit.content, &request.replacement),
            })
            .collect();
        let selected_segment_ids = hits.hits.iter().map(|hit| hit.segment_id).collect();
        Ok(ReplacePreviewResponse {
            base_revision_id: request.base_revision_id,
            selected_segment_ids,
            items,
        })
    }

    pub fn apply_replace(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        request: &ReplaceApplyRequest,
    ) -> Result<ProjectSnapshot, KernelError> {
        self.apply_replace_with_operation_id(project_path, snapshot, request, OperationId::new())
    }

    /// Applies a reviewed replacement while preserving a caller-preallocated
    /// canonical operation ID. The ID is written into the Revision before the
    /// snapshot is persisted, so a durable application journal can reconcile a
    /// crash without guessing from revision ordering or operation text.
    pub fn apply_replace_with_operation_id(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        request: &ReplaceApplyRequest,
        operation_id: OperationId,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_revision(
            snapshot,
            request.preview.project_id,
            request.preview.base_revision_id,
        )?;
        let preview = self.preview_replace(snapshot, &request.preview)?;
        let selected: HashSet<_> = if request.selected_segment_ids.is_empty() {
            preview.selected_segment_ids.iter().copied().collect()
        } else {
            request.selected_segment_ids.iter().copied().collect()
        };
        if selected
            .iter()
            .any(|id| !preview.selected_segment_ids.contains(id))
        {
            return Err(KernelError::ReplaceSelectionMismatch);
        }
        if selected.is_empty() {
            return Ok(snapshot.clone());
        }
        let matcher = SearchMatcher::new(
            &request.preview.query,
            request.preview.regex,
            request.preview.case_sensitive,
        )?;
        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        for segment in &mut next.segments {
            if selected.contains(&segment.segment_id) {
                let replacement = matcher.replace(&segment.content, &request.preview.replacement);
                segment.update_content(replacement, revision_id);
            }
        }
        advance_revision(
            &mut next,
            "replace_segments",
            selected.len() as u64,
            format!("Replaced text in {} segments", selected.len()),
        );
        next.revisions
            .last_mut()
            .expect("advance_revision always appends a Revision")
            .operation_id = operation_id;
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }
}

struct SearchMatcher {
    regex: regex::Regex,
}

impl SearchMatcher {
    fn new(query: &str, regex: bool, case_sensitive: bool) -> Result<Self, KernelError> {
        let pattern = if regex {
            query.to_owned()
        } else {
            regex::escape(query)
        };
        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|error| KernelError::InvalidSearchRegex(error.to_string()))?;
        Ok(Self { regex })
    }
    fn is_match(&self, value: &str) -> bool {
        self.regex.is_match(value)
    }
    fn replace(&self, value: &str, replacement: &str) -> String {
        self.regex.replace_all(value, replacement).into_owned()
    }
}
