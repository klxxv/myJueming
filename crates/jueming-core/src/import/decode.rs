//! Strict byte decoding without filesystem access.

use crate::error::CoreError;
use crate::import::{DecodedText, Encoding};

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
