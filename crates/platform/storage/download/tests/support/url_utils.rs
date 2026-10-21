use super::{extract_filename_from_url, is_non_public_address, sanitize_filename, validate_url};
use std::net::IpAddr;

#[test]
fn extracts_repo_name_from_supported_download_links() {
    let cases = [
        (
            "https://huggingface.co/TinyLlama/TinyLlama-1.1B-Chat-v1.0/resolve/main/model.safetensors",
            Some("TinyLlama-1.1B-Chat-v1.0"),
        ),
        (
            "https://huggingface.co/facebook/bart-large-cnn/resolve/main/rust_model.ot",
            Some("bart-large-cnn"),
        ),
        (
            "https://huggingface.co/google-bert/bert-base-uncased/resolve/main/vocab.txt",
            Some("bert-base-uncased"),
        ),
        (
            "https://hf-mirror.com/Qwen/Qwen2.5-7B-Instruct/resolve/main/config.json",
            Some("Qwen2.5-7B-Instruct"),
        ),
        (
            "https://modelscope.cn/models/Qwen/Qwen2.5-7B-Instruct/resolve/master/config.json?view=false",
            Some("Qwen2.5-7B-Instruct"),
        ),
        (
            "https://modelscope.cn/api/v1/models/Qwen/Qwen2.5-7B-Instruct/repo?Revision=master&FilePath=config.json",
            Some("Qwen2.5-7B-Instruct"),
        ),
        (
            "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct",
            None,
        ),
    ];

    for (url, expected) in cases {
        assert_eq!(extract_filename_from_url(url).as_deref(), expected, "{url}");
    }
}

#[test]
fn sanitizes_filename_and_rejects_empty_results() {
    assert!(matches!(
        sanitize_filename("../model\\file.gguf"),
        Ok(name) if name == "..modelfile.gguf"
    ));
    assert!(matches!(
        sanitize_filename("CON.txt"),
        Ok(name) if name == "_CON.txt"
    ));

    for filename in [".", "..", "...", "", "   "] {
        assert!(
            sanitize_filename(filename).is_err(),
            "{filename:?} must be rejected"
        );
    }
}

#[test]
fn classifies_non_public_and_public_ip_addresses() {
    for value in ["127.0.0.1", "10.1.2.3", "169.254.1.1", "::1", "fc00::1"] {
        let address = match value.parse::<IpAddr>() {
            Ok(address) => address,
            Err(error) => panic!("固定测试地址无效 {value}: {error}"),
        };
        assert!(is_non_public_address(&address), "{address}");
    }
    for value in ["8.8.8.8", "2606:4700:4700::1111"] {
        let address = match value.parse::<IpAddr>() {
            Ok(address) => address,
            Err(error) => panic!("固定测试地址无效 {value}: {error}"),
        };
        assert!(!is_non_public_address(&address), "{address}");
    }
}

#[tokio::test]
async fn rejects_non_http_schemes_and_literal_private_addresses() {
    for url in [
        "ftp://8.8.8.8/model",
        "file://8.8.8.8/model",
        "http://127.0.0.1/model",
        "https://10.1.2.3/model",
        "http://169.254.1.1/model",
        "http://[::1]/model",
    ] {
        assert!(
            matches!(
                validate_url(url).await,
                Err(crate::error::DownloadError::InvalidUrl(_))
            ),
            "{url}"
        );
    }
}
