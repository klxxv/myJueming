//! Command, query, transaction, and event orchestration.

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use jueming_core::{
    AssetId, ChangeSetSummary, ContentRef, CoreError, Document, ImportProfile, OperationId,
    OrderedSegmentRef, PositionKey, Project, Revision, RevisionId, RevisionState, Segment,
    SegmentId, SegmentOrder, build_provisional_layout, decode_bytes, segment_text,
    validate_alignment, validate_project, validate_segment_order,
};
use jueming_protocol::{
    AlignmentRevisionDiff, AnnotationCreateRequest, AnnotationStatus, AnnotationUpdateRequest,
    Bookmark, BookmarkCreateRequest, BookmarkUpdateRequest, CONTRACT_VERSION, CreateProjectRequest,
    ExportFormat, ExportRequest, HumanAnnotation, ImportPreviewResponse, OrderRevisionDiff,
    ProjectSnapshot, ProjectSummary, ReplaceApplyRequest, ReplacePreviewItem,
    ReplacePreviewRequest, ReplacePreviewResponse, RevisionComparison, RevisionListResponse,
    SearchSegmentsRequest, SearchSegmentsResponse, SegmentRevisionDiff, SegmentSearchHit,
    SourceAssetRecord, TextInput,
};
use jueming_storage::{ProjectLayout, StorageError};
use regex::RegexBuilder;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Default, Clone, Copy)]
pub struct KernelService;

impl KernelService {
    pub fn preview_import(
        &self,
        input: &TextInput,
        profile: &ImportProfile,
    ) -> Result<ImportPreviewResponse, KernelError> {
        let loaded = load_input(input, profile)?;
        let preview = segment_text(&loaded.text, profile);
        Ok(ImportPreviewResponse {
            label: loaded.label,
            profile: profile.clone(),
            had_bom: loaded.had_bom,
            sha256: loaded.sha256,
            byte_length: loaded.bytes.len() as u64,
            preview,
        })
    }

    pub fn create_project(
        &self,
        request: &CreateProjectRequest,
    ) -> Result<ProjectSnapshot, KernelError> {
        let layout = ProjectLayout::new(&request.project_path)?;
        let source = load_input(&request.source.input, &request.source.profile)?;
        let target = load_input(&request.target.input, &request.target.profile)?;
        let source_preview = segment_text(&source.text, &request.source.profile);
        let target_preview = segment_text(&target.text, &request.target.profile);
        if source_preview.segments.is_empty() || target_preview.segments.is_empty() {
            return Err(KernelError::EmptyImport);
        }

        let now = timestamp();
        let revision_id = RevisionId::new(1);
        let mut project = Project::new(
            &request.name,
            &request.source.language_id,
            &request.target.language_id,
            revision_id,
            &now,
        );
        let mut source_document = Document::new(
            project.project_id,
            &request.source.language_id,
            &request.source.title,
            revision_id,
        );
        let mut target_document = Document::new(
            project.project_id,
            &request.target.language_id,
            &request.target.title,
            revision_id,
        );
        project.document_ids = vec![source_document.document_id, target_document.document_id];

        let source_segments = materialize_segments(
            source_document.document_id,
            "source",
            &source_preview.segments,
            revision_id,
        );
        let target_segments = materialize_segments(
            target_document.document_id,
            "target",
            &target_preview.segments,
            revision_id,
        );
        let source_order =
            SegmentOrder::initial(source_document.document_id, &source_segments, revision_id);
        let target_order =
            SegmentOrder::initial(target_document.document_id, &target_segments, revision_id);
        source_document.segment_order_id = source_order.segment_order_id;
        target_document.segment_order_id = target_order.segment_order_id;

        let initial_layout = build_provisional_layout(
            project.project_id,
            &source_segments,
            &target_segments,
            revision_id,
        )?;
        let revision = Revision {
            revision_id,
            project_id: project.project_id,
            parent_revision_id: None,
            operation_id: OperationId::new(),
            change_set: ChangeSetSummary {
                operation: "create_project".into(),
                affected_count: (source_segments.len() + target_segments.len()) as u64,
            },
            author_label: "Local user".into(),
            created_at: now.clone(),
            summary: format!(
                "Imported {} source and {} target segments",
                source_segments.len(),
                target_segments.len()
            ),
            state: RevisionState::Complete,
        };
        let source_asset = asset_record(
            source_document.source_asset_id,
            &request.source.input,
            &request.source.profile,
            &source,
            &now,
        );
        let target_asset = asset_record(
            target_document.source_asset_id,
            &request.target.input,
            &request.target.profile,
            &target,
            &now,
        );
        let mut segments = source_segments;
        segments.extend(target_segments);
        let snapshot = ProjectSnapshot {
            contract_version: CONTRACT_VERSION.into(),
            project,
            documents: vec![source_document, target_document],
            source_assets: vec![source_asset, target_asset],
            segments,
            segment_orders: vec![source_order, target_order],
            alignments: initial_layout.alignments,
            revisions: vec![revision],
            source_profile: request.source.profile.clone(),
            target_profile: request.target.profile.clone(),
            bookmarks: Vec::new(),
            annotations: Vec::new(),
        };
        validate_snapshot(&snapshot)?;
        persist_snapshot(&layout, &snapshot)?;
        Ok(snapshot)
    }

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
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn create_bookmark(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        request: BookmarkCreateRequest,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_anchor(snapshot, request.segment_id, request.alignment_id)?;
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "create_bookmark",
            1,
            format!("Created bookmark on segment {}", request.segment_id),
        );
        let now = timestamp();
        next.bookmarks.push(Bookmark {
            bookmark_id: jueming_core::BookmarkId::new(),
            project_id: snapshot.project.project_id,
            segment_id: request.segment_id,
            alignment_id: request.alignment_id,
            label: request.label,
            created_at: now.clone(),
            updated_at: now,
        });
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn list_bookmarks(&self, snapshot: &ProjectSnapshot) -> Result<Vec<Bookmark>, KernelError> {
        validate_snapshot(snapshot)?;
        Ok(snapshot.bookmarks.clone())
    }

    pub fn update_bookmark(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        request: BookmarkUpdateRequest,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_anchor(snapshot, request.segment_id, request.alignment_id)?;
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "update_bookmark",
            1,
            format!("Updated bookmark {}", request.bookmark_id),
        );
        let bookmark = next
            .bookmarks
            .iter_mut()
            .find(|bookmark| bookmark.bookmark_id == request.bookmark_id)
            .ok_or(KernelError::BookmarkNotFound(request.bookmark_id))?;
        bookmark.segment_id = request.segment_id;
        bookmark.alignment_id = request.alignment_id;
        bookmark.label = request.label;
        bookmark.updated_at = timestamp();
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn delete_bookmark(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        bookmark_id: jueming_core::BookmarkId,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let mut next = snapshot.clone();
        let old = next.bookmarks.len();
        next.bookmarks
            .retain(|bookmark| bookmark.bookmark_id != bookmark_id);
        if old == next.bookmarks.len() {
            return Err(KernelError::BookmarkNotFound(bookmark_id));
        }
        advance_revision(
            &mut next,
            "delete_bookmark",
            1,
            format!("Deleted bookmark {bookmark_id}"),
        );
        self.save_project(project_path, &next)?;
        Ok(next)
    }

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

    pub fn export(
        &self,
        snapshot: &ProjectSnapshot,
        request: &ExportRequest,
    ) -> Result<Vec<u8>, KernelError> {
        validate_snapshot(snapshot)?;
        match request.format {
            ExportFormat::Txt => Ok(export_txt(snapshot, request).into_bytes()),
            ExportFormat::Json => serde_json::to_vec_pretty(snapshot)
                .map_err(|source| KernelError::ExportJson(source.to_string())),
            ExportFormat::Xml => Ok(export_xml(snapshot, request).into_bytes()),
        }
    }

    pub fn export_to_file(
        &self,
        snapshot: &ProjectSnapshot,
        request: &ExportRequest,
    ) -> Result<(), KernelError> {
        let bytes = self.export(snapshot, request)?;
        jueming_storage::write_bytes_atomic(Path::new(&request.output_path), &bytes)?;
        Ok(())
    }

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

    pub fn update_segment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        segment_id: SegmentId,
        content: &str,
    ) -> Result<ProjectSnapshot, KernelError> {
        let mut next = snapshot.clone();
        let revision_id = advance_revision(
            &mut next,
            "update_segment",
            1,
            format!("Edited segment {segment_id}"),
        );
        let segment = next
            .segments
            .iter_mut()
            .find(|segment| segment.segment_id == segment_id)
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        segment.update_content(content, revision_id);
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn move_segment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        segment_id: SegmentId,
        before_segment_id: Option<SegmentId>,
        after_segment_id: Option<SegmentId>,
    ) -> Result<ProjectSnapshot, KernelError> {
        let mut next = snapshot.clone();
        let document_id = next
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .map(|segment| segment.document_id)
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        let revision_id = advance_revision(
            &mut next,
            "move_segment",
            1,
            format!("Moved segment {segment_id}"),
        );
        let order = next
            .segment_orders
            .iter_mut()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let moved = order
            .entries
            .iter()
            .position(|entry| entry.segment_id == segment_id)
            .map(|index| order.entries.remove(index))
            .ok_or(KernelError::SegmentNotFound(segment_id))?;
        let insertion_index = if let Some(before) = before_segment_id {
            order
                .entries
                .iter()
                .position(|entry| entry.segment_id == before)
                .map(|index| index + 1)
                .ok_or(KernelError::InvalidMoveAnchor)?
        } else if let Some(after) = after_segment_id {
            order
                .entries
                .iter()
                .position(|entry| entry.segment_id == after)
                .ok_or(KernelError::InvalidMoveAnchor)?
        } else {
            0
        };
        order.entries.insert(insertion_index, moved);
        for (index, entry) in order.entries.iter_mut().enumerate() {
            entry.position_key = PositionKey(format!("{index:020}"));
        }
        order.updated_revision_id = revision_id;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Replace one document's display order in a single durable operation.
    /// The supplied IDs must be an exact permutation of the current order so
    /// reset/reorder cannot accidentally drop or introduce segments.
    pub fn reorder_segments(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        ordered_segment_ids: Vec<SegmentId>,
    ) -> Result<ProjectSnapshot, KernelError> {
        let first_segment_id = ordered_segment_ids
            .first()
            .copied()
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let document_id = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == first_segment_id)
            .map(|segment| segment.document_id)
            .ok_or(KernelError::SegmentNotFound(first_segment_id))?;
        let current_order = snapshot
            .segment_orders
            .iter()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        let current_ids: HashSet<_> = current_order
            .entries
            .iter()
            .map(|entry| entry.segment_id)
            .collect();
        let requested_ids: HashSet<_> = ordered_segment_ids.iter().copied().collect();
        if requested_ids.len() != ordered_segment_ids.len() || requested_ids != current_ids {
            return Err(KernelError::InvalidMoveAnchor);
        }

        let mut next = snapshot.clone();
        let revision_id = advance_revision(
            &mut next,
            "reorder_segments",
            ordered_segment_ids.len() as u64,
            format!("Reordered {} segments", ordered_segment_ids.len()),
        );
        let order = next
            .segment_orders
            .iter_mut()
            .find(|order| order.document_id == document_id)
            .ok_or(KernelError::InvalidMoveAnchor)?;
        order.entries = ordered_segment_ids
            .into_iter()
            .enumerate()
            .map(|(index, segment_id)| OrderedSegmentRef {
                segment_id,
                position_key: PositionKey(format!("{index:020}")),
            })
            .collect();
        order.updated_revision_id = revision_id;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    /// Link the selected segments into one manual alignment. Existing active
    /// alignments are replaced only when the caller explicitly opts in.
    pub fn link_segments(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        source_segment_ids: Vec<SegmentId>,
        target_segment_ids: Vec<SegmentId>,
        replace_existing: bool,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        ensure_selection(snapshot, &source_segment_ids, &target_segment_ids)?;
        let mut next = snapshot.clone();
        let selected: HashSet<_> = source_segment_ids
            .iter()
            .chain(target_segment_ids.iter())
            .copied()
            .collect();
        let removed = next
            .alignments
            .iter()
            .filter(|alignment| {
                alignment
                    .source_segment_ids
                    .iter()
                    .chain(alignment.target_segment_ids.iter())
                    .any(|id| selected.contains(id))
            })
            .count();
        if removed > 0 && !replace_existing {
            return Err(KernelError::AlignmentSelectionConflict);
        }
        let revision_id = next_revision_id(&next);
        advance_revision(
            &mut next,
            "link_alignment",
            (removed + 1) as u64,
            format!(
                "Linked {} source and {} target segments",
                source_segment_ids.len(),
                target_segment_ids.len()
            ),
        );
        next.alignments.retain(|alignment| {
            !alignment
                .source_segment_ids
                .iter()
                .chain(alignment.target_segment_ids.iter())
                .any(|id| selected.contains(id))
        });
        next.alignments.push(jueming_core::Alignment::new(
            snapshot.project.project_id,
            source_segment_ids,
            target_segment_ids,
            revision_id,
        )?);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn unlink_alignment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_id: jueming_core::AlignmentId,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        if !snapshot
            .alignments
            .iter()
            .any(|alignment| alignment.alignment_id == alignment_id)
        {
            return Err(KernelError::AlignmentNotFound(alignment_id));
        }
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "unlink_alignment",
            1,
            format!("Unlinked alignment {alignment_id}"),
        );
        next.alignments
            .retain(|alignment| alignment.alignment_id != alignment_id);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn merge_alignments(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_ids: Vec<jueming_core::AlignmentId>,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        if alignment_ids.len() < 2 || has_duplicate_ids(&alignment_ids) {
            return Err(KernelError::MergeRequiresMultiple);
        }
        let selected: Vec<_> = alignment_ids
            .iter()
            .filter_map(|id| {
                snapshot
                    .alignments
                    .iter()
                    .find(|alignment| alignment.alignment_id == *id)
            })
            .cloned()
            .collect();
        if selected.len() != alignment_ids.len() {
            return Err(KernelError::AlignmentNotFound(
                *alignment_ids
                    .iter()
                    .find(|id| {
                        !snapshot
                            .alignments
                            .iter()
                            .any(|alignment| alignment.alignment_id == **id)
                    })
                    .unwrap(),
            ));
        }
        let source_ids = selected
            .iter()
            .flat_map(|alignment| alignment.source_segment_ids.iter().copied())
            .collect::<Vec<_>>();
        let target_ids = selected
            .iter()
            .flat_map(|alignment| alignment.target_segment_ids.iter().copied())
            .collect::<Vec<_>>();
        if has_duplicate_ids(&source_ids) || has_duplicate_ids(&target_ids) {
            return Err(KernelError::DuplicateActiveAlignment(
                source_ids
                    .iter()
                    .chain(target_ids.iter())
                    .find(|id| {
                        source_ids.iter().filter(|other| *other == *id).count() > 1
                            || target_ids.iter().filter(|other| *other == *id).count() > 1
                    })
                    .copied()
                    .unwrap_or(source_ids[0]),
            ));
        }
        let (source_ids, target_ids) = ordered_alignment_refs(snapshot, source_ids, target_ids);
        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "merge_alignment",
            alignment_ids.len() as u64,
            format!("Merged {} alignments", alignment_ids.len()),
        );
        next.alignments
            .retain(|alignment| !alignment_ids.contains(&alignment.alignment_id));
        next.alignments.push(jueming_core::Alignment::new(
            snapshot.project.project_id,
            source_ids,
            target_ids,
            revision_id,
        )?);
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }

    pub fn split_alignment(
        &self,
        project_path: impl AsRef<Path>,
        snapshot: &ProjectSnapshot,
        alignment_id: jueming_core::AlignmentId,
        mut source_groups: Vec<Vec<SegmentId>>,
        mut target_groups: Vec<Vec<SegmentId>>,
    ) -> Result<ProjectSnapshot, KernelError> {
        validate_snapshot(snapshot)?;
        let original = snapshot
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
        if source_groups.is_empty() && target_groups.is_empty() {
            if original.source_segment_ids.len() != original.target_segment_ids.len() {
                return Err(KernelError::SplitGroupsMismatch);
            }
            source_groups = original
                .source_segment_ids
                .iter()
                .map(|id| vec![*id])
                .collect();
            target_groups = original
                .target_segment_ids
                .iter()
                .map(|id| vec![*id])
                .collect();
        }
        validate_split_groups(original, &source_groups, &target_groups)?;
        let revision_id = next_revision_id(snapshot);
        let mut next = snapshot.clone();
        advance_revision(
            &mut next,
            "split_alignment",
            source_groups.len() as u64,
            format!(
                "Split alignment {alignment_id} into {} groups",
                source_groups.len()
            ),
        );
        next.alignments
            .retain(|alignment| alignment.alignment_id != alignment_id);
        for (source, target) in source_groups.into_iter().zip(target_groups) {
            next.alignments.push(jueming_core::Alignment::new(
                snapshot.project.project_id,
                source,
                target,
                revision_id,
            )?);
        }
        validate_snapshot(&next)?;
        self.save_project(project_path, &next)?;
        Ok(next)
    }
}

fn advance_revision(
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

fn advance_revision_with_operation(
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

fn persist_snapshot(layout: &ProjectLayout, snapshot: &ProjectSnapshot) -> Result<(), KernelError> {
    let revision_id = snapshot.project.current_revision_id;
    // Write the immutable revision first. If the main pointer write fails, a
    // harmless orphan remains; the previous project.json is still valid and no
    // history entry points at a partial file.
    layout.write_revision_snapshot(revision_id, snapshot)?;
    layout.write_snapshot(snapshot)?;
    Ok(())
}

fn read_revision_snapshot(
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

fn alignment_by_segment(
    snapshot: &ProjectSnapshot,
) -> HashMap<SegmentId, jueming_core::AlignmentId> {
    snapshot
        .alignments
        .iter()
        .flat_map(|alignment| {
            alignment
                .source_segment_ids
                .iter()
                .chain(alignment.target_segment_ids.iter())
                .map(move |id| (*id, alignment.alignment_id))
        })
        .collect()
}

fn ensure_revision(
    snapshot: &ProjectSnapshot,
    project_id: jueming_core::ProjectId,
    revision_id: RevisionId,
) -> Result<(), KernelError> {
    if snapshot.project.project_id != project_id {
        return Err(KernelError::ProjectMismatch);
    }
    if snapshot.project.current_revision_id != revision_id {
        return Err(KernelError::StaleRevision {
            expected: snapshot.project.current_revision_id,
            provided: revision_id,
        });
    }
    Ok(())
}

fn ensure_anchor(
    snapshot: &ProjectSnapshot,
    segment_id: SegmentId,
    alignment_id: Option<jueming_core::AlignmentId>,
) -> Result<(), KernelError> {
    if !snapshot
        .segments
        .iter()
        .any(|segment| segment.segment_id == segment_id)
    {
        return Err(KernelError::SegmentNotFound(segment_id));
    }
    if let Some(alignment_id) = alignment_id {
        let alignment = snapshot
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
        if !alignment.source_segment_ids.contains(&segment_id)
            && !alignment.target_segment_ids.contains(&segment_id)
        {
            return Err(KernelError::AnchorMismatch);
        }
    }
    Ok(())
}

fn ensure_annotation_anchors(
    snapshot: &ProjectSnapshot,
    segment_ids: &[SegmentId],
    alignment_id: Option<jueming_core::AlignmentId>,
) -> Result<(), KernelError> {
    if segment_ids.is_empty() || has_duplicate_ids(segment_ids) {
        return Err(KernelError::InvalidAnnotationLinks);
    }
    for id in segment_ids {
        if !snapshot
            .segments
            .iter()
            .any(|segment| segment.segment_id == *id)
        {
            return Err(KernelError::SegmentNotFound(*id));
        }
    }
    if let Some(alignment_id) = alignment_id {
        let alignment = snapshot
            .alignments
            .iter()
            .find(|alignment| alignment.alignment_id == alignment_id)
            .ok_or(KernelError::AlignmentNotFound(alignment_id))?;
        if segment_ids.iter().any(|id| {
            !alignment.source_segment_ids.contains(id) && !alignment.target_segment_ids.contains(id)
        }) {
            return Err(KernelError::AnchorMismatch);
        }
    }
    Ok(())
}

fn unlinked_segments(
    snapshot: &ProjectSnapshot,
    document_id: jueming_core::DocumentId,
) -> Vec<&Segment> {
    let linked: HashSet<_> = snapshot
        .alignments
        .iter()
        .flat_map(|alignment| {
            alignment
                .source_segment_ids
                .iter()
                .chain(alignment.target_segment_ids.iter())
        })
        .copied()
        .collect();
    snapshot
        .segments
        .iter()
        .filter(|segment| {
            segment.document_id == document_id && !linked.contains(&segment.segment_id)
        })
        .collect()
}

fn export_txt(snapshot: &ProjectSnapshot, request: &ExportRequest) -> String {
    let source_id = snapshot.documents[0].document_id;
    let target_id = snapshot.documents[1].document_id;
    let segments: HashMap<_, _> = snapshot
        .segments
        .iter()
        .map(|segment| (segment.segment_id, segment.content.as_str()))
        .collect();
    let join_side = |ids: &[SegmentId]| {
        ids.iter()
            .filter_map(|id| segments.get(id).copied())
            .collect::<Vec<_>>()
            .join(&request.side_separator)
    };
    let mut lines = snapshot
        .alignments
        .iter()
        .map(|alignment| {
            format!(
                "{}\t{}\t{}",
                serde_json::to_string(&alignment.cardinality)
                    .unwrap()
                    .trim_matches('"'),
                join_side(&alignment.source_segment_ids),
                join_side(&alignment.target_segment_ids)
            )
        })
        .collect::<Vec<_>>();
    if request.include_unlinked {
        for segment in unlinked_segments(snapshot, source_id) {
            lines.push(format!("unlinked\t{}\t", segment.content));
        }
        for segment in unlinked_segments(snapshot, target_id) {
            lines.push(format!("unlinked\t\t{}", segment.content));
        }
    }
    lines.join("\n") + if lines.is_empty() { "" } else { "\n" }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn export_xml(snapshot: &ProjectSnapshot, request: &ExportRequest) -> String {
    let segments: HashMap<_, _> = snapshot
        .segments
        .iter()
        .map(|segment| (segment.segment_id, segment.content.as_str()))
        .collect();
    let side = |name: &str, ids: &[SegmentId]| {
        format!(
            "<{name}>{}</{name}>",
            ids.iter()
                .filter_map(|id| segments.get(id).copied())
                .map(xml_escape)
                .collect::<Vec<_>>()
                .join(&xml_escape(&request.side_separator))
        )
    };
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><jueming-alignment>");
    for alignment in &snapshot.alignments {
        let cardinality = serde_json::to_string(&alignment.cardinality).unwrap();
        let cardinality = cardinality.trim_matches('"');
        out.push_str(&format!(
            "<alignment id=\"{}\" cardinality=\"{}\">{}{}</alignment>",
            alignment.alignment_id,
            cardinality,
            side("source", &alignment.source_segment_ids),
            side("target", &alignment.target_segment_ids)
        ));
    }
    if request.include_unlinked {
        for segment in snapshot
            .segments
            .iter()
            .filter(|segment| !alignment_by_segment(snapshot).contains_key(&segment.segment_id))
        {
            out.push_str(&format!(
                "<unlinked segment_id=\"{}\" document_id=\"{}\">{}</unlinked>",
                segment.segment_id,
                segment.document_id,
                xml_escape(&segment.content)
            ));
        }
    }
    out.push_str("</jueming-alignment>");
    out
}

fn next_revision_id(snapshot: &ProjectSnapshot) -> RevisionId {
    RevisionId::new(snapshot.project.current_revision_id.value() + 1)
}

fn has_duplicate_ids<T: Eq + std::hash::Hash>(values: &[T]) -> bool {
    let mut seen = HashSet::new();
    values.iter().any(|value| !seen.insert(value))
}

fn ensure_selection(
    snapshot: &ProjectSnapshot,
    source_ids: &[SegmentId],
    target_ids: &[SegmentId],
) -> Result<(), KernelError> {
    if source_ids.is_empty()
        || target_ids.is_empty()
        || has_duplicate_ids(source_ids)
        || has_duplicate_ids(target_ids)
    {
        return Err(KernelError::InvalidAlignmentSelection);
    }
    let source_document = snapshot
        .documents
        .first()
        .ok_or_else(|| KernelError::InvalidSnapshot("missing source document".into()))?;
    let target_document = snapshot
        .documents
        .get(1)
        .ok_or_else(|| KernelError::InvalidSnapshot("missing target document".into()))?;
    for id in source_ids {
        let segment = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == *id)
            .ok_or(KernelError::SegmentNotFound(*id))?;
        if segment.document_id != source_document.document_id {
            return Err(KernelError::WrongAlignmentSide(*id));
        }
    }
    for id in target_ids {
        let segment = snapshot
            .segments
            .iter()
            .find(|segment| segment.segment_id == *id)
            .ok_or(KernelError::SegmentNotFound(*id))?;
        if segment.document_id != target_document.document_id {
            return Err(KernelError::WrongAlignmentSide(*id));
        }
    }
    Ok(())
}

fn ordered_alignment_refs(
    snapshot: &ProjectSnapshot,
    mut source_ids: Vec<SegmentId>,
    mut target_ids: Vec<SegmentId>,
) -> (Vec<SegmentId>, Vec<SegmentId>) {
    let source_order = snapshot
        .segment_orders
        .iter()
        .find(|order| order.document_id == snapshot.documents[0].document_id);
    let target_order = snapshot
        .segment_orders
        .iter()
        .find(|order| order.document_id == snapshot.documents[1].document_id);
    let rank = |ids: &mut Vec<SegmentId>, order: Option<&SegmentOrder>| {
        if let Some(order) = order {
            let ranks: HashMap<_, _> = order
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| (entry.segment_id, index))
                .collect();
            ids.sort_by_key(|id| ranks.get(id).copied().unwrap_or(usize::MAX));
        }
    };
    rank(&mut source_ids, source_order);
    rank(&mut target_ids, target_order);
    (source_ids, target_ids)
}

fn validate_split_groups(
    original: &jueming_core::Alignment,
    source_groups: &[Vec<SegmentId>],
    target_groups: &[Vec<SegmentId>],
) -> Result<(), KernelError> {
    if source_groups.is_empty()
        || source_groups.len() != target_groups.len()
        || source_groups.iter().any(Vec::is_empty)
        || target_groups.iter().any(Vec::is_empty)
    {
        return Err(KernelError::SplitGroupsMismatch);
    }
    let flatten = |groups: &[Vec<SegmentId>]| groups.iter().flatten().copied().collect::<Vec<_>>();
    let source = flatten(source_groups);
    let target = flatten(target_groups);
    if has_duplicate_ids(&source)
        || has_duplicate_ids(&target)
        || {
            let mut left = source.clone();
            let mut right = original.source_segment_ids.clone();
            left.sort();
            right.sort();
            left != right
        }
        || {
            let mut left = target.clone();
            let mut right = original.target_segment_ids.clone();
            left.sort();
            right.sort();
            left != right
        }
    {
        return Err(KernelError::SplitGroupsMismatch);
    }
    Ok(())
}

struct LoadedInput {
    label: String,
    original_path: Option<String>,
    bytes: Vec<u8>,
    text: String,
    had_bom: bool,
    sha256: String,
}

fn load_input(input: &TextInput, profile: &ImportProfile) -> Result<LoadedInput, KernelError> {
    let (label, original_path, bytes, text, had_bom) = match input {
        TextInput::File { path } => {
            let bytes = fs::read(path).map_err(|source| KernelError::Io {
                path: PathBuf::from(path),
                source,
            })?;
            let decoded = decode_bytes(&bytes, profile.encoding)?;
            let label = Path::new(path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(path)
                .to_owned();
            (
                label,
                Some(path.clone()),
                bytes,
                decoded.text,
                decoded.had_bom,
            )
        }
        TextInput::Paste { label, text } => {
            if !matches!(
                profile.encoding,
                jueming_core::Encoding::Utf8 | jueming_core::Encoding::Utf8Bom
            ) {
                return Err(KernelError::InvalidPasteEncoding);
            }
            (
                label.clone(),
                None,
                text.as_bytes().to_vec(),
                text.clone(),
                false,
            )
        }
    };
    let sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(LoadedInput {
        label,
        original_path,
        bytes,
        text,
        had_bom,
        sha256,
    })
}

fn materialize_segments(
    document_id: jueming_core::DocumentId,
    namespace: &str,
    preview: &[jueming_core::SegmentPreview],
    revision: RevisionId,
) -> Vec<Segment> {
    preview
        .iter()
        .map(|item| {
            let mut segment = Segment::new(document_id, &item.content, revision);
            segment.content_ref = ContentRef {
                key: format!("{namespace}/{:08}", item.ordinal + 1),
            };
            segment
        })
        .collect()
}

fn asset_record(
    asset_id: AssetId,
    input: &TextInput,
    profile: &ImportProfile,
    loaded: &LoadedInput,
    now: &str,
) -> SourceAssetRecord {
    SourceAssetRecord {
        asset_id,
        original_path: loaded.original_path.clone().or_else(|| match input {
            TextInput::File { path } => Some(path.clone()),
            TextInput::Paste { .. } => None,
        }),
        label: loaded.label.clone(),
        encoding: profile.encoding,
        sha256: loaded.sha256.clone(),
        byte_length: loaded.bytes.len() as u64,
        imported_at: now.to_owned(),
    }
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn validate_snapshot(snapshot: &ProjectSnapshot) -> Result<(), KernelError> {
    if snapshot.contract_version != CONTRACT_VERSION || snapshot.documents.len() != 2 {
        return Err(KernelError::InvalidSnapshot(
            "contract version or bilingual documents are invalid".into(),
        ));
    }
    validate_project(&snapshot.project, &snapshot.documents, &snapshot.revisions)?;
    let source_document = &snapshot.documents[0];
    let target_document = &snapshot.documents[1];
    let segment_map: HashMap<_, _> = snapshot
        .segments
        .iter()
        .cloned()
        .map(|segment| (segment.segment_id, segment))
        .collect();
    if segment_map.len() != snapshot.segments.len() {
        return Err(KernelError::InvalidSnapshot("duplicate segment IDs".into()));
    }
    for document in &snapshot.documents {
        let segments: Vec<_> = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == document.document_id)
            .cloned()
            .collect();
        let order = snapshot
            .segment_orders
            .iter()
            .find(|order| order.segment_order_id == document.segment_order_id)
            .ok_or_else(|| KernelError::InvalidSnapshot("missing segment order".into()))?;
        validate_segment_order(order, &segments)?;
    }
    let mut occupied = HashSet::new();
    for alignment in &snapshot.alignments {
        validate_alignment(
            alignment,
            snapshot.project.project_id,
            source_document,
            target_document,
            &segment_map,
        )?;
        for segment_id in alignment
            .source_segment_ids
            .iter()
            .chain(alignment.target_segment_ids.iter())
        {
            if !occupied.insert(*segment_id) {
                return Err(KernelError::DuplicateActiveAlignment(*segment_id));
            }
        }
    }
    if snapshot
        .bookmarks
        .iter()
        .any(|bookmark| bookmark.project_id != snapshot.project.project_id)
    {
        return Err(KernelError::InvalidSnapshot(
            "bookmark belongs to another project".into(),
        ));
    }
    for bookmark in &snapshot.bookmarks {
        ensure_anchor(snapshot, bookmark.segment_id, bookmark.alignment_id)?;
    }
    for annotation in &snapshot.annotations {
        if annotation.project_id != snapshot.project.project_id {
            return Err(KernelError::InvalidSnapshot(
                "annotation belongs to another project".into(),
            ));
        }
        ensure_annotation_anchors(
            snapshot,
            &annotation.linked_segment_ids,
            annotation.alignment_id,
        )?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum KernelError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("both sides must produce at least one segment")]
    EmptyImport,
    #[error("pasted Unicode text must use UTF-8")]
    InvalidPasteEncoding,
    #[error("invalid project snapshot: {0}")]
    InvalidSnapshot(String),
    #[error("segment {0} belongs to more than one active alignment")]
    DuplicateActiveAlignment(jueming_core::SegmentId),
    #[error("segment {0} does not exist in the current project")]
    SegmentNotFound(SegmentId),
    #[error("segment move anchor is not valid for this document")]
    InvalidMoveAnchor,
    #[error("alignment {0} does not exist in the current project")]
    AlignmentNotFound(jueming_core::AlignmentId),
    #[error("an alignment selection must contain unique, non-empty source and target segments")]
    InvalidAlignmentSelection,
    #[error(
        "one or more selected segments are already aligned; confirm replacement before linking"
    )]
    AlignmentSelectionConflict,
    #[error("segment {0} is on the wrong alignment side")]
    WrongAlignmentSide(SegmentId),
    #[error("merge requires at least two distinct alignments")]
    MergeRequiresMultiple,
    #[error("split groups must be non-empty, paired, and partition the original alignment")]
    SplitGroupsMismatch,
    #[error("revision {0} is not available")]
    RevisionNotFound(RevisionId),
    #[error("revision belongs to a different project")]
    RevisionProjectMismatch,
    #[error("no earlier revision is available for undo")]
    NoUndoAvailable,
    #[error("no redo branch is available from the current revision")]
    NoRedoAvailable,
    #[error("project ID does not match the current project")]
    ProjectMismatch,
    #[error("stale revision: expected {expected}, provided {provided}")]
    StaleRevision {
        expected: RevisionId,
        provided: RevisionId,
    },
    #[error("invalid search regular expression: {0}")]
    InvalidSearchRegex(String),
    #[error("replace selection does not belong to the preview")]
    ReplaceSelectionMismatch,
    #[error("bookmark {0} does not exist")]
    BookmarkNotFound(jueming_core::BookmarkId),
    #[error("annotation {0} does not exist")]
    AnnotationNotFound(jueming_core::AnnotationId),
    #[error("annotation links must contain unique existing segments")]
    InvalidAnnotationLinks,
    #[error("alignment anchor does not include the bookmarked/annotated segment")]
    AnchorMismatch,
    #[error("export JSON failed: {0}")]
    ExportJson(String),
}

#[cfg(test)]
mod tests {
    use jueming_core::{Encoding, SegmentationMode};
    use jueming_protocol::ImportSideRequest;

    use super::*;

    fn government_request(project_path: &Path) -> CreateProjectRequest {
        CreateProjectRequest {
            project_path: project_path.to_string_lossy().into_owned(),
            name: "2024 government report".into(),
            source: ImportSideRequest {
                language_id: "zh-CN".into(),
                title: "中文（原文）".into(),
                input: TextInput::Paste {
                    label: "report_zh.txt".into(),
                    text: include_str!("../../../tests/fixtures/government-report/report_zh.txt")
                        .into(),
                },
                profile: ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
            },
            target: ImportSideRequest {
                language_id: "en".into(),
                title: "English（译文）".into(),
                input: TextInput::Paste {
                    label: "report_en.txt".into(),
                    text: include_str!("../../../tests/fixtures/government-report/report_en.txt")
                        .into(),
                },
                profile: ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
            },
        }
    }

    #[test]
    fn project_create_save_open_round_trip() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("government.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let summary = service.summarize(&snapshot).expect("summary");
        assert_eq!(summary.source_count, 8);
        assert_eq!(summary.target_count, 8);
        assert_eq!(summary.alignment_count, 8);
        assert_eq!(summary.source_unlinked_count, 0);
        assert_eq!(summary.target_unlinked_count, 0);

        let restored = service.open_project(&path).expect("open project");
        assert_eq!(restored, snapshot);
        assert_eq!(
            restored.segments[0].segment_id,
            snapshot.segments[0].segment_id
        );
    }

    #[test]
    fn one_extra_target_is_unlinked() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("unlinked.jm");
        let mut request = government_request(&path);
        if let TextInput::Paste { text, .. } = &mut request.target.input {
            text.push_str("\nOne extra target segment.");
        }
        let service = KernelService;
        let snapshot = service.create_project(&request).expect("create project");
        let summary = service.summarize(&snapshot).expect("summary");
        assert_eq!(summary.alignment_count, 8);
        assert_eq!(summary.target_unlinked_count, 1);
    }

    #[test]
    fn edits_advance_revision_and_survive_reopen() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("edited.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let segment_id = snapshot.segments[0].segment_id;
        let edited = service
            .update_segment(&path, &snapshot, segment_id, "修订后的第一句。")
            .expect("edit segment");
        assert_eq!(edited.project.current_revision_id, RevisionId::new(2));
        assert_eq!(edited.revisions.len(), 2);
        assert_eq!(
            edited
                .segments
                .iter()
                .find(|segment| segment.segment_id == segment_id)
                .expect("edited segment")
                .content,
            "修订后的第一句。"
        );
        assert_eq!(service.open_project(&path).expect("reopen"), edited);
    }

    #[test]
    fn reorder_changes_only_document_order_and_keeps_alignment_ids() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("ordered.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let source_order = &snapshot.segment_orders[0];
        let first = source_order.entries[0].segment_id;
        let second = source_order.entries[1].segment_id;
        let third = source_order.entries[2].segment_id;
        let alignment_ids: Vec<_> = snapshot
            .alignments
            .iter()
            .map(|alignment| alignment.alignment_id)
            .collect();
        let moved = service
            .move_segment(&path, &snapshot, first, Some(second), Some(third))
            .expect("move segment");
        let moved_order = &moved.segment_orders[0];
        assert_eq!(moved_order.entries[0].segment_id, second);
        assert_eq!(moved_order.entries[1].segment_id, first);
        assert_eq!(
            moved
                .alignments
                .iter()
                .map(|alignment| alignment.alignment_id)
                .collect::<Vec<_>>(),
            alignment_ids
        );
        assert_eq!(moved.project.current_revision_id, RevisionId::new(2));
    }

    #[test]
    fn batch_reorder_requires_an_exact_permutation_and_survives_reopen() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("batch-ordered.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let mut requested: Vec<_> = snapshot.segment_orders[0]
            .entries
            .iter()
            .map(|entry| entry.segment_id)
            .collect();
        requested.rotate_left(2);
        let reordered = service
            .reorder_segments(&path, &snapshot, requested.clone())
            .expect("batch reorder");
        assert_eq!(
            reordered.segment_orders[0]
                .entries
                .iter()
                .map(|entry| entry.segment_id)
                .collect::<Vec<_>>(),
            requested
        );
        assert_eq!(service.open_project(&path).expect("reopen"), reordered);

        let invalid = service
            .reorder_segments(&path, &reordered, vec![requested[0], requested[0]])
            .expect_err("duplicate IDs are rejected");
        assert!(matches!(invalid, KernelError::InvalidMoveAnchor));
    }

    #[test]
    fn link_supports_all_cardinalities_and_unlinks_existing_occupants() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("cardinality.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let source: Vec<_> = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == snapshot.documents[0].document_id)
            .map(|segment| segment.segment_id)
            .collect();
        let target: Vec<_> = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == snapshot.documents[1].document_id)
            .map(|segment| segment.segment_id)
            .collect();

        let conflict = service
            .link_segments(
                &path,
                &snapshot,
                vec![source[0]],
                vec![target[0], target[1]],
                false,
            )
            .expect_err("occupied link requires explicit replacement");
        assert!(matches!(conflict, KernelError::AlignmentSelectionConflict));
        let one_to_many = service
            .link_segments(
                &path,
                &snapshot,
                vec![source[0]],
                vec![target[0], target[1]],
                true,
            )
            .expect("1:n link");
        assert_eq!(
            one_to_many
                .alignments
                .iter()
                .find(|alignment| alignment.source_segment_ids == vec![source[0]])
                .unwrap()
                .cardinality,
            jueming_core::Cardinality::OneToMany
        );
        let many_to_one = service
            .link_segments(
                &path,
                &one_to_many,
                vec![source[1], source[2]],
                vec![target[1]],
                true,
            )
            .expect("n:1 link");
        assert_eq!(
            many_to_one
                .alignments
                .iter()
                .find(|alignment| alignment.target_segment_ids == vec![target[1]])
                .unwrap()
                .cardinality,
            jueming_core::Cardinality::ManyToOne
        );
        let many_to_many = service
            .link_segments(
                &path,
                &many_to_one,
                vec![source[3], source[4]],
                vec![target[2], target[3]],
                true,
            )
            .expect("n:m link");
        assert!(
            many_to_many
                .alignments
                .iter()
                .any(|alignment| alignment.cardinality == jueming_core::Cardinality::ManyToMany)
        );
        assert_eq!(many_to_many.project.current_revision_id, RevisionId::new(4));
    }

    #[test]
    fn unlink_merge_split_and_reopen_preserve_ids_and_revisions() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("relations.jm");
        let service = KernelService;
        let initial = service
            .create_project(&government_request(&path))
            .expect("create project");
        let source: Vec<_> = initial
            .segments
            .iter()
            .filter(|segment| segment.document_id == initial.documents[0].document_id)
            .map(|segment| segment.segment_id)
            .collect();
        let target: Vec<_> = initial
            .segments
            .iter()
            .filter(|segment| segment.document_id == initial.documents[1].document_id)
            .map(|segment| segment.segment_id)
            .collect();
        let linked = service
            .link_segments(
                &path,
                &initial,
                vec![source[0], source[1]],
                vec![target[0], target[1]],
                true,
            )
            .expect("n:m");
        let created_id = linked
            .alignments
            .iter()
            .find(|alignment| alignment.source_segment_ids.len() == 2)
            .unwrap()
            .alignment_id;
        let merged = service
            .merge_alignments(
                &path,
                &linked,
                vec![created_id, linked.alignments[2].alignment_id],
            )
            .expect("merge");
        assert_eq!(merged.project.current_revision_id, RevisionId::new(3));
        let merged_id = merged
            .alignments
            .iter()
            .find(|alignment| alignment.source_segment_ids.len() >= 2)
            .unwrap()
            .alignment_id;
        let split = service
            .split_alignment(&path, &merged, merged_id, Vec::new(), Vec::new())
            .expect("split into pairs");
        assert!(
            split
                .alignments
                .iter()
                .all(|alignment| alignment.cardinality == jueming_core::Cardinality::OneToOne)
        );
        let removed = split.alignments[0].alignment_id;
        let unlinked = service
            .unlink_alignment(&path, &split, removed)
            .expect("unlink");
        assert!(
            !unlinked
                .alignments
                .iter()
                .any(|alignment| alignment.alignment_id == removed)
        );
        assert_eq!(service.open_project(&path).expect("reopen"), unlinked);
    }

    #[test]
    fn invalid_duplicate_selection_is_rejected_without_revision_or_disk_change() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("rollback.jm");
        let service = KernelService;
        let initial = service
            .create_project(&government_request(&path))
            .expect("create project");
        let source_id = initial.segments[0].segment_id;
        let target_id = initial.segments[8].segment_id;
        let error = service
            .link_segments(
                &path,
                &initial,
                vec![source_id, source_id],
                vec![target_id],
                false,
            )
            .expect_err("duplicate selection must fail");
        assert!(matches!(error, KernelError::InvalidAlignmentSelection));
        assert_eq!(service.open_project(&path).expect("reopen"), initial);
    }

    #[test]
    fn explicit_split_groups_must_partition_original_alignment() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("split-validation.jm");
        let service = KernelService;
        let initial = service
            .create_project(&government_request(&path))
            .expect("create project");
        let source: Vec<_> = initial
            .segments
            .iter()
            .filter(|segment| segment.document_id == initial.documents[0].document_id)
            .map(|segment| segment.segment_id)
            .collect();
        let target: Vec<_> = initial
            .segments
            .iter()
            .filter(|segment| segment.document_id == initial.documents[1].document_id)
            .map(|segment| segment.segment_id)
            .collect();
        let linked = service
            .link_segments(
                &path,
                &initial,
                vec![source[0], source[1]],
                vec![target[0], target[1]],
                true,
            )
            .expect("n:m");
        let alignment_id = linked
            .alignments
            .iter()
            .find(|alignment| alignment.source_segment_ids.len() == 2)
            .unwrap()
            .alignment_id;
        let error = service
            .split_alignment(
                &path,
                &linked,
                alignment_id,
                vec![vec![source[0]]],
                vec![vec![target[0]]],
            )
            .expect_err("incomplete partition must fail");
        assert!(matches!(error, KernelError::SplitGroupsMismatch));
        assert_eq!(service.open_project(&path).expect("reopen"), linked);
    }

    #[test]
    fn history_snapshots_support_edit_move_alignment_undo_and_redo_after_reopen() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("history.jm");
        let service = KernelService;
        let initial = service
            .create_project(&government_request(&path))
            .expect("create project");
        let first = initial.segments[0].segment_id;
        let second = initial.segments[1].segment_id;
        let target = initial.segments[8].segment_id;
        let edited = service
            .update_segment(&path, &initial, first, "历史编辑后的句子")
            .expect("edit");
        let moved = service
            .move_segment(&path, &edited, first, Some(second), None)
            .expect("move");
        let changed = service
            .link_segments(&path, &moved, vec![first], vec![target], true)
            .expect("alignment");
        assert_eq!(changed.project.current_revision_id, RevisionId::new(4));

        let undo_one = service.undo(&path, &changed).expect("undo alignment");
        let reopened = service.open_project(&path).expect("reopen after undo");
        assert_eq!(reopened, undo_one);
        let undo_two = service.undo(&path, &reopened).expect("undo move");
        let redo_one = service.redo(&path, &undo_two).expect("redo move");
        let redo_two = service.redo(&path, &redo_one).expect("redo alignment");
        assert_eq!(redo_two.project.current_revision_id, RevisionId::new(8));
        assert_eq!(
            redo_two
                .segments
                .iter()
                .find(|segment| segment.segment_id == first)
                .unwrap()
                .content,
            "历史编辑后的句子"
        );
        assert!(
            redo_two
                .alignments
                .iter()
                .any(|alignment| alignment.source_segment_ids == vec![first]
                    && alignment.target_segment_ids == vec![target])
        );
        assert_eq!(service.list_revisions(&path).unwrap().revisions.len(), 8);
    }

    #[test]
    fn compare_and_restore_are_structured_and_append_only() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("restore.jm");
        let service = KernelService;
        let initial = service
            .create_project(&government_request(&path))
            .expect("create project");
        let segment_id = initial.segments[0].segment_id;
        let edited = service
            .update_segment(&path, &initial, segment_id, "new text")
            .expect("edit");
        let comparison = service
            .compare_revision(&path, RevisionId::new(1), RevisionId::new(2))
            .expect("compare");
        assert_eq!(comparison.from_revision_id, RevisionId::new(1));
        assert_eq!(comparison.to_revision_id, RevisionId::new(2));
        assert!(
            comparison
                .segment_changes
                .iter()
                .any(|change| change.segment_id == segment_id)
        );
        let restored = service
            .restore_revision(&path, &edited, RevisionId::new(1))
            .expect("restore");
        assert_eq!(restored.project.current_revision_id, RevisionId::new(3));
        assert_eq!(restored.revisions.len(), 3);
        assert_eq!(
            restored
                .segments
                .iter()
                .find(|segment| segment.segment_id == segment_id)
                .unwrap()
                .content,
            initial
                .segments
                .iter()
                .find(|segment| segment.segment_id == segment_id)
                .unwrap()
                .content
        );
        assert_eq!(
            service.open_project(&path).expect("reopen restored"),
            restored
        );
    }

    #[test]
    fn search_supports_literal_regex_language_and_invalid_regex_errors() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("search.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let request = SearchSegmentsRequest {
            project_id: snapshot.project.project_id,
            query: "government".into(),
            regex: false,
            case_sensitive: false,
            language_id: Some("en".into()),
            base_revision_id: snapshot.project.current_revision_id,
        };
        let hits = service
            .search_segments(&snapshot, &request)
            .expect("literal search");
        assert!(!hits.hits.is_empty());
        assert!(hits.hits.iter().all(|hit| hit.language_id == "en"));
        let regex_hits = service
            .search_segments(
                &snapshot,
                &SearchSegmentsRequest {
                    query: "^Employment.*stable".into(),
                    regex: true,
                    ..request.clone()
                },
            )
            .expect("regex search");
        assert_eq!(regex_hits.hits.len(), 1);
        let invalid = service
            .search_segments(
                &snapshot,
                &SearchSegmentsRequest {
                    query: "[".into(),
                    regex: true,
                    ..request
                },
            )
            .expect_err("invalid regex");
        assert!(matches!(invalid, KernelError::InvalidSearchRegex(_)));
    }

    #[test]
    fn replace_is_one_revision_stale_safe_and_undoable() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("replace.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let request = ReplacePreviewRequest {
            project_id: snapshot.project.project_id,
            query: "stable".into(),
            replacement: "steady".into(),
            regex: false,
            case_sensitive: false,
            language_id: Some("en".into()),
            base_revision_id: snapshot.project.current_revision_id,
        };
        let preview = service
            .preview_replace(&snapshot, &request)
            .expect("preview");
        assert!(!preview.selected_segment_ids.is_empty());
        let replaced = service
            .apply_replace(
                &path,
                &snapshot,
                &ReplaceApplyRequest {
                    preview: request.clone(),
                    selected_segment_ids: preview.selected_segment_ids.clone(),
                },
            )
            .expect("apply replace");
        assert_eq!(replaced.project.current_revision_id, RevisionId::new(2));
        assert_eq!(replaced.revisions.len(), 2);
        let stale = service
            .apply_replace(
                &path,
                &replaced,
                &ReplaceApplyRequest {
                    preview: request,
                    selected_segment_ids: preview.selected_segment_ids,
                },
            )
            .expect_err("stale replace");
        assert!(matches!(stale, KernelError::StaleRevision { .. }));
        let undone = service.undo(&path, &replaced).expect("undo replace");
        assert_eq!(undone.project.current_revision_id, RevisionId::new(3));
        assert_eq!(undone.segments, snapshot.segments);
    }

    #[test]
    fn bookmarks_and_annotations_follow_stable_segment_anchors() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("sidecars.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let segment_id = snapshot.segments[0].segment_id;
        let alignment_id = snapshot.alignments[0].alignment_id;
        let bookmarked = service
            .create_bookmark(
                &path,
                &snapshot,
                BookmarkCreateRequest {
                    segment_id,
                    alignment_id: Some(alignment_id),
                    label: "review".into(),
                },
            )
            .expect("bookmark");
        let bookmark_id = bookmarked.bookmarks[0].bookmark_id;
        let moved = service
            .move_segment(&path, &bookmarked, segment_id, None, None)
            .expect("reorder");
        let updated = service
            .update_bookmark(
                &path,
                &moved,
                BookmarkUpdateRequest {
                    bookmark_id,
                    segment_id,
                    alignment_id: Some(alignment_id),
                    label: "still review".into(),
                },
            )
            .expect("bookmark survives reorder");
        assert_eq!(updated.bookmarks[0].segment_id, segment_id);
        let annotated = service
            .create_annotation(
                &path,
                &updated,
                AnnotationCreateRequest {
                    title: "Terminology".into(),
                    body: "Check this phrase".into(),
                    status: AnnotationStatus::Draft,
                    linked_segment_ids: vec![segment_id],
                    alignment_id: Some(alignment_id),
                    local_author_label: "Local".into(),
                },
            )
            .expect("annotation");
        let annotation_id = annotated.annotations[0].annotation_id;
        let in_progress = service
            .update_annotation(
                &path,
                &annotated,
                AnnotationUpdateRequest {
                    annotation_id,
                    title: "Terminology".into(),
                    body: "Checked".into(),
                    status: AnnotationStatus::InProgress,
                    linked_segment_ids: vec![segment_id],
                    alignment_id: Some(alignment_id),
                },
            )
            .expect("annotation update");
        let resolved = service
            .resolve_annotation(&path, &in_progress, annotation_id)
            .expect("resolve");
        assert_eq!(resolved.annotations[0].status, AnnotationStatus::Resolved);
        assert_eq!(resolved.project.current_revision_id, RevisionId::new(7));
        assert_eq!(resolved.revisions.len(), 7);
    }

    #[test]
    fn exports_include_cardinality_and_unlinked_without_revision() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("export.jm");
        let service = KernelService;
        let snapshot = service
            .create_project(&government_request(&path))
            .expect("create project");
        let source = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == snapshot.documents[0].document_id)
            .map(|segment| segment.segment_id)
            .collect::<Vec<_>>();
        let target = snapshot
            .segments
            .iter()
            .filter(|segment| segment.document_id == snapshot.documents[1].document_id)
            .map(|segment| segment.segment_id)
            .collect::<Vec<_>>();
        let complex = service
            .link_segments(
                &path,
                &snapshot,
                vec![source[0]],
                vec![target[0], target[1]],
                true,
            )
            .expect("1:n");
        let before_revision = complex.project.current_revision_id;
        let txt_path = temporary.path().join("out.txt");
        service
            .export_to_file(
                &complex,
                &ExportRequest {
                    format: ExportFormat::Txt,
                    output_path: txt_path.to_string_lossy().into_owned(),
                    include_unlinked: true,
                    side_separator: " / ".into(),
                },
            )
            .expect("txt export");
        let txt = std::fs::read_to_string(&txt_path).expect("read txt");
        assert!(txt.contains("1:n") && txt.contains("unlinked"));
        let json_path = temporary.path().join("out.json");
        service
            .export_to_file(
                &complex,
                &ExportRequest {
                    format: ExportFormat::Json,
                    output_path: json_path.to_string_lossy().into_owned(),
                    include_unlinked: true,
                    side_separator: " ".into(),
                },
            )
            .expect("json export");
        assert!(
            std::fs::read_to_string(&json_path)
                .unwrap()
                .contains("alignments")
        );
        let xml_path = temporary.path().join("out.xml");
        service
            .export_to_file(
                &complex,
                &ExportRequest {
                    format: ExportFormat::Xml,
                    output_path: xml_path.to_string_lossy().into_owned(),
                    include_unlinked: true,
                    side_separator: " ".into(),
                },
            )
            .expect("xml export");
        let xml = std::fs::read_to_string(&xml_path).unwrap();
        assert!(xml.contains("cardinality=\"1:n\"") && xml.contains("<unlinked"));
        assert_eq!(complex.project.current_revision_id, before_revision);
    }
}
