//! Canonical Jueming domain objects and deterministic import primitives.
//!
//! This crate deliberately has no filesystem, Tauri, or UI dependencies. It
//! owns semantic identity and the validations that protect it during reorder.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
            pub fn parse(value: &str) -> Result<Self, uuid::Error> {
                value.parse().map(Self)
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

stable_id!(ProjectId);
stable_id!(DocumentId);
stable_id!(AssetId);
stable_id!(SegmentId);
stable_id!(SegmentOrderId);
stable_id!(AlignmentId);
stable_id!(AnnotationId);
stable_id!(BookmarkId);
stable_id!(OperationId);
stable_id!(CommandId);

/// Revision IDs are strings over the wire so JavaScript cannot lose precision.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RevisionId(u64);
impl RevisionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}
impl From<u64> for RevisionId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
impl fmt::Display for RevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Serialize for RevisionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for RevisionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = RevisionId;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a decimal revision ID string")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value
                    .parse::<u64>()
                    .map(RevisionId)
                    .map_err(|_| E::custom("invalid revision ID"))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                self.visit_str(&value)
            }
        }
        deserializer.deserialize_string(Visitor)
    }
}

pub type LanguageId = String;
pub type Timestamp = String;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub project_id: ProjectId,
    pub name: String,
    pub source_language: LanguageId,
    pub target_language: LanguageId,
    pub document_ids: Vec<DocumentId>,
    pub current_revision_id: RevisionId,
    pub format_version: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Project {
    pub fn new(
        name: impl Into<String>,
        source_language: impl Into<String>,
        target_language: impl Into<String>,
        revision: RevisionId,
        now: impl Into<String>,
    ) -> Self {
        let now = now.into();
        Self {
            project_id: ProjectId::new(),
            name: name.into(),
            source_language: source_language.into(),
            target_language: target_language.into(),
            document_ids: Vec::new(),
            current_revision_id: revision,
            format_version: "1.0".into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub document_id: DocumentId,
    pub project_id: ProjectId,
    pub language_id: LanguageId,
    pub title: String,
    pub source_asset_id: AssetId,
    pub segment_order_id: SegmentOrderId,
    pub created_revision_id: RevisionId,
}

impl Document {
    pub fn new(
        project_id: ProjectId,
        language_id: impl Into<String>,
        title: impl Into<String>,
        revision: RevisionId,
    ) -> Self {
        Self {
            document_id: DocumentId::new(),
            project_id,
            language_id: language_id.into(),
            title: title.into(),
            source_asset_id: AssetId::new(),
            segment_order_id: SegmentOrderId::new(),
            created_revision_id: revision,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentRef {
    pub key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    Sentence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub segment_id: SegmentId,
    pub document_id: DocumentId,
    pub kind: SegmentKind,
    pub content_ref: ContentRef,
    pub content: String,
    pub content_hash: String,
    pub created_revision_id: RevisionId,
    pub updated_revision_id: RevisionId,
}

impl Segment {
    pub fn new(document_id: DocumentId, content: impl Into<String>, revision: RevisionId) -> Self {
        let content = content.into();
        Self {
            segment_id: SegmentId::new(),
            document_id,
            kind: SegmentKind::Sentence,
            content_ref: ContentRef { key: String::new() },
            content_hash: content_hash(&content),
            content,
            created_revision_id: revision,
            updated_revision_id: revision,
        }
    }

    pub fn update_content(&mut self, content: impl Into<String>, revision: RevisionId) {
        let content = content.into();
        self.content_hash = content_hash(&content);
        self.content = content;
        self.updated_revision_id = revision;
    }
}

fn content_hash(value: &str) -> String {
    // FNV-1a is intentionally dependency-free at this layer; storage may add a
    // cryptographic digest when it materializes ContentRef chunks.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PositionKey(pub String);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderedSegmentRef {
    pub segment_id: SegmentId,
    pub position_key: PositionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentOrder {
    pub segment_order_id: SegmentOrderId,
    pub document_id: DocumentId,
    pub entries: Vec<OrderedSegmentRef>,
    pub updated_revision_id: RevisionId,
}
impl SegmentOrder {
    pub fn initial(document_id: DocumentId, segments: &[Segment], revision: RevisionId) -> Self {
        Self {
            segment_order_id: SegmentOrderId::new(),
            document_id,
            entries: segments
                .iter()
                .enumerate()
                .map(|(i, s)| OrderedSegmentRef {
                    segment_id: s.segment_id,
                    position_key: PositionKey(format!("{:020}", i)),
                })
                .collect(),
            updated_revision_id: revision,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentProducer {
    Provisional,
    Manual,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Cardinality {
    #[serde(rename = "1:1")]
    OneToOne,
    #[serde(rename = "1:n")]
    OneToMany,
    #[serde(rename = "n:1")]
    ManyToOne,
    #[serde(rename = "n:m")]
    ManyToMany,
}
impl Cardinality {
    pub fn of(source: usize, target: usize) -> Result<Self, CoreError> {
        match (source, target) {
            (0, _) | (_, 0) => Err(CoreError::EmptyAlignment),
            (1, 1) => Ok(Self::OneToOne),
            (1, _) => Ok(Self::OneToMany),
            (_, 1) => Ok(Self::ManyToOne),
            _ => Ok(Self::ManyToMany),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Alignment {
    pub alignment_id: AlignmentId,
    pub project_id: ProjectId,
    pub source_segment_ids: Vec<SegmentId>,
    pub target_segment_ids: Vec<SegmentId>,
    pub producer: AlignmentProducer,
    pub cardinality: Cardinality,
    pub created_revision_id: RevisionId,
    pub updated_revision_id: RevisionId,
}
impl Alignment {
    pub fn new(
        project_id: ProjectId,
        source_segment_ids: Vec<SegmentId>,
        target_segment_ids: Vec<SegmentId>,
        revision: RevisionId,
    ) -> Result<Self, CoreError> {
        let cardinality = Cardinality::of(source_segment_ids.len(), target_segment_ids.len())?;
        if has_duplicates(&source_segment_ids) || has_duplicates(&target_segment_ids) {
            return Err(CoreError::DuplicateAlignmentReference);
        }
        Ok(Self {
            alignment_id: AlignmentId::new(),
            project_id,
            source_segment_ids,
            target_segment_ids,
            producer: AlignmentProducer::Manual,
            cardinality,
            created_revision_id: revision,
            updated_revision_id: revision,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangeSetSummary {
    pub operation: String,
    pub affected_count: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionState {
    Complete,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    pub revision_id: RevisionId,
    pub project_id: ProjectId,
    pub parent_revision_id: Option<RevisionId>,
    pub operation_id: OperationId,
    pub change_set: ChangeSetSummary,
    pub author_label: String,
    pub created_at: Timestamp,
    pub summary: String,
    pub state: RevisionState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Gb18030,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentationMode {
    NonEmptyLine,
    SentenceRules,
    LegacyTaggedLine,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ImportProfile {
    pub encoding: Encoding,
    pub segmentation_mode: SegmentationMode,
    pub strip_seg_wrappers: bool,
    pub strip_pos_suffixes: bool,
    pub compact_cjk_interchar_spaces: bool,
}
impl ImportProfile {
    pub fn new(encoding: Encoding, segmentation_mode: SegmentationMode) -> Self {
        let legacy = segmentation_mode == SegmentationMode::LegacyTaggedLine;
        Self {
            encoding,
            segmentation_mode,
            strip_seg_wrappers: legacy,
            strip_pos_suffixes: legacy,
            compact_cjk_interchar_spaces: legacy,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedText {
    pub text: String,
    pub encoding: Encoding,
    pub had_bom: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentPreview {
    pub ordinal: usize,
    pub content: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentationPreview {
    pub segments: Vec<SegmentPreview>,
    pub cleaned_text: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum CoreError {
    #[error("text is not valid {encoding:?}: {detail}")]
    Decode { encoding: Encoding, detail: String },
    #[error("GB18030 decoding is unavailable on this platform")]
    UnsupportedGb18030,
    #[error("alignment must contain at least one segment on each side")]
    EmptyAlignment,
    #[error("an alignment contains duplicate segment references")]
    DuplicateAlignmentReference,
    #[error("segment order has duplicate or missing segment references")]
    InvalidSegmentOrder,
    #[error("segment belongs to the wrong document")]
    WrongDocument,
    #[error("alignment contains a segment from the wrong project/document")]
    InvalidAlignmentReference,
    #[error("project contains duplicate or missing document references")]
    InvalidProject,
}

pub fn decode_bytes(bytes: &[u8], encoding: Encoding) -> Result<DecodedText, CoreError> {
    let (bytes, had_bom) = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        (&bytes[3..], true)
    } else {
        (bytes, false)
    };
    let text = match encoding {
        Encoding::Utf8 | Encoding::Utf8Bom => std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|e| CoreError::Decode {
                encoding,
                detail: e.to_string(),
            })?,
        Encoding::Gb18030 => decode_gb18030(bytes)?,
    };
    Ok(DecodedText {
        text,
        encoding,
        had_bom,
    })
}

#[cfg(windows)]
fn decode_gb18030(bytes: &[u8]) -> Result<String, CoreError> {
    const CP_GB18030: u32 = 54936;
    const MB_ERR_INVALID_CHARS: u32 = 8;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MultiByteToWideChar(
            cp: u32,
            flags: u32,
            bytes: *const u8,
            length: i32,
            wide: *mut u16,
            wide_length: i32,
        ) -> i32;
    }
    if bytes.len() > i32::MAX as usize {
        return Err(CoreError::Decode {
            encoding: Encoding::Gb18030,
            detail: "input is too large".into(),
        });
    }
    let length = unsafe {
        MultiByteToWideChar(
            CP_GB18030,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        )
    };
    if length <= 0 {
        return Err(CoreError::Decode {
            encoding: Encoding::Gb18030,
            detail: "invalid byte sequence".into(),
        });
    }
    let mut wide = vec![0u16; length as usize];
    let written = unsafe {
        MultiByteToWideChar(
            CP_GB18030,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            length,
        )
    };
    if written != length {
        return Err(CoreError::Decode {
            encoding: Encoding::Gb18030,
            detail: "invalid byte sequence".into(),
        });
    }
    String::from_utf16(&wide).map_err(|e| CoreError::Decode {
        encoding: Encoding::Gb18030,
        detail: e.to_string(),
    })
}
#[cfg(not(windows))]
fn decode_gb18030(_bytes: &[u8]) -> Result<String, CoreError> {
    Err(CoreError::UnsupportedGb18030)
}

pub fn segment_text(text: &str, profile: &ImportProfile) -> SegmentationPreview {
    let candidates = match profile.segmentation_mode {
        SegmentationMode::NonEmptyLine => text.lines().map(str::to_owned).collect(),
        SegmentationMode::SentenceRules => split_sentences(text),
        SegmentationMode::LegacyTaggedLine => extract_legacy_segments(text),
    };
    let mut warnings = Vec::new();
    let mut segments = Vec::new();
    for raw in candidates {
        let mut content = raw.trim().to_owned();
        if profile.segmentation_mode == SegmentationMode::LegacyTaggedLine {
            if profile.strip_seg_wrappers {
                content = strip_seg_wrappers(&content);
            }
            if profile.strip_pos_suffixes {
                content = strip_pos_suffixes(&content);
            }
            if profile.compact_cjk_interchar_spaces {
                content = compact_cjk_spaces(&content);
            }
            content = compact_punctuation_spaces(&content);
        }
        content = content.trim().to_owned();
        if !content.is_empty() {
            segments.push(SegmentPreview {
                ordinal: segments.len(),
                content,
            });
        }
    }
    if segments.is_empty() && !text.trim().is_empty() {
        warnings.push("文本未产生可用 Segment".into());
    }
    let cleaned_text = segments
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    SegmentationPreview {
        segments,
        cleaned_text,
        warnings,
    }
}
fn split_sentences(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if matches!(character, '。' | '！' | '？' | '!' | '?' | '.') {
            let end = index + character.len_utf8();
            if !text[start..end].trim().is_empty() {
                result.push(text[start..end].to_owned());
            }
            start = end;
        }
    }
    if !text[start..].trim().is_empty() {
        result.push(text[start..].to_owned());
    }
    result
}
fn extract_legacy_segments(text: &str) -> Vec<String> {
    // SISU-era corpora are line-aligned even when their XML-like wrappers are
    // malformed or repeated. Physical non-empty lines therefore remain the
    // stable import boundary; wrapper cleanup happens below and empty markers
    // are filtered after normalization.
    text.lines().map(str::to_owned).collect()
}
fn strip_seg_wrappers(value: &str) -> String {
    value.replace("<seg>", "").replace("</seg>", "")
}
fn is_pos_tag(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 12
        && value.chars().any(|c| c.is_ascii_alphabetic())
        && value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}
fn strip_pos_suffixes(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            if let Some(index) = token.rfind('_') {
                let (word, suffix) = token.split_at(index);
                let marker = &suffix[1..];
                let punctuation_marker = marker.chars().all(|c| !c.is_alphanumeric());
                if (is_pos_tag(marker) || punctuation_marker) && !word.is_empty() {
                    return word.to_owned();
                }
            }
            token.to_owned()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn is_cjk(value: char) -> bool {
    matches!(value as u32, 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff)
}
fn is_sentence_punctuation(value: char) -> bool {
    matches!(
        value,
        '，' | '。' | '！' | '？' | '；' | '：' | '、' | ',' | '.' | '!' | '?' | ';' | ':'
    )
}
fn compact_punctuation_spaces(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if is_sentence_punctuation(character) && output.ends_with(' ') {
            output.pop();
        }
        output.push(character);
    }
    output
}
fn compact_cjk_spaces(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut output = String::new();
    for i in 0..chars.len() {
        if chars[i].is_whitespace() {
            let previous = output.chars().next_back();
            let next = chars[i + 1..]
                .iter()
                .copied()
                .find(|character| !character.is_whitespace());
            let remove = matches!((previous, next), (Some(left), Some(right))
                if (is_cjk(left) && (is_cjk(right) || is_sentence_punctuation(right)))
                    || (is_sentence_punctuation(left) && is_cjk(right)));
            if remove || output.ends_with(' ') {
                continue;
            }
            output.push(' ');
            continue;
        }
        output.push(chars[i]);
    }
    output.trim().to_owned()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InitialLayout {
    pub alignments: Vec<Alignment>,
    pub source_unlinked: Vec<SegmentId>,
    pub target_unlinked: Vec<SegmentId>,
}
pub fn build_provisional_layout(
    project_id: ProjectId,
    source: &[Segment],
    target: &[Segment],
    revision: RevisionId,
) -> Result<InitialLayout, CoreError> {
    let count = source.len().min(target.len());
    let mut alignments = Vec::with_capacity(count);
    for index in 0..count {
        let mut alignment = Alignment::new(
            project_id,
            vec![source[index].segment_id],
            vec![target[index].segment_id],
            revision,
        )?;
        alignment.producer = AlignmentProducer::Provisional;
        alignments.push(alignment);
    }
    Ok(InitialLayout {
        alignments,
        source_unlinked: source[count..].iter().map(|s| s.segment_id).collect(),
        target_unlinked: target[count..].iter().map(|s| s.segment_id).collect(),
    })
}
pub fn provisional_alignment_layout(
    project_id: ProjectId,
    source: &[Segment],
    target: &[Segment],
    revision: RevisionId,
) -> Result<InitialLayout, CoreError> {
    build_provisional_layout(project_id, source, target, revision)
}
fn has_duplicates(values: &[SegmentId]) -> bool {
    let mut seen = HashSet::new();
    values.iter().any(|value| !seen.insert(*value))
}

pub fn validate_segment_order(order: &SegmentOrder, segments: &[Segment]) -> Result<(), CoreError> {
    if order.entries.len() != segments.len()
        || order
            .entries
            .windows(2)
            .any(|pair| pair[0].position_key >= pair[1].position_key)
    {
        return Err(CoreError::InvalidSegmentOrder);
    }
    let expected: HashSet<_> = segments.iter().map(|s| s.segment_id).collect();
    let actual: HashSet<_> = order.entries.iter().map(|e| e.segment_id).collect();
    if expected.len() != segments.len() || actual != expected {
        return Err(CoreError::InvalidSegmentOrder);
    }
    if segments.iter().any(|s| s.document_id != order.document_id) {
        return Err(CoreError::WrongDocument);
    }
    Ok(())
}
pub fn validate_alignment(
    alignment: &Alignment,
    project_id: ProjectId,
    source_document: &Document,
    target_document: &Document,
    segments: &HashMap<SegmentId, Segment>,
) -> Result<(), CoreError> {
    if alignment.project_id != project_id
        || alignment.cardinality
            != Cardinality::of(
                alignment.source_segment_ids.len(),
                alignment.target_segment_ids.len(),
            )?
        || has_duplicates(&alignment.source_segment_ids)
        || has_duplicates(&alignment.target_segment_ids)
    {
        return Err(CoreError::InvalidAlignmentReference);
    }
    for id in &alignment.source_segment_ids {
        if segments
            .get(id)
            .map(|s| s.document_id != source_document.document_id)
            .unwrap_or(true)
        {
            return Err(CoreError::InvalidAlignmentReference);
        }
    }
    for id in &alignment.target_segment_ids {
        if segments
            .get(id)
            .map(|s| s.document_id != target_document.document_id)
            .unwrap_or(true)
        {
            return Err(CoreError::InvalidAlignmentReference);
        }
    }
    Ok(())
}

/// Validate the project directory references without knowing its physical store.
pub fn validate_project(
    project: &Project,
    documents: &[Document],
    revisions: &[Revision],
) -> Result<(), CoreError> {
    let document_ids: HashSet<_> = documents
        .iter()
        .map(|document| document.document_id)
        .collect();
    if document_ids.len() != documents.len()
        || project.document_ids.len() != documents.len()
        || project.document_ids.iter().collect::<HashSet<_>>()
            != document_ids.iter().collect::<HashSet<_>>()
        || documents
            .iter()
            .any(|document| document.project_id != project.project_id)
        || !revisions.iter().any(|revision| {
            revision.project_id == project.project_id
                && revision.revision_id == project.current_revision_id
        })
    {
        return Err(CoreError::InvalidProject);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_are_uuid_v7_and_serde_transparent() {
        let id = SegmentId::new();
        assert_eq!(id.as_uuid().get_version_num(), 7);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(serde_json::from_str::<SegmentId>(&json).unwrap(), id);
    }
    #[test]
    fn revision_id_is_a_decimal_wire_string() {
        assert_eq!(
            serde_json::to_string(&RevisionId::new(42)).unwrap(),
            "\"42\""
        );
        assert_eq!(
            serde_json::from_str::<RevisionId>("\"42\"")
                .unwrap()
                .value(),
            42
        );
    }
    #[test]
    fn government_fixture_lines_make_provisional_layout() {
        let source = segment_text(
            "甲\n乙\n丙\n",
            &ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
        );
        let target = segment_text(
            "A\nB\nC\n",
            &ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
        );
        let project = ProjectId::new();
        let make = |items: &[SegmentPreview]| {
            items
                .iter()
                .enumerate()
                .map(|(i, item)| Segment {
                    segment_id: SegmentId::new(),
                    document_id: DocumentId::new(),
                    kind: SegmentKind::Sentence,
                    content_ref: ContentRef { key: i.to_string() },
                    content: item.content.clone(),
                    content_hash: String::new(),
                    created_revision_id: RevisionId::new(1),
                    updated_revision_id: RevisionId::new(1),
                })
                .collect::<Vec<_>>()
        };
        let layout = build_provisional_layout(
            project,
            &make(&source.segments),
            &make(&target.segments),
            RevisionId::new(1),
        )
        .unwrap();
        assert_eq!(layout.alignments.len(), 3);
        assert!(layout.source_unlinked.is_empty() && layout.target_unlinked.is_empty());
    }

    #[test]
    fn bundled_government_fixture_is_eight_by_eight() {
        let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
        let source = segment_text(
            include_str!("../../../tests/fixtures/government-report/report_zh.txt"),
            &profile,
        );
        let target = segment_text(
            include_str!("../../../tests/fixtures/government-report/report_en.txt"),
            &profile,
        );
        assert_eq!(source.segments.len(), 8);
        assert_eq!(target.segments.len(), 8);
        let make = |items: &[SegmentPreview]| {
            items
                .iter()
                .map(|item| Segment {
                    segment_id: SegmentId::new(),
                    document_id: DocumentId::new(),
                    kind: SegmentKind::Sentence,
                    content_ref: ContentRef {
                        key: item.ordinal.to_string(),
                    },
                    content: item.content.clone(),
                    content_hash: String::new(),
                    created_revision_id: RevisionId::new(1),
                    updated_revision_id: RevisionId::new(1),
                })
                .collect::<Vec<_>>()
        };
        let layout = build_provisional_layout(
            ProjectId::new(),
            &make(&source.segments),
            &make(&target.segments),
            RevisionId::new(1),
        )
        .unwrap();
        assert_eq!(layout.alignments.len(), 8);
        assert!(layout.source_unlinked.is_empty());
        assert!(layout.target_unlinked.is_empty());
    }
    #[test]
    fn gb18030_is_strict() {
        let bytes = [0xd6, 0xd0, 0xce, 0xc4];
        #[cfg(windows)]
        assert_eq!(
            decode_bytes(&bytes, Encoding::Gb18030).unwrap().text,
            "中文"
        );
        #[cfg(not(windows))]
        assert!(matches!(
            decode_bytes(&bytes, Encoding::Gb18030),
            Err(CoreError::UnsupportedGb18030)
        ));
        assert!(decode_bytes(&[0xff], Encoding::Utf8).is_err());
    }
    #[test]
    fn legacy_preview_removes_wrappers_pos_and_cjk_spaces() {
        let p = ImportProfile::new(Encoding::Utf8, SegmentationMode::LegacyTaggedLine);
        let preview = segment_text("<seg>中 文_NN ，_, test_VB</seg>\n<seg>第二 句。</seg>", &p);
        assert_eq!(
            preview
                .segments
                .iter()
                .map(|s| s.content.as_str())
                .collect::<Vec<_>>(),
            vec!["中文， test", "第二句。"]
        );
    }
    #[test]
    fn legacy_preview_preserves_physical_lines_with_malformed_wrappers() {
        let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::LegacyTaggedLine);
        let source = segment_text(
            "<seg> 阿 古 顿 巴\n<seg> 产 生 故 事 , 牲 牛 .</seg>\n<seg>\n",
            &profile,
        );
        let target = segment_text(
            "<seg>Akhu_FW Tenpa_NP1\n<seg> yaks_NN2 ,_ horses_NN2 .</seg>\n<seg>\n",
            &profile,
        );
        assert_eq!(
            source
                .segments
                .iter()
                .map(|segment| segment.content.as_str())
                .collect::<Vec<_>>(),
            vec!["阿古顿巴", "产生故事,牲牛."]
        );
        assert_eq!(
            target
                .segments
                .iter()
                .map(|segment| segment.content.as_str())
                .collect::<Vec<_>>(),
            vec!["Akhu Tenpa", "yaks, horses."]
        );
    }
    #[test]
    fn cardinalities_are_derived() {
        let project = ProjectId::new();
        let r = RevisionId::new(1);
        assert_eq!(
            Alignment::new(project, vec![SegmentId::new()], vec![SegmentId::new()], r)
                .unwrap()
                .cardinality,
            Cardinality::OneToOne
        );
        assert_eq!(
            Alignment::new(
                project,
                vec![SegmentId::new()],
                vec![SegmentId::new(), SegmentId::new()],
                r
            )
            .unwrap()
            .cardinality,
            Cardinality::OneToMany
        );
        assert_eq!(
            Alignment::new(
                project,
                vec![SegmentId::new(), SegmentId::new()],
                vec![SegmentId::new()],
                r
            )
            .unwrap()
            .cardinality,
            Cardinality::ManyToOne
        );
        assert_eq!(
            Alignment::new(
                project,
                vec![SegmentId::new(), SegmentId::new()],
                vec![SegmentId::new(), SegmentId::new()],
                r
            )
            .unwrap()
            .cardinality,
            Cardinality::ManyToMany
        );
    }
}
