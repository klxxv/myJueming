//! Concrete encodings persisted with import facts. Automatic detection is a preview option.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Encoding {
    #[serde(rename = "utf8")]
    Utf8,
    #[serde(rename = "utf8-bom")]
    Utf8Bom,
    #[serde(rename = "gb18030")]
    Gb18030,
    #[serde(rename = "utf-16le")]
    Utf16Le,
    #[serde(rename = "utf-16be")]
    Utf16Be,
    #[serde(rename = "big5")]
    Big5,
    #[serde(rename = "shift-jis")]
    ShiftJis,
    #[serde(rename = "euc-jp")]
    EucJp,
    #[serde(rename = "iso-2022-jp")]
    Iso2022Jp,
    #[serde(rename = "euc-kr")]
    EucKr,
    #[serde(rename = "windows-1252")]
    Windows1252,
    #[serde(rename = "windows-1251")]
    Windows1251,
    #[serde(rename = "windows-1250")]
    Windows1250,
    #[serde(rename = "windows-1253")]
    Windows1253,
    #[serde(rename = "windows-1254")]
    Windows1254,
    #[serde(rename = "windows-1255")]
    Windows1255,
    #[serde(rename = "windows-1256")]
    Windows1256,
    #[serde(rename = "windows-1257")]
    Windows1257,
    #[serde(rename = "windows-1258")]
    Windows1258,
    #[serde(rename = "windows-874")]
    Windows874,
    #[serde(rename = "koi8-r")]
    Koi8R,
    #[serde(rename = "koi8-u")]
    Koi8U,
    #[serde(rename = "ibm866")]
    Ibm866,
    #[serde(rename = "macintosh")]
    Macintosh,
    #[serde(rename = "x-mac-cyrillic")]
    XMacCyrillic,
    #[serde(rename = "iso-8859-2")]
    Iso8859_2,
    #[serde(rename = "iso-8859-3")]
    Iso8859_3,
    #[serde(rename = "iso-8859-4")]
    Iso8859_4,
    #[serde(rename = "iso-8859-5")]
    Iso8859_5,
    #[serde(rename = "iso-8859-6")]
    Iso8859_6,
    #[serde(rename = "iso-8859-7")]
    Iso8859_7,
    #[serde(rename = "iso-8859-8")]
    Iso8859_8,
    #[serde(rename = "iso-8859-10")]
    Iso8859_10,
    #[serde(rename = "iso-8859-13")]
    Iso8859_13,
    #[serde(rename = "iso-8859-14")]
    Iso8859_14,
    #[serde(rename = "iso-8859-15")]
    Iso8859_15,
    #[serde(rename = "iso-8859-16")]
    Iso8859_16,
    #[serde(rename = "iso-8859-8-i")]
    Iso8859_8I,
}

impl Encoding {
    pub const ALL: &[Self] = &[
        Self::Utf8,
        Self::Utf8Bom,
        Self::Gb18030,
        Self::Utf16Le,
        Self::Utf16Be,
        Self::Big5,
        Self::ShiftJis,
        Self::EucJp,
        Self::Iso2022Jp,
        Self::EucKr,
        Self::Windows1252,
        Self::Windows1251,
        Self::Windows1250,
        Self::Windows1253,
        Self::Windows1254,
        Self::Windows1255,
        Self::Windows1256,
        Self::Windows1257,
        Self::Windows1258,
        Self::Windows874,
        Self::Koi8R,
        Self::Koi8U,
        Self::Ibm866,
        Self::Macintosh,
        Self::XMacCyrillic,
        Self::Iso8859_2,
        Self::Iso8859_3,
        Self::Iso8859_4,
        Self::Iso8859_5,
        Self::Iso8859_6,
        Self::Iso8859_7,
        Self::Iso8859_8,
        Self::Iso8859_10,
        Self::Iso8859_13,
        Self::Iso8859_14,
        Self::Iso8859_15,
        Self::Iso8859_16,
        Self::Iso8859_8I,
    ];

    pub(super) fn codec(self) -> &'static encoding_rs::Encoding {
        match self {
            Self::Utf8 => encoding_rs::UTF_8,
            Self::Utf8Bom => encoding_rs::UTF_8,
            Self::Gb18030 => encoding_rs::GB18030,
            Self::Utf16Le => encoding_rs::UTF_16LE,
            Self::Utf16Be => encoding_rs::UTF_16BE,
            Self::Big5 => encoding_rs::BIG5,
            Self::ShiftJis => encoding_rs::SHIFT_JIS,
            Self::EucJp => encoding_rs::EUC_JP,
            Self::Iso2022Jp => encoding_rs::ISO_2022_JP,
            Self::EucKr => encoding_rs::EUC_KR,
            Self::Windows1252 => encoding_rs::WINDOWS_1252,
            Self::Windows1251 => encoding_rs::WINDOWS_1251,
            Self::Windows1250 => encoding_rs::WINDOWS_1250,
            Self::Windows1253 => encoding_rs::WINDOWS_1253,
            Self::Windows1254 => encoding_rs::WINDOWS_1254,
            Self::Windows1255 => encoding_rs::WINDOWS_1255,
            Self::Windows1256 => encoding_rs::WINDOWS_1256,
            Self::Windows1257 => encoding_rs::WINDOWS_1257,
            Self::Windows1258 => encoding_rs::WINDOWS_1258,
            Self::Windows874 => encoding_rs::WINDOWS_874,
            Self::Koi8R => encoding_rs::KOI8_R,
            Self::Koi8U => encoding_rs::KOI8_U,
            Self::Ibm866 => encoding_rs::IBM866,
            Self::Macintosh => encoding_rs::MACINTOSH,
            Self::XMacCyrillic => encoding_rs::X_MAC_CYRILLIC,
            Self::Iso8859_2 => encoding_rs::ISO_8859_2,
            Self::Iso8859_3 => encoding_rs::ISO_8859_3,
            Self::Iso8859_4 => encoding_rs::ISO_8859_4,
            Self::Iso8859_5 => encoding_rs::ISO_8859_5,
            Self::Iso8859_6 => encoding_rs::ISO_8859_6,
            Self::Iso8859_7 => encoding_rs::ISO_8859_7,
            Self::Iso8859_8 => encoding_rs::ISO_8859_8,
            Self::Iso8859_10 => encoding_rs::ISO_8859_10,
            Self::Iso8859_13 => encoding_rs::ISO_8859_13,
            Self::Iso8859_14 => encoding_rs::ISO_8859_14,
            Self::Iso8859_15 => encoding_rs::ISO_8859_15,
            Self::Iso8859_16 => encoding_rs::ISO_8859_16,
            Self::Iso8859_8I => encoding_rs::ISO_8859_8_I,
        }
    }

    pub(super) fn from_codec(codec: &'static encoding_rs::Encoding) -> Option<Self> {
        // encoding_rs uses the GB18030 decoder for both GBK and GB18030.
        if codec == encoding_rs::GBK {
            return Some(Self::Gb18030);
        }
        Self::ALL
            .iter()
            .copied()
            .find(|encoding| encoding.codec() == codec)
    }
}
