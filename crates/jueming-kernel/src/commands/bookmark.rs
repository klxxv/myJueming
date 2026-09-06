//! Bookmark commands and current-content preview queries.

use crate::revision::{advance_revision, timestamp};
use crate::validation::{ensure_anchor, validate_snapshot};
use crate::{KernelError, KernelService};
use jueming_protocol::{
    Bookmark, BookmarkCreateRequest, BookmarkPreview, BookmarkUpdateRequest, ProjectSnapshot,
};
use std::collections::HashMap;
use std::path::Path;

impl KernelService {
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

    pub fn list_bookmarks(
        &self,
        snapshot: &ProjectSnapshot,
    ) -> Result<Vec<BookmarkPreview>, KernelError> {
        validate_snapshot(snapshot)?;
        let segment_map: HashMap<_, _> = snapshot
            .segments
            .iter()
            .map(|segment| (segment.segment_id, segment))
            .collect();
        let document_map: HashMap<_, _> = snapshot
            .documents
            .iter()
            .map(|document| (document.document_id, document))
            .collect();
        let order_map: HashMap<_, _> = snapshot
            .segment_orders
            .iter()
            .map(|order| (order.document_id, order))
            .collect();
        snapshot
            .bookmarks
            .iter()
            .map(|bookmark| {
                let segment = segment_map
                    .get(&bookmark.segment_id)
                    .ok_or(KernelError::SegmentNotFound(bookmark.segment_id))?;
                let document = document_map.get(&segment.document_id).ok_or_else(|| {
                    KernelError::InvalidSnapshot("bookmark document is missing".into())
                })?;
                let order = order_map.get(&segment.document_id).ok_or_else(|| {
                    KernelError::InvalidSnapshot("bookmark segment order is missing".into())
                })?;
                let index = order
                    .entries
                    .iter()
                    .position(|entry| entry.segment_id == bookmark.segment_id)
                    .ok_or(KernelError::SegmentNotFound(bookmark.segment_id))?;
                let context = |offset: isize| {
                    index
                        .checked_add_signed(offset)
                        .and_then(|position| order.entries.get(position))
                        .and_then(|entry| segment_map.get(&entry.segment_id))
                        .map(|segment| segment.content.clone())
                };
                Ok(BookmarkPreview {
                    bookmark: bookmark.clone(),
                    document_title: document.title.clone(),
                    language_id: document.language_id.clone(),
                    segment_content: segment.content.clone(),
                    before_context: context(-1),
                    after_context: context(1),
                })
            })
            .collect()
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
}
