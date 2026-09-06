//! Input loading, previews, language catalogue and initial project creation.

use crate::persistence::persist_snapshot;
use crate::revision::timestamp;
use crate::validation::{canonical_language, validate_snapshot};
use crate::{KernelError, KernelService};
use jueming_core::{
    AssetId, ChangeSetSummary, ContentRef, Document, ImportProfile, OperationId, Project, Revision,
    RevisionId, RevisionState, Segment, SegmentOrder, build_provisional_layout, decode_bytes,
    segment_text,
};
use jueming_protocol::{
    COMMON_LTR_LANGUAGES, CONTRACT_VERSION, CreateProjectRequest, ImportPreviewResponse,
    ProjectSnapshot, SourceAssetRecord, SupportedLanguage, TextInput,
};
use jueming_storage::ProjectLayout;
use sha2::Digest;
use sha2::Sha256;
use std::fs;
use std::path::{Path, PathBuf};

impl KernelService {
    /// Returns the frozen, left-to-right language chooser catalogue. Kept at
    /// the Kernel boundary so the UI does not duplicate a domain policy.
    pub fn supported_languages(&self) -> Vec<SupportedLanguage> {
        COMMON_LTR_LANGUAGES.to_vec()
    }

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

        let source_language = canonical_language(&request.source.language_id)?;
        let target_language = canonical_language(&request.target.language_id)?;
        let now = timestamp();
        let revision_id = RevisionId::new(1);
        let mut project = Project::new(
            &request.name,
            &source_language,
            &target_language,
            revision_id,
            &now,
        );
        let mut source_document = Document::new(
            project.project_id,
            &source_language,
            &request.source.title,
            revision_id,
        );
        let mut target_document = Document::new(
            project.project_id,
            &target_language,
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
