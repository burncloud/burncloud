#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_types,
    reason = "模型下载接口单元测试使用固定请求和 JSON 响应断言"
)]

use super::*;
use axum::http::{header, StatusCode};
use serde_json::json;

const CHECKSUM: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn request(
    source: DownloadSource,
    model: &str,
    revision: &str,
    variant_id: &str,
    artifact_uri: &str,
    checksum: &str,
) -> DownloadModelRequest {
    DownloadModelRequest {
        model: model.into(),
        source,
        revision: revision.into(),
        variant_id: variant_id.into(),
        artifact_uri: artifact_uri.into(),
        checksum: checksum.into(),
    }
}

#[test]
fn supported_repository_urls_pass_validation() {
    for (source, model, revision, variant_id, artifact_uri) in [
        (
            DownloadSource::ModelScope,
            "Qwen/Qwen3-8B",
            "master",
            "Q4_K_M.gguf",
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
        ),
        (
            DownloadSource::HfMirror,
            "Qwen/Qwen3-8B",
            "main",
            "Q4_K_M.gguf",
            "https://hf-mirror.com/Qwen/Qwen3-8B/resolve/main/Q4_K_M.gguf",
        ),
        (
            DownloadSource::HuggingFace,
            "TheBloke/TinyLlama",
            "main",
            "ggml-model-Q4_K_M.gguf",
            "https://huggingface.co/TheBloke/TinyLlama/resolve/main/ggml-model-Q4_K_M.gguf",
        ),
    ] {
        let validated = validate_request(request(
            source,
            model,
            revision,
            variant_id,
            artifact_uri,
            CHECKSUM,
        ))
        .expect("支持的仓库地址应通过校验");
        assert_eq!(validated.checksum, CHECKSUM);
    }
}

#[test]
fn invalid_identity_checksum_and_variant_are_rejected() {
    for request in [
        request(
            DownloadSource::ModelScope,
            "../Qwen3-8B",
            "master",
            "Q4_K_M.gguf",
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
            CHECKSUM,
        ),
        request(
            DownloadSource::ModelScope,
            "Qwen/Qwen3-8B",
            "../master",
            "Q4_K_M.gguf",
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
            CHECKSUM,
        ),
        request(
            DownloadSource::ModelScope,
            "Qwen/Qwen3-8B",
            "master",
            "../Q4_K_M.gguf",
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
            CHECKSUM,
        ),
        request(
            DownloadSource::ModelScope,
            "Qwen/Qwen3-8B",
            "master",
            "Q4_K_M.gguf",
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
            "sha256:not-a-digest",
        ),
        request(
            DownloadSource::ModelScope,
            "Qwen/Qwen3-8B",
            "master",
            "Q4_K_M.gguf",
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
            &"a".repeat(64),
        ),
    ] {
        assert!(validate_request(request).is_err());
    }
}

#[test]
fn unsupported_or_mismatched_artifact_urls_are_rejected() {
    for (source, artifact_uri) in [
        (
            DownloadSource::ModelScope,
            "http://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
        ),
        (
            DownloadSource::ModelScope,
            "https://huggingface.co/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
        ),
        (
            DownloadSource::ModelScope,
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/config.json",
        ),
        (
            DownloadSource::ModelScope,
            "https://modelscope.cn/models/Qwen/Qwen3-8B/files/master/Q4_K_M.gguf",
        ),
        (
            DownloadSource::ModelScope,
            "https://modelscope.cn/models/Qwen/Other/resolve/master/Q4_K_M.gguf",
        ),
        (
            DownloadSource::HuggingFace,
            "https://huggingface.co/Qwen/Qwen3-8B/resolve/main/Q4_K_M.gguf#fragment",
        ),
        (
            DownloadSource::HfMirror,
            "https://127.0.0.1/Qwen/Qwen3-8B/resolve/main/Q4_K_M.gguf",
        ),
    ] {
        let revision = if matches!(source, DownloadSource::ModelScope) {
            "master"
        } else {
            "main"
        };
        assert!(validate_request(request(
            source,
            "Qwen/Qwen3-8B",
            revision,
            "Q4_K_M.gguf",
            artifact_uri,
            CHECKSUM,
        ))
        .is_err());
    }
}

#[test]
fn model_scope_storage_directory_is_scoped_by_identity_and_checksum() {
    let validated = validate_request(request(
        DownloadSource::ModelScope,
        "Qwen/Qwen3-8B",
        "master",
        "Q4_K_M.gguf",
        "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
        CHECKSUM,
    ))
    .expect("固定下载请求有效");

    assert_eq!(
        storage_directory(&validated),
        "models/modelscope/Qwen_Qwen3-8B/master/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert!(!storage_directory(&validated).contains(".."));
}

#[test]
fn request_source_deserialization_rejects_unknown_repository() {
    let value = json!({
        "model": "Qwen/Qwen3-8B",
        "source": "private-repository",
        "revision": "master",
        "variant_id": "Q4_K_M.gguf",
        "artifact_uri": "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
        "checksum": CHECKSUM
    });
    assert!(serde_json::from_value::<DownloadModelRequest>(value).is_err());
}

#[tokio::test]
async fn responses_are_json_and_not_cacheable() {
    let error = error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "checksum_mismatch",
        "下载文件摘要与请求摘要不一致",
    );
    assert_eq!(error.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    let body = axum::body::to_bytes(error.into_body(), 4096)
        .await
        .expect("错误响应体可读");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("错误响应为 JSON");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "checksum_mismatch");

    let identity = validate_request(request(
        DownloadSource::ModelScope,
        "Qwen/Qwen3-8B",
        "master",
        "Q4_K_M.gguf",
        "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf",
        CHECKSUM,
    ))
    .expect("固定任务身份有效");
    let success = task_response(
        StatusCode::ACCEPTED,
        DownloadTaskSnapshot {
            task_id: 42,
            identity,
            status: DownloadTaskState::Succeeded,
            completed_bytes: 10,
            total_bytes: Some(10),
            download_speed: 0,
            error: None,
            final_path: Some("/node/models/Q4_K_M.gguf".into()),
            actual_sha256: Some(CHECKSUM.into()),
        },
    );
    assert_eq!(
        success
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    let body = axum::body::to_bytes(success.into_body(), 4096)
        .await
        .expect("成功响应体可读");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("成功响应为 JSON");
    assert_eq!(value["data"]["status"], "succeeded");
}

#[test]
fn task_id_parser_accepts_only_decimal_i64_values() {
    assert_eq!(parse_task_id("42"), Ok(42));
    assert_eq!(parse_task_id(" 42 "), Ok(42));
    assert!(parse_task_id("").is_err());
    assert!(parse_task_id("-1").is_err());
    assert!(parse_task_id("1x").is_err());
    assert!(parse_task_id("9223372036854775808").is_err());
}

#[test]
fn validated_identity_contains_scoped_storage_and_filename() {
    let validated = validate_request(request(
        DownloadSource::HuggingFace,
        "TheBloke/TinyLlama",
        "main",
        "nested/ggml-model-Q4_K_M.gguf",
        "https://huggingface.co/TheBloke/TinyLlama/resolve/main/nested/ggml-model-Q4_K_M.gguf",
        CHECKSUM,
    ))
    .expect("固定下载请求有效");

    assert_eq!(validated.filename, "ggml-model-Q4_K_M.gguf");
    assert!(validated.storage_directory.contains("huggingface"));
    assert!(validated.storage_directory.contains(&CHECKSUM[7..]));
}
