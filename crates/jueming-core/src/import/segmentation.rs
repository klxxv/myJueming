//! Deterministic sentence rules and legacy text cleanup.

use crate::import::{
    ImportCleanup, ImportProfile, SegmentBoundary, SegmentPreview, SegmentationMode,
    SegmentationPreview,
};

pub fn segment_text(text: &str, profile: &ImportProfile) -> SegmentationPreview {
    let candidates = match profile.segmentation_mode {
        SegmentationMode::NonEmptyLine | SegmentationMode::LegacyTaggedLine => text
            .lines()
            .map(|line| (line, SegmentBoundary::NonEmptyLine, None))
            .collect(),
        SegmentationMode::SentenceRules => split_sentences(text),
    };
    let mut warnings = Vec::new();
    let mut segments = Vec::new();
    for (raw, boundary, boundary_marker) in candidates {
        let mut content = raw.trim().to_owned();
        let original_text = content.clone();
        let mut cleanups = Vec::new();
        if profile.segmentation_mode == SegmentationMode::LegacyTaggedLine {
            if profile.strip_seg_wrappers {
                record_cleanup(
                    &mut content,
                    &mut cleanups,
                    ImportCleanup::SegWrappers,
                    strip_seg_wrappers,
                );
            }
            if profile.strip_pos_suffixes {
                record_cleanup(
                    &mut content,
                    &mut cleanups,
                    ImportCleanup::PosSuffixesAndWhitespace,
                    strip_pos_suffixes,
                );
            }
            if profile.compact_cjk_interchar_spaces {
                record_cleanup(
                    &mut content,
                    &mut cleanups,
                    ImportCleanup::CjkSpaces,
                    compact_cjk_spaces,
                );
            }
            record_cleanup(
                &mut content,
                &mut cleanups,
                ImportCleanup::PunctuationSpaces,
                compact_punctuation_spaces,
            );
        }
        content = content.trim().to_owned();
        if !content.is_empty() {
            segments.push(SegmentPreview {
                ordinal: segments.len(),
                content,
                original_text,
                boundary,
                boundary_marker,
                cleanups,
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
fn record_cleanup(
    content: &mut String,
    cleanups: &mut Vec<ImportCleanup>,
    kind: ImportCleanup,
    clean: fn(&str) -> String,
) {
    let cleaned = clean(content);
    if cleaned != *content {
        cleanups.push(kind);
        *content = cleaned;
    }
}

fn split_sentences(text: &str) -> Vec<(&str, SegmentBoundary, Option<char>)> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if matches!(character, '。' | '！' | '？' | '!' | '?' | '.') {
            let end = index + character.len_utf8();
            if !text[start..end].trim().is_empty() {
                result.push((
                    &text[start..end],
                    SegmentBoundary::SentencePunctuation,
                    Some(character),
                ));
            }
            start = end;
        }
    }
    if !text[start..].trim().is_empty() {
        result.push((&text[start..], SegmentBoundary::TextEnd, None));
    }
    result
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
