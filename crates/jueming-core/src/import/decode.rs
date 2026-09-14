//! Local encoding detection and strict decoding without filesystem access.

use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};

use crate::error::CoreError;
use crate::import::{DecodedText, Encoding, EncodingDetection};

pub fn detect_encoding(bytes: &[u8]) -> Result<(Encoding, EncodingDetection), CoreError> {
    // UTF-32 LE starts with the UTF-16 LE BOM; reject it before BOM matching.
    if bytes.starts_with(&[0xff, 0xfe, 0, 0]) || bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        return Err(CoreError::EncodingDetection(
            "暂不支持 UTF-32，请先转换为 UTF-8".into(),
        ));
    }
    if let Some((codec, _)) = encoding_rs::Encoding::for_bom(bytes) {
        let encoding = if codec == encoding_rs::UTF_8 {
            Encoding::Utf8Bom
        } else {
            concrete_encoding(codec)?
        };
        return Ok((encoding, EncodingDetection::Bom));
    }
    // BOM-less UTF-16 and binary input must not pass as ASCII/UTF-8 with embedded NULs.
    if bytes.contains(&0) {
        return Err(CoreError::EncodingDetection(
            "文件包含 NUL 字节；若为无 BOM 的 UTF-16，请手动选择 UTF-16 LE 或 BE 并检查预览".into(),
        ));
    }
    // ISO-2022-JP consists of valid UTF-8 bytes too; let its escape sequences reach the detector.
    if !bytes.contains(&0x1b) && std::str::from_utf8(bytes).is_ok() {
        return Ok((Encoding::Utf8, EncodingDetection::Utf8));
    }
    let mut detector = EncodingDetector::new(Iso2022JpDetection::Allow);
    detector.feed(bytes, true);
    let encoding = concrete_encoding(detector.guess(None, Utf8Detection::Allow))?;
    let method = if encoding == Encoding::Utf8 {
        EncodingDetection::Utf8
    } else {
        EncodingDetection::Statistical
    };
    Ok((encoding, method))
}

fn concrete_encoding(codec: &'static encoding_rs::Encoding) -> Result<Encoding, CoreError> {
    Encoding::from_codec(codec).ok_or_else(|| {
        CoreError::EncodingDetection(format!("暂不支持 {}，请先转换为 UTF-8", codec.name()))
    })
}

pub fn decode_bytes(bytes: &[u8], encoding: Encoding) -> Result<DecodedText, CoreError> {
    let codec = encoding.codec();
    let (bytes, had_bom) = match encoding_rs::Encoding::for_bom(bytes) {
        Some((bom_codec, length)) if bom_codec == codec => (&bytes[length..], true),
        Some(_) => {
            return Err(CoreError::Decode {
                encoding,
                detail: "文件 BOM 与所选编码不符，请选择自动识别或匹配的编码".into(),
            });
        }
        None => (bytes, false),
    };
    // Never replace malformed sequences with U+FFFD or override an explicit encoding via BOM.
    let text = codec
        .decode_without_bom_handling_and_without_replacement(bytes)
        .ok_or_else(|| CoreError::Decode {
            encoding,
            detail: "包含无效或不完整的字节序列，请检查编码或原文件".into(),
        })?
        .into_owned();
    Ok(DecodedText {
        text,
        encoding,
        had_bom,
    })
}
