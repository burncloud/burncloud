use super::{extract_filename_from_url, is_non_public_address, sanitize_filename};
use std::net::IpAddr;

#[test]
fn extracts_filename_as_repo_name() {
    assert_eq!(
        extract_filename_from_url(
            "https://huggingface.co/TinyLlama/TinyLlama-1.1B-Chat-v1.0/resolve/main/model.safetensors"
        )
        .as_deref(),
        Some("TinyLlama-1.1B-Chat-v1.0")
    );
    assert_eq!(
        extract_filename_from_url(
            "https://huggingface.co/facebook/bart-large-cnn/resolve/main/rust_model.ot"
        )
        .as_deref(),
        Some("bart-large-cnn")
    );
    assert_eq!(
        extract_filename_from_url(
            "https://huggingface.co/distil-whisper/distil-large-v3/resolve/main/model.safetensors"
        )
        .as_deref(),
        Some("distil-large-v3")
    );
    assert_eq!(
        extract_filename_from_url(
            "https://huggingface.co/google-bert/bert-base-uncased/resolve/main/vocab.txt"
        )
        .as_deref(),
        Some("bert-base-uncased")
    );
    assert_eq!(
        extract_filename_from_url(
            "https://hf-mirror.com/Qwen/Qwen2.5-7B-Instruct/resolve/main/config.json"
        )
        .as_deref(),
        Some("Qwen2.5-7B-Instruct")
    );
    assert_eq!(
        extract_filename_from_url(
            "https://modelscope.cn/models/Qwen/Qwen2.5-7B-Instruct/resolve/master/config.json?view=false"
        )
        .as_deref(),
        Some("Qwen2.5-7B-Instruct")
    );
    assert_eq!(
        extract_filename_from_url(
            "https://modelscope.cn/api/v1/models/Qwen/Qwen2.5-7B-Instruct/repo?Revision=master&FilePath=config.json"
        )
        .as_deref(),
        Some("Qwen2.5-7B-Instruct")
    );
    assert_eq!(
        extract_filename_from_url("https://huggingface.co/Qwen/Qwen2.5-7B-Instruct"),
        None
    );
}

#[test]
fn sanitizes_path_separators_and_windows_device_names() {
    assert!(matches!(
        sanitize_filename("../model\\file.gguf"),
        Ok(name) if name == "..modelfile.gguf"
    ));
    assert!(matches!(
        sanitize_filename("CON.txt"),
        Ok(name) if name == "_CON.txt"
    ));
    assert!(sanitize_filename("...").is_err());
}

#[test]
#[allow(
    clippy::expect_used,
    reason = "测试地址均为固定有效字面量，expect 仅用于构造测试输入"
)]
fn rejects_non_public_ip_ranges() {
    for address in ["127.0.0.1", "10.1.2.3", "169.254.1.1", "::1", "fc00::1"] {
        let address = address.parse::<IpAddr>().expect("valid test address");
        assert!(is_non_public_address(&address), "{address}");
    }
    for address in ["8.8.8.8", "2606:4700:4700::1111"] {
        let address = address.parse::<IpAddr>().expect("valid test address");
        assert!(!is_non_public_address(&address), "{address}");
    }
}
