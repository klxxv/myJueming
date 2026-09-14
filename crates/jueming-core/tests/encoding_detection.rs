//! Real byte fixtures exercise detection, strict decoding and portable legacy codecs.
use jueming_core::{Encoding, EncodingDetection, decode_bytes, detect_encoding};

#[test]
fn unicode_bom_and_plain_utf8_take_priority() {
    for text in ["", "ASCII only\n", "中文与日本語 café русский 😀"] {
        assert_eq!(
            detect_encoding(text.as_bytes()).unwrap(),
            (Encoding::Utf8, EncodingDetection::Utf8)
        );
        let bytes = [b"\xef\xbb\xbf".as_slice(), text.as_bytes()].concat();
        assert_eq!(
            detect_encoding(&bytes).unwrap(),
            (Encoding::Utf8Bom, EncodingDetection::Bom)
        );
        let decoded = decode_bytes(&bytes, Encoding::Utf8Bom).unwrap();
        assert!(decoded.had_bom);
        assert_eq!(decoded.text, text);
    }
}

#[test]
fn utf16_bom_is_detected_and_bomless_input_has_a_manual_path() {
    let text = "中文 café 😀\n第二行";
    for (encoding, bom, little) in [
        (Encoding::Utf16Le, [0xff, 0xfe], true),
        (Encoding::Utf16Be, [0xfe, 0xff], false),
    ] {
        let bytes: Vec<_> = text
            .encode_utf16()
            .flat_map(|unit| {
                if little {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                }
            })
            .collect();
        assert_eq!(decode_bytes(&bytes, encoding).unwrap().text, text);
        assert!(
            detect_encoding(&bytes).is_err(),
            "NULs must not become false UTF-8 text"
        );
        let bytes = [bom.as_slice(), &bytes].concat();
        assert_eq!(
            detect_encoding(&bytes).unwrap(),
            (encoding, EncodingDetection::Bom)
        );
        let decoded = decode_bytes(&bytes, encoding).unwrap();
        assert_eq!(decoded.text, text);
        assert!(decoded.had_bom);
    }
}

#[test]
fn detects_real_multilingual_legacy_text_without_language_hints() {
    let cases = [
        (
            encoding_rs::GB18030,
            Encoding::Gb18030,
            "今天我们一起阅读中文书籍，比较不同译本的段落，保留文字与标点。",
        ),
        (
            encoding_rs::BIG5,
            Encoding::Big5,
            "今天我們一起閱讀中文書籍，比較不同譯本的段落，保留文字與標點。",
        ),
        (
            encoding_rs::SHIFT_JIS,
            Encoding::ShiftJis,
            "これは日本語の文章です。私たちは図書館で本を読み、翻訳された文章を比較しています。",
        ),
        (
            encoding_rs::EUC_JP,
            Encoding::EucJp,
            "これは日本語の文章です。私たちは図書館で本を読み、翻訳された文章を比較しています。",
        ),
        (
            encoding_rs::ISO_2022_JP,
            Encoding::Iso2022Jp,
            "これは日本語の文章です。私たちは図書館で本を読み、翻訳された文章を比較しています。",
        ),
        (
            encoding_rs::EUC_KR,
            Encoding::EucKr,
            "우리는 도서관에서 책을 읽고 여러 번역본의 문장을 비교합니다. 한국어 문장을 정확하게 읽습니다.",
        ),
        (
            encoding_rs::WINDOWS_1251,
            Encoding::Windows1251,
            "Сегодня мы читаем интересную книгу и сравниваем несколько переводов. Сохраняем все предложения.",
        ),
        (
            encoding_rs::WINDOWS_1252,
            Encoding::Windows1252,
            "L’été à Paris : les élèves étudient la littérature française et comparent différentes éditions. C’est très intéressant.",
        ),
    ];
    for (codec, encoding, text) in cases {
        let (bytes, _, errors) = codec.encode(text);
        assert!(!errors);
        let detected = detect_encoding(&bytes).unwrap();
        assert_eq!(
            detected,
            (encoding, EncodingDetection::Statistical),
            "{}",
            codec.name()
        );
        assert_eq!(decode_bytes(&bytes, detected.0).unwrap().text, text);
    }
}

#[test]
fn gb18030_four_byte_characters_decode_on_every_platform() {
    let text = "中文𠮷😀";
    let (bytes, _, errors) = encoding_rs::GB18030.encode(text);
    assert!(!errors);
    assert_eq!(decode_bytes(&bytes, Encoding::Gb18030).unwrap().text, text);
}

#[test]
fn invalid_sequences_and_conflicting_boms_are_never_replaced() {
    for (bytes, encoding) in [
        (b"\xef\xbb\xbf\xff".as_slice(), Encoding::Utf8Bom),
        (b"\xff\xfe\x00\xd8", Encoding::Utf16Le),
        (b"\xfe\xff\x00", Encoding::Utf16Be),
        (b"\x81", Encoding::Gb18030),
        (b"\x82", Encoding::ShiftJis),
        (b"\xef\xbb\xbfhello", Encoding::Gb18030),
        (b"\xff\xfeA\x00", Encoding::Utf16Be),
    ] {
        assert!(decode_bytes(bytes, encoding).is_err(), "{encoding:?}");
    }
    assert!(detect_encoding(b"\xff\xfe\x00\x00A\x00\x00\x00").is_err());
    assert!(detect_encoding(b"\x00\x00\xfe\xff\x00\x00\x00A").is_err());
    assert!(decode_bytes(b"caf\xe9", Encoding::Utf8).is_err());
    assert_eq!(
        decode_bytes(b"caf\xe9", Encoding::Windows1252)
            .unwrap()
            .text,
        "café"
    );
}
