//! Languages DTOs for the versioned Kernel boundary.

use serde::{Deserialize, Serialize};

/// Canonical IDs accepted by the MVP's left-to-right language chooser. They
/// serialize as BCP-47 base language codes; `zh-CN` is accepted separately as
/// a legacy input when opening or creating an older project.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SupportedLanguageId {
    En,
    Zh,
    Hi,
    Es,
    Fr,
    Bn,
    Pt,
    Ru,
    Id,
    De,
}

impl SupportedLanguageId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
            Self::Hi => "hi",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::Bn => "bn",
            Self::Pt => "pt",
            Self::Ru => "ru",
            Self::Id => "id",
            Self::De => "de",
        }
    }

    pub fn parse_compatible(value: &str) -> Option<Self> {
        Some(match value {
            "en" => Self::En,
            "zh" | "zh-CN" => Self::Zh,
            "hi" => Self::Hi,
            "es" => Self::Es,
            "fr" => Self::Fr,
            "bn" => Self::Bn,
            "pt" => Self::Pt,
            "ru" => Self::Ru,
            "id" => Self::Id,
            "de" => Self::De,
            _ => return None,
        })
    }
}

/// The initial language chooser catalogue.  All entries use a left-to-right
/// script, while language IDs remain ordinary BCP-47 strings so older project
/// files (for example `zh-CN`) stay readable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SupportedLanguage {
    pub language_id: &'static str,
    pub native_name: &'static str,
    pub english_name: &'static str,
}

pub const COMMON_LTR_LANGUAGES: [SupportedLanguage; 10] = [
    SupportedLanguage {
        language_id: "en",
        native_name: "English",
        english_name: "English",
    },
    SupportedLanguage {
        language_id: "zh",
        native_name: "\u{4e2d}\u{6587}",
        english_name: "Chinese",
    },
    SupportedLanguage {
        language_id: "hi",
        native_name: "\u{0939}\u{093f}\u{0928}\u{094d}\u{0926}\u{0940}",
        english_name: "Hindi",
    },
    SupportedLanguage {
        language_id: "es",
        native_name: "Espa\u{00f1}ol",
        english_name: "Spanish",
    },
    SupportedLanguage {
        language_id: "fr",
        native_name: "Fran\u{00e7}ais",
        english_name: "French",
    },
    SupportedLanguage {
        language_id: "bn",
        native_name: "\u{09ac}\u{09be}\u{0982}\u{09b2}\u{09be}",
        english_name: "Bengali",
    },
    SupportedLanguage {
        language_id: "pt",
        native_name: "Portugu\u{00ea}s",
        english_name: "Portuguese",
    },
    SupportedLanguage {
        language_id: "ru",
        native_name: "\u{0420}\u{0443}\u{0441}\u{0441}\u{043a}\u{0438}\u{0439}",
        english_name: "Russian",
    },
    SupportedLanguage {
        language_id: "id",
        native_name: "Bahasa Indonesia",
        english_name: "Indonesian",
    },
    SupportedLanguage {
        language_id: "de",
        native_name: "Deutsch",
        english_name: "German",
    },
];
