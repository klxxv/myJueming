//! Preview resolves encoding once; creation uses those reviewed facts for every document.
mod common;

use jueming_core::{Encoding, EncodingDetection, ImportProfile, SegmentationMode};
use jueming_kernel::KernelService;
use jueming_protocol::TextInput;
use std::fs;

fn profile() -> ImportProfile {
    ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine)
}

#[test]
fn independently_decoded_translations_match_preview_and_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("encodings.jm");
    let mut request = common::government_request(&path);
    request.additional_targets.push(request.target.clone());
    let source = "原文第一段。\n原文第二段。";
    let french = "Première édition.\nLe café.";
    let utf16: Vec<_> = [0xff, 0xfe]
        .into_iter()
        .chain(source.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let inputs = [
        utf16,
        b"Premi\xe8re \xe9dition.\nLe caf\xe9.".to_vec(),
        b"\xef\xbb\xbfThird translation.\nSecond line.".to_vec(),
    ];
    let mut previews = Vec::new();
    for (index, side) in std::iter::once(&mut request.source)
        .chain(std::iter::once(&mut request.target))
        .chain(request.additional_targets.iter_mut())
        .enumerate()
    {
        let file = temp.path().join(format!("{index}.txt"));
        fs::write(&file, &inputs[index]).unwrap();
        side.input = TextInput::File {
            path: file.to_string_lossy().into(),
        };
        let preview = KernelService
            .preview_import_with_detection(&side.input, &side.profile, true)
            .unwrap();
        side.profile = preview.profile.clone();
        side.expected_sha256 = Some(preview.sha256.clone());
        previews.push(preview);
    }
    assert_eq!(previews[0].profile.encoding, Encoding::Utf16Le);
    assert_eq!(previews[1].profile.encoding, Encoding::Windows1252);
    assert_eq!(previews[2].profile.encoding, Encoding::Utf8Bom);
    assert_eq!(previews[1].preview.cleaned_text, french);
    let created = KernelService.create_project(&request).unwrap();
    assert_eq!(created.source_profile, previews[0].profile);
    assert_eq!(created.target_profile, previews[1].profile);
    for (index, preview) in previews.iter().enumerate() {
        let document = &created.documents[index];
        let actual: Vec<_> = created
            .segments
            .iter()
            .filter(|s| s.document_id == document.document_id)
            .map(|s| s.content.as_str())
            .collect();
        let expected: Vec<_> = preview
            .preview
            .segments
            .iter()
            .map(|s| s.content.as_str())
            .collect();
        assert_eq!(actual, expected);
        let asset = &created.source_assets[index];
        assert_eq!(asset.encoding, preview.profile.encoding);
        assert_eq!(asset.import_profile.as_ref(), Some(&preview.profile));
        assert_eq!(asset.sha256, preview.sha256);
        assert_eq!(asset.byte_length, inputs[index].len() as u64);
    }
    assert_eq!(KernelService.open_project(&path).unwrap(), created);
    assert_eq!(fs::read(temp.path().join("0.txt")).unwrap(), inputs[0]);
}

#[test]
fn changed_bytes_after_detection_cannot_create_partial_project() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("changed.jm");
    let file = temp.path().join("french.txt");
    fs::write(&file, b"Le caf\xe9.").unwrap();
    let mut request = common::government_request(&path);
    request.target.input = TextInput::File {
        path: file.to_string_lossy().into(),
    };
    let preview = KernelService
        .preview_import_with_detection(&request.target.input, &profile(), true)
        .unwrap();
    request.target.profile = preview.profile;
    request.target.expected_sha256 = Some(preview.sha256);
    fs::write(&file, b"Le th\xe9.").unwrap();
    assert!(
        KernelService
            .create_project(&request)
            .unwrap_err()
            .to_string()
            .contains("重新预览")
    );
    assert!(!path.exists());
}

#[test]
fn paste_is_already_unicode_and_manual_decoding_is_respected() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("french.txt");
    fs::write(&file, b"caf\xe9").unwrap();
    let input = TextInput::File {
        path: file.to_string_lossy().into(),
    };
    assert!(KernelService.preview_import(&input, &profile()).is_err());
    let manual = ImportProfile::new(Encoding::Windows1252, SegmentationMode::NonEmptyLine);
    let preview = KernelService.preview_import(&input, &manual).unwrap();
    assert_eq!(preview.encoding_detection, EncodingDetection::Manual);
    assert_eq!(preview.preview.cleaned_text, "café");
    let paste = TextInput::Paste {
        label: "unicode".into(),
        text: "中文 café".into(),
    };
    let preview = KernelService
        .preview_import_with_detection(&paste, &manual, true)
        .unwrap();
    assert_eq!(preview.profile.encoding, Encoding::Utf8);
    assert_eq!(preview.encoding_detection, EncodingDetection::UnicodeText);
    assert_eq!(preview.preview.cleaned_text, "中文 café");
    assert!(KernelService.preview_import(&paste, &manual).is_err());
}

#[test]
fn malformed_bom_text_never_falls_back_to_a_lossy_preview() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("broken.txt");
    fs::write(&file, b"\xef\xbb\xbf\xff").unwrap();
    let input = TextInput::File {
        path: file.to_string_lossy().into(),
    };
    assert!(
        KernelService
            .preview_import_with_detection(&input, &profile(), true)
            .is_err()
    );
    let mut request = common::government_request(&temp.path().join("broken.jm"));
    request.target.input = input;
    assert!(KernelService.create_project(&request).is_err());
    assert!(!temp.path().join("broken.jm").exists());
}
