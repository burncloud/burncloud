use std::path::Path;

use super::{is_retryable_error, normalize_aria2_path, parse_length};

#[test]
fn recognizes_only_retryable_aria2_error_codes() {
    for code in ["2", "6", "19", "29"] {
        assert!(is_retryable_error(Some(code)), "{code}");
    }
    for code in ["1", "3", "5", "24", "not-a-code"] {
        assert!(!is_retryable_error(Some(code)), "{code}");
    }
    assert!(!is_retryable_error(None));
}

#[test]
fn parses_non_negative_byte_fields_as_i64() {
    assert_eq!(parse_length("0", "totalLength").as_ref().ok(), Some(&0));
    assert_eq!(
        parse_length("123456789", "completedLength").as_ref().ok(),
        Some(&123_456_789)
    );
    assert!(parse_length("not-a-number", "downloadSpeed").is_err());
}

#[cfg(windows)]
#[test]
fn normalizes_windows_extended_paths_returned_by_aria2() {
    assert_eq!(
        normalize_aria2_path("//?/E:/burnCloud/models/model.gguf"),
        Path::new(r"E:\burnCloud\models\model.gguf")
    );
    assert_eq!(
        normalize_aria2_path(r"\\?\E:\burnCloud\models\model.gguf"),
        Path::new(r"E:\burnCloud\models\model.gguf")
    );
}
