//! Command, query, transaction, and event orchestration.

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use jueming_core::{
    AssetId, ChangeSetSummary, ContentRef, CoreError, Document, ImportProfile, OperationId,
    PositionKey, Project, Revision, RevisionId, RevisionState, Segment, SegmentId, SegmentOrder,
    build_provisional_layout, decode_bytes, segment_text, validate_alignment, validate_project,
    validate_segment_order,
};
use jueming_protocol::{
    CONTRACT_VERSION, CreateProjectRequest, ImportPreviewResponse, ProjectSnapshot, ProjectSummary,
    SourceAssetRecord, TextInput,
};
use jueming_storage::{ProjectLayout, StorageError};
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
        };
        validate_snapshot(&snapshot)?;
        layout.write_snapshot(&snapshot)?;
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
        ProjectLayout::new(project_path.as_ref())?.write_snapshot(snapshot)?;
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
}

fn advance_revision(
    snapshot: &mut ProjectSnapshot,
    operation: &str,
    affected_count: u64,
    summary: String,
) -> RevisionId {
    let parent = snapshot.project.current_revision_id;
    let revision_id = RevisionId::new(parent.value() + 1);
    let now = timestamp();
    snapshot.project.current_revision_id = revision_id;
    snapshot.project.updated_at = now.clone();
    snapshot.revisions.push(Revision {
        revision_id,
        project_id: snapshot.project.project_id,
        parent_revision_id: Some(parent),
        operation_id: OperationId::new(),
        change_set: ChangeSetSummary {
            operation: operation.into(),
            affected_count,
        },
        author_label: "Local user".into(),
        created_at: now,
        summary,
        state: RevisionState::Complete,
    });
    revision_id
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
}
