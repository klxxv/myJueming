//! Input loading, previews, language catalogue and initial project creation.

use crate::persistence::persist_snapshot;
use crate::revision::timestamp;
use crate::validation::{canonical_language, validate_snapshot};
use crate::{KernelError, KernelService};
use jueming_core::{
    AssetId, ChangeSetSummary, ContentRef, Document, EncodingDetection, ImportProfile, OperationId,
    Project, Revision, RevisionId, RevisionState, Segment, SegmentOrder, build_provisional_layout,
    decode_bytes, detect_encoding, segment_text,
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
        self.preview_import_with_detection(input, profile, false)
    }

    pub fn preview_import_with_detection(
        &self,
        input: &TextInput,
        profile: &ImportProfile,
        auto_detect_encoding: bool,
    ) -> Result<ImportPreviewResponse, KernelError> {
        let loaded = load_input(input, profile, auto_detect_encoding)?;
        let preview = segment_text(&loaded.text, &loaded.profile);
        Ok(ImportPreviewResponse {
            encoding_detection: loaded.encoding_detection,
            label: loaded.label,
            profile: loaded.profile,
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
        let sides = std::iter::once(&request.source)
            .chain(std::iter::once(&request.target))
            .chain(request.additional_targets.iter())
            .collect::<Vec<_>>();
        // Validate every input before the first durable write, including preview digests.
        let loaded = sides
            .iter()
            .map(|side| {
                let input = load_input(&side.input, &side.profile, false)?;
                if side
                    .expected_sha256
                    .as_ref()
                    .is_some_and(|hash| hash != &input.sha256)
                {
                    return Err(KernelError::InvalidSnapshot(format!(
                        "导入内容已变化，请重新预览：{}",
                        input.label
                    )));
                }
                let preview = segment_text(&input.text, &side.profile);
                if preview.segments.is_empty() {
                    return Err(KernelError::EmptyImport);
                }
                let language = canonical_language(&side.language_id)?;
                Ok((input, preview, language))
            })
            .collect::<Result<Vec<_>, KernelError>>()?;
        let now = timestamp();
        let revision_id = RevisionId::new(1);
        let mut project =
            Project::new(&request.name, &loaded[0].2, &loaded[1].2, revision_id, &now);
        let mut documents = Vec::new();
        let mut segments = Vec::new();
        let mut segment_orders = Vec::new();
        let mut source_assets = Vec::new();
        let mut alignments = Vec::new();
        let mut source_segments = Vec::new();
        for (index, (side, (input, preview, language))) in sides.iter().zip(&loaded).enumerate() {
            let mut document =
                Document::new(project.project_id, language, &side.title, revision_id);
            let namespace = if index == 0 {
                "source".into()
            } else {
                format!("target-{index}")
            };
            let document_segments = materialize_segments(
                document.document_id,
                &namespace,
                &preview.segments,
                revision_id,
            );
            let order =
                SegmentOrder::initial(document.document_id, &document_segments, revision_id);
            document.segment_order_id = order.segment_order_id;
            if index == 0 {
                source_segments = document_segments.clone();
            } else {
                alignments.extend(
                    build_provisional_layout(
                        project.project_id,
                        &source_segments,
                        &document_segments,
                        revision_id,
                    )?
                    .alignments,
                );
            }
            source_assets.push(asset_record(
                document.source_asset_id,
                &side.input,
                &side.profile,
                input,
                &now,
            ));
            project.document_ids.push(document.document_id);
            documents.push(document);
            segment_orders.push(order);
            segments.extend(document_segments);
        }
        if !request.additional_targets.is_empty() {
            project.format_version = "2.0".into();
            project.comparison = Some(jueming_core::ComparisonSet {
                source_document_id: documents[0].document_id,
                target_document_ids: documents[1..].iter().map(|doc| doc.document_id).collect(),
            });
        }
        let revision = Revision {
            revision_id,
            project_id: project.project_id,
            parent_revision_id: None,
            operation_id: OperationId::new(),
            change_set: ChangeSetSummary {
                operation: "create_project".into(),
                affected_count: segments.len() as u64,
            },
            author_label: "Local user".into(),
            created_at: now,
            summary: format!(
                "Imported {} documents and {} segments",
                documents.len(),
                segments.len()
            ),
            state: RevisionState::Complete,
        };
        let snapshot = ProjectSnapshot {
            research_records: Vec::new(),
            command_receipts: Vec::new(),
            contract_version: if project.comparison.is_some() {
                "2.0".into()
            } else {
                CONTRACT_VERSION.into()
            },
            project,
            documents,
            source_assets,
            segments,
            segment_orders,
            alignments,
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
    profile: ImportProfile,
    encoding_detection: EncodingDetection,
    label: String,
    original_path: Option<String>,
    bytes: Vec<u8>,
    text: String,
    had_bom: bool,
    sha256: String,
}

fn load_input(
    input: &TextInput,
    profile: &ImportProfile,
    auto_detect_encoding: bool,
) -> Result<LoadedInput, KernelError> {
    let mut profile = profile.clone();
    let mut encoding_detection = EncodingDetection::Manual;
    let (label, original_path, bytes, text, had_bom) = match input {
        TextInput::File { path } => {
            let bytes = fs::read(path).map_err(|source| KernelError::Io {
                path: PathBuf::from(path),
                source,
            })?;
            if auto_detect_encoding {
                (profile.encoding, encoding_detection) = detect_encoding(&bytes)?;
            }
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
            if auto_detect_encoding {
                profile.encoding = jueming_core::Encoding::Utf8;
                encoding_detection = EncodingDetection::UnicodeText;
            }
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
        profile,
        encoding_detection,
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
        import_profile: Some(profile.clone()),
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
