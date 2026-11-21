//! 使用真实本地 HTTP 响应验证目录契约，不访问公网。
use super::super::model::ChatModel;
use super::*;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Server {
    client: CatalogClient,
    requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server(responses: Vec<(u16, String)>, delay: Duration, timeout: Duration) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("测试监听");
    let base = format!("http://{}", listener.local_addr().expect("地址"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    let task = tokio::spawn(async move {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.expect("测试连接");
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).await.expect("读取请求");
                bytes.extend_from_slice(&buffer[..count]);
                if count == 0 || bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                    break;
                }
            }
            seen.lock()
                .expect("请求锁")
                .push(String::from_utf8(bytes).expect("请求文本"));
            tokio::time::sleep(delay).await;
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            if let Err(error) = stream.write_all(response.as_bytes()).await {
                // 超时测试中客户端先关闭连接，服务端允许写回失败。
                eprintln!("测试响应连接已关闭：{error}");
            }
        }
    });
    Server {
        client: CatalogClient::new(&base, timeout).expect("测试客户端"),
        requests,
        task,
    }
}
async fn json_server(values: Vec<impl serde::Serialize>) -> Server {
    server(
        values
            .into_iter()
            .map(|value| (200, serde_json::to_string(&value).expect("测试响应")))
            .collect(),
        Duration::ZERO,
        Duration::from_secs(2),
    )
    .await
}
fn version() -> CatalogVersion {
    CatalogVersion {
        verified_variants: Vec::new(),
        name: "Qwen3-8B".into(),
        model: "Qwen/Qwen3-8B".into(),
        source: "modelscope".into(),
        revision: "master".into(),
        resource_url: "https://modelscope.cn/models/Qwen/Qwen3-8B/files?Revision=master".into(),
        variants: Vec::new(),
    }
}
#[allow(
    clippy::disallowed_types,
    reason = "测试需要构造模型选择协议及非法响应"
)]
fn item(preset: bool, source: &str) -> serde_json::Value {
    json!({"id":"qwen3-8b","name":"Qwen3-8B","preset":preset,"model":"Qwen/Qwen3-8B","source":source,
        "revision":if source == "modelscope" { "master" } else { "main" }})
}

#[allow(clippy::disallowed_types, reason = "测试需要构造完整预置目录协议")]
fn preset_item(source: &str) -> serde_json::Value {
    json!({"id":"qwen3-8b","name":"Qwen3-8B","preset":true,"model":"Qwen/Qwen3-8B-GGUF","source":source,
        "revision":if source == "modelscope" { "master" } else { "main" }})
}

#[allow(clippy::disallowed_types, reason = "测试需要构造完整状态协议")]
fn status(source: &str, status: &str) -> serde_json::Value {
    json!({"status":status,"model":"Qwen/Qwen3-8B","source":source,
        "revision":if source == "modelscope" {"master"} else {"main"}, "resource_url":"https://example.com/models/Qwen/Qwen3-8B",
        "variants":[{"id":"qwen3-8b-q4","quantization":"Q4_K_M","runtime":"llama.cpp",
            "artifact_source":"https://example.com/qwen3-8b-q4.gguf",
            "expected_sha256":format!("sha256:{}", "a".repeat(64)),
            "ram_bytes":1,"vram_bytes":1,"disk_bytes":1}]})
}

async fn collect(
    client: &CatalogClient,
    query: &str,
) -> Result<Vec<CatalogPage<CatalogModel>>, ChatError> {
    let mut pages = Vec::new();
    client.models(query, |page| pages.push(page)).await?;
    Ok(pages)
}

#[tokio::test]
async fn selected_catalog_model_resolution_parses_the_node_contract() {
    let checksum = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let service = json_server(vec![json!({
        "success": true,
        "data": {
            "status": "resolved",
            "model": "Qwen/Qwen3-8B",
            "hardware": {
                "platform": "windows",
                "arch": "x86_64",
                "cpu": "测试 CPU",
                "memory_bytes": 8589934592u64,
                "disk_available_bytes": 17179869184u64,
                "accelerators": [{"kind":"nvidia","name":"测试 GPU","memory_bytes":4294967296u64}]
            },
            "candidates": [{
                "model_name":"Qwen/Qwen3-8B",
                "variant_id":"Qwen3-8B-GGUF/qwen3-8b-q4_k_m.gguf",
                "quantization":"Q4_K_M",
                "min_ram_bytes":4294967296u64,
                "min_vram_bytes":0u64,
                "disk_size_bytes":1929903008u64,
                "runtime":"llama.cpp",
                "artifact_uri":"https://example.invalid/qwen.gguf",
                "checksum":checksum
            }]
        }
    })])
    .await;
    let selection = CatalogSelection {
        name: "Qwen3-8B".into(),
        preset: Some("qwen3-8b".into()),
        version: version(),
    };

    let resolution = service
        .client
        .resolve_model(&selection)
        .await
        .expect("模型解析");
    assert_eq!(
        resolution.variants[0].id,
        "Qwen3-8B-GGUF/qwen3-8b-q4_k_m.gguf"
    );
    assert_eq!(resolution.variants[0].expected_sha256, checksum);
    assert_eq!(resolution.variants[0].vram_bytes, 0);
    assert_eq!(resolution.hardware.max_vram(), Some(4294967296));

    let calls = service.requests.lock().expect("请求记录");
    assert_eq!(calls.len(), 1);
    assert!(calls[0].starts_with("POST /api/node/models/resolve"));
    assert!(calls[0].contains("Qwen/Qwen3-8B"));
    assert!(calls[0].contains("modelscope"));
    assert!(calls[0].contains("master"));
}

#[tokio::test]
async fn model_resolution_rejects_mismatched_candidate_model() {
    let service = json_server(vec![json!({
        "success": true,
        "data": {
            "status": "resolved",
            "model": "qwen2.5:3b",
            "hardware": {
                "platform": "windows",
                "arch": "x86_64",
                "cpu": "测试 CPU",
                "memory_bytes": 8,
                "disk_available_bytes": 8,
                "accelerators": []
            },
            "candidates": [{
                "model_name":"other",
                "variant_id":"other-q4",
                "quantization":"Q4_K_M",
                "min_ram_bytes":1,
                "min_vram_bytes":0,
                "disk_size_bytes":1,
                "runtime":"llama.cpp",
                "artifact_uri":"https://example.invalid/model.gguf",
                "checksum":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }]
        }
    })])
    .await;

    assert!(service
        .client
        .resolve_model(&CatalogSelection {
            name: "Qwen3-8B".into(),
            preset: None,
            version: version(),
        })
        .await
        .is_err());
}

#[tokio::test]
async fn presets_load_from_main_service_with_flat_resources() {
    let service = json_server(vec![json!([preset_item("modelscope")])]).await;
    let pages = collect(&service.client, " ").await.expect("预置列表");
    assert_eq!(pages.len(), 1);
    assert!(pages[0].done && pages[0].items[0].preset);
    assert_eq!(pages[0].items[0].id, "qwen3-8b");
    assert_eq!(pages[0].items[0].model, "Qwen/Qwen3-8B-GGUF");
    let calls = service.requests.lock().expect("请求记录");
    assert_eq!(calls.len(), 1);
    assert!(calls[0].starts_with("GET /api/models/presets"));
    assert!(!calls[0].contains("page="));
}

#[tokio::test]
async fn imported_presets_preserve_duplicate_rows_hardware_and_links() {
    let mut first = item(true, "modelscope");
    first["resource_url"] = json!("https://www.modelscope.cn/models/Qwen/Qwen3-8B");
    first["hardware"] = json!({
        "vram_min":"6.00G", "ram_min":"12.00G", "disk_min":"6.00G",
        "vram_recommended":"8.00G", "ram_recommended":"16.00G", "disk_recommended":"8.00G"
    });
    let mut duplicate = first.clone();
    duplicate["id"] = json!("qwen3-8b-row-24");
    let service = json_server(vec![json!([first, duplicate])]).await;
    let pages = collect(&service.client, "").await.expect("导入列表");
    let items = &pages[0].items;
    assert_eq!(items.len(), 2);
    assert_ne!(items[0].id, items[1].id);
    assert_eq!(items[0].name, items[1].name);
    assert_eq!(
        items[0].resource_url.as_deref(),
        Some("https://www.modelscope.cn/models/Qwen/Qwen3-8B")
    );
    assert_eq!(
        items[0].hardware,
        Some(HardwareRequirements {
            vram_min: "6.00G".into(),
            ram_min: "12.00G".into(),
            disk_min: "6.00G".into(),
            vram_recommended: "8.00G".into(),
            ram_recommended: "16.00G".into(),
            disk_recommended: "8.00G".into()
        })
    );
}

#[test]
fn catalog_rejects_unsafe_links_and_duplicate_suffixes_in_search_results() {
    for link in [
        "javascript:alert(1)",
        "file:///tmp/model",
        "https://user:password@example.com",
    ] {
        let mut value = item(true, "modelscope");
        value["resource_url"] = json!(link);
        let model: CatalogModel = serde_json::from_value(value).expect("测试条目");
        assert!(model.validate().is_err());
    }
    for (preset, id) in [
        (false, "qwen3-8b-row-24"),
        (true, "qwen3-8b-row-"),
        (true, "qwen3-8b-row-../1"),
    ] {
        let mut value = item(preset, "modelscope");
        value["id"] = json!(id);
        let model: CatalogModel = serde_json::from_value(value).expect("测试条目");
        assert!(model.validate().is_err());
    }
}

#[tokio::test]
async fn search_publishes_snapshots_and_only_reports_empty_final_results() {
    let partial = json!({"done":false,"items":[item(false,"hf-mirror")]}).to_string();
    let last = json!({"done":true,"type":"models","items":[item(false,"modelscope")]}).to_string();
    let service = server(
        vec![(200, format!("{partial}\n{last}\n"))],
        Duration::ZERO,
        Duration::from_secs(2),
    )
    .await;
    let pages = collect(&service.client, " qwen ").await.expect("搜索结果");
    assert_eq!(pages.len(), 2);
    assert!(!pages[0].done && pages[1].done);
    assert_eq!(pages[0].items[0].source, "hf-mirror");
    assert_eq!(pages[1].items[0].source, "modelscope");
    assert!(pages.iter().all(|page| page.feedback().is_none()));
    assert!(service.requests.lock().expect("请求")[0].contains("/api/models/search?q=qwen"));
    for (kind, feedback) in [
        ("unavailable", "没有找到匹配模型"),
        ("failed", "模型搜索失败，请重试"),
    ] {
        let service = json_server(vec![json!({"done":true,"type":kind,"items":[]})]).await;
        let pages = collect(&service.client, "q").await.expect("最终响应");
        assert_eq!(pages[0].feedback().as_deref(), Some(feedback));
    }
}

#[tokio::test]
async fn one_status_request_resolves_each_selection_and_accepts_four_states() {
    for preset in [true, false] {
        let source = if preset { "modelscope" } else { "hf-mirror" };
        let model: CatalogModel = serde_json::from_value(item(preset, source)).expect("目录模型");
        for (name, expected) in [
            ("available", VersionStatus::Available),
            ("preparing", VersionStatus::Preparing),
            ("downloadable", VersionStatus::Downloadable),
            ("developing", VersionStatus::Developing),
        ] {
            let service = json_server(vec![status(source, name)]).await;
            let inspection = service.client.inspect(&model).await.expect("状态");
            assert_eq!(inspection.remote_status, expected);
            let resource = inspection.version;
            assert_eq!(resource.source, source);
            assert_eq!(resource.variants[0].id, "qwen3-8b-q4");
            let calls = service.requests.lock().expect("请求");
            assert_eq!(calls.len(), 1);
            assert!(calls[0].contains("/api/models/status?"));
            if preset {
                assert!(calls[0].contains("preset=qwen3-8b"));
                assert!(!calls[0].contains("model="));
            } else {
                assert!(
                    calls[0].contains("model=Qwen%2FQwen3-8B")
                        && calls[0].contains("source=hf-mirror")
                        && calls[0].contains("revision=main")
                );
            }
        }
    }
}

#[tokio::test]
async fn hardware_snapshot_decodes_server_contract_and_preserves_unknown_memory() {
    let service = json_server(vec![json!({
        "success": true,
        "data": {
            "platform": "windows",
            "arch": "x86_64",
            "cpu": "Example CPU",
            "memory_bytes": 16 * 1024 * 1024 * 1024_u64,
            "disk_available_bytes": 32 * 1024 * 1024 * 1024_u64,
            "accelerators": [{
                "kind": "nvidia",
                "name": "Example GPU",
                "memory_bytes": null
            }]
        }
    })])
    .await;

    let hardware = service.client.inspect_hardware().await.expect("硬件快照");
    assert_eq!(hardware.platform, Some(Platform::Windows));
    assert_eq!(hardware.cpu.as_deref(), Some("Example CPU"));
    assert_eq!(hardware.ram_bytes, Some(16 * 1024 * 1024 * 1024));
    assert_eq!(hardware.disk_bytes, Some(32 * 1024 * 1024 * 1024));
    assert_eq!(hardware.gpus.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        hardware
            .gpus
            .as_ref()
            .and_then(|gpus| gpus[0].available_bytes),
        None
    );
    assert!(service.requests.lock().expect("请求")[0].starts_with("GET /api/node/hardware"));
}

#[tokio::test]
async fn hardware_error_response_preserves_public_error_message() {
    let service = server(
        vec![(
            501,
            json!({
                "success": false,
                "error": {
                    "code": "unsupported_platform",
                    "message": "当前平台暂不支持硬件检测"
                }
            })
            .to_string(),
        )],
        Duration::ZERO,
        Duration::from_secs(2),
    )
    .await;

    let error = service
        .client
        .inspect_hardware()
        .await
        .expect_err("错误响应必须失败");
    assert_eq!(error.kind, super::super::model::ErrorKind::ExecutionFailed);
    assert!(error.reason.contains("unsupported_platform"));
    assert!(error.reason.contains("当前平台暂不支持硬件检测"));
}
#[tokio::test]
async fn hardware_snapshot_rejects_unknown_accelerator_kind() {
    let service = json_server(vec![json!({
        "success": true,
        "data": {
            "platform": "linux",
            "arch": "x86_64",
            "cpu": "Example CPU",
            "memory_bytes": 1,
            "disk_available_bytes": 1,
            "accelerators": [{
                "kind": "unknown",
                "name": "Example GPU",
                "memory_bytes": 1
            }]
        }
    })])
    .await;

    assert!(service.client.inspect_hardware().await.is_err());
}

#[tokio::test]
async fn malformed_truncated_or_inconsistent_responses_fail_explicitly() {
    let valid = json!({"done":true,"type":"models","items":[item(false,"modelscope")]}).to_string();
    for body in [String::new(), "<html>not json</html>".into(), json!({"done":false,"items":[item(false,"modelscope")]}).to_string(),
        json!({"done":true,"items":[]}).to_string(), json!({"done":true,"type":"unexpected","items":[]}).to_string(),
        json!({"done":true,"type":"models","items":[]}).to_string(), json!({"done":true,"type":"failed","items":[item(false,"modelscope")]}).to_string(),
        json!({"done":true,"type":"models","items":[item(false,"modelscope"),item(false,"modelscope")]}).to_string(),
        format!("{valid}\n{valid}\n"), format!("{valid}\n{{")]
    {
        let service = server(vec![(200,body)],Duration::ZERO,Duration::from_secs(2)).await;
        assert!(collect(&service.client,"q").await.is_err());
    }
    for bad in [
        json!([{"id":"broken"}]),
        json!([item(false, "modelscope")]),
        json!([item(true, "unknown")]),
        json!([item(true, "modelscope"), item(true, "modelscope")]),
    ] {
        let service = json_server(vec![bad]).await;
        assert!(collect(&service.client, "").await.is_err());
    }
    let model = serde_json::from_value(item(true, "modelscope")).expect("预置");
    for bad in [
        json!({"status":"downloadable"}),
        status("hf-mirror", "downloadable"),
        status("modelscope", "unknown"),
    ] {
        let service = json_server(vec![bad]).await;
        assert!(service.client.inspect(&model).await.is_err());
    }
}

#[tokio::test]
async fn transport_errors_and_timeouts_are_not_mocked() {
    let service = server(
        vec![(503, String::new())],
        Duration::ZERO,
        Duration::from_secs(1),
    )
    .await;
    assert!(collect(&service.client, "").await.is_err());
    let service = server(
        vec![(200, "[]".into())],
        Duration::from_millis(200),
        Duration::from_millis(20),
    )
    .await;
    assert!(collect(&service.client, "").await.is_err());
}

#[test]
fn main_service_address_and_configuration_are_validated() {
    for (host, expected) in [
        ("127.0.0.1", "http://127.0.0.1:3210"),
        ("0.0.0.0", "http://127.0.0.1:3210"),
        ("::", "http://[::1]:3210"),
        ("[::]", "http://[::1]:3210"),
        ("::1", "http://[::1]:3210"),
        ("gateway.local", "http://gateway.local:3210"),
    ] {
        assert_eq!(server_base(host, 3210), expected);
        assert!(CatalogClient::new(expected, Duration::from_secs(1)).is_ok());
    }
    for base in [
        "file:///tmp/models",
        "not a url",
        "http://user:password@localhost",
        "http://localhost:not-a-port",
    ] {
        assert!(CatalogClient::new(base, Duration::from_secs(1)).is_err());
    }
    assert!(CatalogClient::new("http://localhost", Duration::ZERO).is_err());
}

#[test]
fn identity_and_resource_validation_preserve_source_and_revision() {
    let first = catalog_selection();
    let mut other = first.clone();
    other.version.source = "hf-mirror".into();
    assert_ne!(first.id(), other.id());
    other = first.clone();
    other.version.revision = "release".into();
    assert_ne!(first.id(), other.id());
    for (model, source, revision) in [
        ("../x", "modelscope", "master"),
        ("a/b/c", "modelscope", "master"),
        ("a/b", "bad", "main"),
        ("a/b", "hf-mirror", ".."),
        ("a/b", "huggingface", ""),
    ] {
        assert!(!valid_resource(model, source, revision));
    }
    let mut bad = version();
    bad.resource_url = "file:///tmp/model".into();
    assert!(bad.validate().is_err());
}

#[test]
fn preset_accepts_stable_id_for_gguf_repository_but_search_result_does_not() {
    let preset = CatalogModel {
        id: "qwen3-8b".into(),
        name: "Qwen3-8B".into(),
        preset: true,
        model: "Qwen/Qwen3-8B-GGUF".into(),
        source: "modelscope".into(),
        revision: "master".into(),
        resource_url: Some("https://www.modelscope.cn/models/Qwen/Qwen3-8B-GGUF".into()),
        hardware: None,
    };
    assert!(preset.validate().is_ok());

    let search_result = CatalogModel {
        preset: false,
        ..preset
    };
    assert!(search_result.validate().is_err());
}

#[tokio::test]
async fn ndjson_handles_split_unicode_and_final_line_without_newline() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("分块服务");
    let client = CatalogClient::new(
        &format!("http://{}", listener.local_addr().expect("地址")),
        Duration::from_secs(2),
    )
    .expect("客户端");
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("连接");
        let mut request = Vec::new();
        loop {
            let mut bytes = [0; 1024];
            let count = socket.read(&mut bytes).await.expect("请求");
            request.extend_from_slice(&bytes[..count]);
            if count == 0 || request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                break;
            }
        }
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n").await.expect("响应头");
        let mut model = item(false, "modelscope");
        model["name"] = json!("千问模型");
        let body = format!(
            "{}\n{}",
            json!({"done":false,"items":[model.clone()]}),
            json!({"done":true,"type":"models","items":[model]})
        );
        for chunk in body.as_bytes().chunks(7) {
            socket
                .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await
                .expect("分块长度");
            socket.write_all(chunk).await.expect("分块内容");
            socket.write_all(b"\r\n").await.expect("分块结束");
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        socket.write_all(b"0\r\n\r\n").await.expect("响应结束");
    });
    let pages = collect(&client, "qwen").await.expect("完整搜索");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].items[0].name, "千问模型");
    assert!(pages[1].done);
    task.await.expect("分块服务结束");
}

fn catalog_selection() -> CatalogSelection {
    CatalogSelection {
        name: "Qwen3-8B".into(),
        preset: None,
        version: version(),
    }
}

#[expect(
    clippy::disallowed_types,
    reason = "下载接口测试需要在 HTTP 边界构造和解析动态 JSON"
)]
async fn download_server(status: u16, body: serde_json::Value) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("下载接口监听");
    let base = format!("http://{}", listener.local_addr().expect("下载接口地址"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    let body = serde_json::to_string(&body).expect("下载接口响应");
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("下载接口连接");
        let mut bytes = Vec::new();
        let mut content_length = None;
        loop {
            let mut buffer = [0_u8; 1024];
            let count = stream.read(&mut buffer).await.expect("读取下载请求");
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let header_end = header_end + 4;
                if content_length.is_none() {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = headers.lines().find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    });
                }
                if content_length.is_none()
                    || content_length.is_some_and(|length| bytes.len() >= header_end + length)
                {
                    break;
                }
            }
        }
        seen.lock()
            .expect("下载请求锁")
            .push(String::from_utf8(bytes).expect("下载请求文本"));
        let response = format!(
            "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("写入下载响应");
    });
    Server {
        client: CatalogClient::new(&base, Duration::from_secs(2)).expect("测试客户端"),
        requests,
        task,
    }
}

#[expect(
    clippy::disallowed_types,
    reason = "下载任务测试需要在 HTTP 边界构造动态 JSON"
)]
async fn download_task_server(responses: Vec<(u16, serde_json::Value)>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("任务接口监听");
    let base = format!("http://{}", listener.local_addr().expect("任务接口地址"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    let task = tokio::spawn(async move {
        for (status, value) in responses {
            let (mut stream, _) = listener.accept().await.expect("任务接口连接");
            let mut bytes = Vec::new();
            let mut content_length = None;
            loop {
                let mut buffer = [0_u8; 1024];
                let count = stream.read(&mut buffer).await.expect("读取任务请求");
                if count == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(header_end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let header_end = header_end + 4;
                    if content_length.is_none() {
                        let headers = String::from_utf8_lossy(&bytes[..header_end]);
                        content_length = headers.lines().find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        });
                    }
                    if content_length.is_none()
                        || content_length.is_some_and(|length| bytes.len() >= header_end + length)
                    {
                        break;
                    }
                }
            }
            seen.lock()
                .expect("任务请求锁")
                .push(String::from_utf8(bytes).expect("任务请求文本"));
            let body = serde_json::to_string(&value).expect("任务接口响应");
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("写入任务响应");
        }
    });
    Server {
        client: CatalogClient::new(&base, Duration::from_secs(2)).expect("任务测试客户端"),
        requests,
        task,
    }
}

#[expect(
    clippy::disallowed_types,
    reason = "下载任务测试需要在 HTTP 边界构造动态 JSON"
)]
fn task_response(
    status: &str,
    completed_bytes: u64,
    total_bytes: Option<u64>,
    path: Option<&str>,
    actual_sha256: Option<&str>,
) -> serde_json::Value {
    json!({
        "success": true,
        "data": {
            "task_id": "7",
            "status": status,
            "model": "Qwen/Qwen3-8B",
            "source": "modelscope",
            "revision": "master",
            "variant_id": "Q4_K_M.gguf",
            "completed_bytes": completed_bytes,
            "total_bytes": total_bytes,
            "download_speed": 12,
            "error": null,
            "local_path": path,
            "actual_sha256": actual_sha256
        }
    })
}

fn download_request(checksum: &str) -> InitializationRequest {
    let variant = ModelVariant {
        id: "Q4_K_M.gguf".into(),
        quantization: "Q4_K_M".into(),
        runtime: "llama.cpp".into(),
        artifact_source: "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf"
            .into(),
        expected_sha256: checksum.into(),
        ram_bytes: 1,
        vram_bytes: 0,
        disk_bytes: 1,
    };
    let selection = catalog_selection();
    InitializationRequest {
        run_id: 1,
        model: ChatModel {
            id: selection.id(),
            name: selection.name.clone(),
            description: String::new(),
            updated_at: None,
            variants: vec![variant.clone()],
            catalog_selection: Some(selection),
        },
        variant,
    }
}

const DOWNLOAD_DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[tokio::test]
#[expect(
    clippy::disallowed_types,
    reason = "下载接口测试需要检查实际提交的 JSON 字段"
)]
async fn download_model_posts_selected_artifact_and_returns_prepared_model() {
    let service = download_server(
        200,
        task_response(
            "succeeded",
            10,
            Some(10),
            Some("/node/models/modelscope/Qwen_Qwen3-8B/master/Q4_K_M.gguf"),
            Some(DOWNLOAD_DIGEST),
        ),
    )
    .await;
    let request = download_request(DOWNLOAD_DIGEST);
    let mut events = Vec::new();
    let prepared = service
        .client
        .download_model(&request, |event| events.push(event))
        .await
        .expect("Node 下载响应成功");

    assert_eq!(prepared.model, request.model);
    assert_eq!(prepared.variant, request.variant);
    assert_eq!(prepared.actual_sha256, DOWNLOAD_DIGEST);
    assert_eq!(
        prepared.local_path,
        "/node/models/modelscope/Qwen_Qwen3-8B/master/Q4_K_M.gguf"
    );
    assert_eq!(
        events,
        vec![
            PreparationEvent::DownloadTaskCreated {
                task_id: "7".into(),
            },
            PreparationEvent::Downloaded,
            PreparationEvent::Verified {
                expected: DOWNLOAD_DIGEST.into(),
                actual: DOWNLOAD_DIGEST.into(),
                path: prepared.local_path.clone(),
            },
        ]
    );

    let raw = service.requests.lock().expect("下载请求").join("");
    assert!(raw.starts_with("POST /api/node/models/download"));
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("下载请求体");
    let body: serde_json::Value = serde_json::from_str(body).expect("下载 JSON");
    assert_eq!(body["model"], "Qwen/Qwen3-8B");
    assert_eq!(body["source"], "modelscope");
    assert_eq!(body["revision"], "master");
    assert_eq!(body["variant_id"], "Q4_K_M.gguf");
    assert_eq!(
        body["artifact_uri"],
        "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf"
    );
    assert_eq!(body["checksum"], DOWNLOAD_DIGEST);
}

#[tokio::test]
async fn download_model_polls_progress_and_verification_until_succeeded() {
    let service = download_task_server(vec![
        (202, task_response("queued", 0, Some(10), None, None)),
        (200, task_response("downloading", 5, Some(10), None, None)),
        (200, task_response("verifying", 10, Some(10), None, None)),
        (
            200,
            task_response(
                "succeeded",
                10,
                Some(10),
                Some("/node/models/model.gguf"),
                Some(DOWNLOAD_DIGEST),
            ),
        ),
    ])
    .await;
    let selected = download_request(DOWNLOAD_DIGEST);
    let mut chat = super::super::state::ChatState::default();
    let run_id = chat
        .accept_catalog(
            selected.model.catalog_selection.expect("目录选择"),
            VersionStatus::Downloadable,
            vec![selected.variant],
        )
        .expect("启动目录模型准备");
    chat.finish_hardware(
        run_id,
        Ok(Hardware {
            ram_bytes: Some(100),
            disk_bytes: Some(100),
            gpus: Some(Vec::new()),
            ..Hardware::default()
        }),
    );
    assert!(chat.choose_variant(run_id, "Q4_K_M.gguf"));
    let request = chat.initialization_request(run_id).expect("下载快照");
    let mut events = Vec::new();
    let prepared = service
        .client
        .download_model(&request, |event| {
            // 两次轮询跨过下载完成点时，真实状态机也必须能接收校验边界。
            chat.preparation_event(run_id, event.clone());
            events.push(event);
        })
        .await
        .expect("轮询下载成功");

    let preparation = chat.preparation().expect("准备卡片");
    assert_eq!(preparation.stage, PreparationStage::ArtifactReady);
    assert!(preparation.error.is_none());
    assert_eq!(preparation.completed_bytes, 10);
    assert_eq!(preparation.total_bytes, Some(10));
    assert_eq!(
        preparation.local_path.as_deref(),
        Some("/node/models/model.gguf")
    );
    assert_eq!(preparation.actual_digest.as_deref(), Some(DOWNLOAD_DIGEST));
    assert_eq!(prepared.actual_sha256, DOWNLOAD_DIGEST);
    assert_eq!(
        events,
        vec![
            PreparationEvent::DownloadTaskCreated {
                task_id: "7".into(),
            },
            PreparationEvent::DownloadProgress {
                completed: 0,
                total: Some(10),
            },
            PreparationEvent::DownloadProgress {
                completed: 5,
                total: Some(10),
            },
            PreparationEvent::Downloaded,
            PreparationEvent::Verified {
                expected: DOWNLOAD_DIGEST.into(),
                actual: DOWNLOAD_DIGEST.into(),
                path: "/node/models/model.gguf".into(),
            },
        ]
    );
    let requests = service.requests.lock().expect("轮询请求");
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with("POST /api/node/models/download"));
    assert!(requests[1].starts_with("GET /api/node/models/download/7"));
    assert!(requests[2].starts_with("GET /api/node/models/download/7"));
    assert!(requests[3].starts_with("GET /api/node/models/download/7"));
}

#[tokio::test]
async fn download_model_maps_server_errors_to_chat_errors() {
    for (status, code, kind, operation) in [
        (
            422,
            "checksum_mismatch",
            ErrorKind::VerificationFailed,
            Operation::Initialize(PreparationStage::Verifying),
        ),
        (
            502,
            "download_failed",
            ErrorKind::ExecutionFailed,
            Operation::Initialize(PreparationStage::Downloading),
        ),
        (
            503,
            "download_service_unavailable",
            ErrorKind::ExecutionFailed,
            Operation::Initialize(PreparationStage::Downloading),
        ),
    ] {
        let service = download_server(
            status,
            json!({
                "success": false,
                "error": {"code": code, "message": "测试错误"}
            }),
        )
        .await;
        let error = service
            .client
            .download_model(&download_request(DOWNLOAD_DIGEST), |_| {})
            .await
            .expect_err("下载错误应返回 ChatError");
        assert_eq!(error.kind, kind);
        assert_eq!(error.operation, operation);
        assert!(error.reason.contains(code));
    }
}

#[tokio::test]
async fn cancel_download_task_requires_matching_cancelled_snapshot() {
    let service = download_server(200, task_response("cancelled", 10, Some(10), None, None)).await;
    service
        .client
        .cancel_download_task("7")
        .await
        .expect("取消快照应被接受");

    let service = download_server(
        200,
        task_response(
            "succeeded",
            10,
            Some(10),
            Some("/node/model.gguf"),
            Some(DOWNLOAD_DIGEST),
        ),
    )
    .await;
    let error = service
        .client
        .cancel_download_task("7")
        .await
        .expect_err("非 cancelled 快照不得伪装为取消成功");
    assert_eq!(error.kind, ErrorKind::InvalidResult);

    let service = download_server(200, task_response("cancelled", 10, Some(10), None, None)).await;
    let error = service
        .client
        .cancel_download_task("8")
        .await
        .expect_err("任务身份不匹配必须拒绝");
    assert_eq!(error.kind, ErrorKind::InvalidResult);
}
#[tokio::test]
async fn download_model_rejects_identity_mismatch_in_success_response() {
    let service = download_server(
        200,
        json!({
            "success": true,
            "data": {
                "task_id": "7",
                "status": "succeeded",
                "model": "Other/Model",
                "source": "modelscope",
                "revision": "master",
                "variant_id": "Q4_K_M.gguf",
                "completed_bytes": 10,
                "total_bytes": 10,
                "download_speed": 0,
                "error": null,
                "local_path": "/node/models/model.gguf",
                "actual_sha256": DOWNLOAD_DIGEST
            }
        }),
    )
    .await;
    let error = service
        .client
        .download_model(&download_request(DOWNLOAD_DIGEST), |_| {})
        .await
        .expect_err("不匹配的下载结果必须拒绝");
    assert_eq!(error.kind, ErrorKind::InvalidResult);
    assert_eq!(
        error.operation,
        Operation::Initialize(PreparationStage::Verifying)
    );
}

#[test]
fn download_task_mapping_validates_progress_identity_and_terminal_fields() {
    let request = download_request(DOWNLOAD_DIGEST);
    let queued = DownloadTaskData {
        task_id: "7".into(),
        status: "queued".into(),
        model: "Qwen/Qwen3-8B".into(),
        source: "modelscope".into(),
        revision: "master".into(),
        variant_id: "Q4_K_M.gguf".into(),
        completed_bytes: 0,
        total_bytes: Some(10),
        download_speed: 0,
        error: None,
        local_path: None,
        actual_sha256: None,
    };
    assert_eq!(
        map_download_task(queued.clone(), &request),
        Ok(queued.clone())
    );

    let mut invalid_progress = queued.clone();
    invalid_progress.completed_bytes = 11;
    assert!(map_download_task(invalid_progress, &request).is_err());

    let mut invalid_identity = queued;
    invalid_identity.model = "Other/Model".into();
    assert!(map_download_task(invalid_identity, &request).is_err());

    let mut missing_success_fields = DownloadTaskData {
        task_id: "7".into(),
        status: "succeeded".into(),
        model: "Qwen/Qwen3-8B".into(),
        source: "modelscope".into(),
        revision: "master".into(),
        variant_id: "Q4_K_M.gguf".into(),
        completed_bytes: 10,
        total_bytes: Some(10),
        download_speed: 0,
        error: None,
        local_path: None,
        actual_sha256: None,
    };
    missing_success_fields.actual_sha256 = Some(DOWNLOAD_DIGEST.into());
    assert!(map_download_task(missing_success_fields, &request).is_err());
}

#[test]
fn task_ids_accept_only_decimal_i64_values() {
    assert!(valid_task_id("1"));
    assert!(valid_task_id("9223372036854775807"));
    assert!(!valid_task_id(""));
    assert!(!valid_task_id("-1"));
    assert!(!valid_task_id("1x"));
    assert!(!valid_task_id("9223372036854775808"));
}

#[allow(clippy::disallowed_types, reason = "测试必须构造部署响应及其非法变体")]
fn deploy_snapshot(status: &str) -> serde_json::Value {
    json!({"model_id":"model", "status":status, "stage":null, "state":null,
        "endpoint":null, "pid":null, "last_heartbeat_unix_ms":null,
        "error":null, "events":[]})
}

#[expect(
    clippy::disallowed_types,
    reason = "部署协议替身需要构造动态 JSON，不参与生产业务逻辑"
)]
fn route_snapshot(route_status: &str) -> serde_json::Value {
    let mut snapshot = deploy_snapshot("ready");
    snapshot["route_status"] = json!(route_status);
    // 测试不绑定任何固定持久化 ID；真实 ID 由 Router 创建并返回。
    snapshot["route_attachment_id"] = json!(null);
    snapshot["capacity"] = json!({
        "total_slots": 1,
        "free_slots": if route_status == "saturated" { 0 } else { 1 },
        "observed_at_unix_ms": 1790000000000u64
    });
    snapshot["last_heartbeat_unix_ms"] = json!(1790000000000u64);
    snapshot
}

#[tokio::test]
async fn deployment_route_status_validates_model_and_stable_status() {
    for status in [
        "unroutable",
        "routable",
        "saturated",
        "quarantined",
        "lost",
        "detaching",
        "detached",
        "failed",
    ] {
        let service = json_server(vec![route_snapshot(status)]).await;
        assert_eq!(
            service
                .client
                .deployment_route_status("model")
                .await
                .expect("合法 Router 状态"),
            status
        );
    }

    let mut mismatch = route_snapshot("routable");
    mismatch["model_id"] = json!("other");
    let service = json_server(vec![mismatch]).await;
    let error = service
        .client
        .deployment_route_status("model")
        .await
        .expect_err("模型 ID 不一致");
    assert_eq!(error.kind, ErrorKind::InvalidResult);

    let service = json_server(vec![deploy_snapshot("ready")]).await;
    let error = service
        .client
        .deployment_route_status("model")
        .await
        .expect_err("缺少 Router 状态");
    assert_eq!(error.kind, ErrorKind::InvalidResult);

    let mut unknown = route_snapshot("routable");
    unknown["route_status"] = json!("unknown");
    let service = json_server(vec![unknown]).await;
    let error = service
        .client
        .deployment_route_status("model")
        .await
        .expect_err("未知 Router 状态");
    assert_eq!(error.kind, ErrorKind::InvalidResult);
}

#[tokio::test]
async fn deployment_route_status_polls_unroutable_until_capacity_is_ready() {
    let service = json_server(vec![
        deploy_snapshot("ready"),
        route_snapshot("unroutable"),
        route_snapshot("unroutable"),
        route_snapshot("routable"),
    ])
    .await;
    service
        .client
        .deploy_model("model.gguf")
        .await
        .expect("部署就绪");
    let status = service
        .client
        .poll_deployment_route_status("model")
        .await
        .expect("Router 状态查询");
    assert_eq!(status, "routable");
    assert_eq!(service.requests.lock().expect("请求记录").len(), 4);
}

#[tokio::test]
async fn deployment_route_status_query_failure_does_not_cancel_deployment() {
    let service = server(
        vec![
            (200, deploy_snapshot("ready").to_string()),
            (503, "service unavailable".into()),
        ],
        Duration::ZERO,
        Duration::from_secs(2),
    )
    .await;
    service
        .client
        .deploy_model("model.gguf")
        .await
        .expect("部署应保持成功");
    let error = service
        .client
        .poll_deployment_route_status("model")
        .await
        .expect_err("状态查询失败");
    assert_eq!(error.kind, ErrorKind::ExecutionFailed);
}

#[tokio::test]
async fn deploy_http_posts_original_path_and_maps_terminal_states() {
    let path = " C:\\模型\\model.gguf ";
    let service = json_server(vec![deploy_snapshot("ready")]).await;
    service.client.deploy_model(path).await.expect("部署就绪");
    {
        let calls = service.requests.lock().expect("请求记录");
        assert_eq!(calls.len(), 1);
        assert!(calls[0].starts_with("POST /console/api/node/deployments "));
        let body = calls[0].split_once("\r\n\r\n").expect("请求体").1;
        let body: std::collections::BTreeMap<String, String> =
            serde_json::from_str(body).expect("请求 JSON");
        assert_eq!(body["model_path"], path);
    }

    for (status, stage) in [
        ("failed", PreparationStage::Deploying),
        ("unhealthy", PreparationStage::HealthChecking),
    ] {
        let mut snapshot = deploy_snapshot(status);
        snapshot["error"] = json!("远端故障");
        let service = json_server(vec![snapshot]).await;
        let error = service
            .client
            .deploy_model(path)
            .await
            .expect_err("部署失败");
        assert_eq!(error.kind, ErrorKind::ExecutionFailed);
        assert_eq!(error.operation, Operation::Initialize(stage));
        assert_eq!(error.reason, "远端故障");
    }
}

#[tokio::test]
async fn deploy_http_rejects_invalid_snapshots_and_http_errors() {
    let mut invalid = vec!["not json".to_string()];
    for field in [
        "model_id",
        "status",
        "stage",
        "state",
        "endpoint",
        "pid",
        "last_heartbeat_unix_ms",
        "error",
        "events",
    ] {
        let mut snapshot = deploy_snapshot("ready");
        snapshot.as_object_mut().expect("对象").remove(field);
        invalid.push(snapshot.to_string());
    }
    let mut blank = deploy_snapshot("ready");
    blank["model_id"] = json!("");
    invalid.push(blank.to_string());
    let mut bad_events = deploy_snapshot("ready");
    bad_events["events"] = json!({});
    invalid.push(bad_events.to_string());
    for status in ["queued", "deploying", "unknown", ""] {
        invalid.push(deploy_snapshot(status).to_string());
    }
    for body in invalid {
        let service = server(vec![(200, body)], Duration::ZERO, Duration::from_secs(2)).await;
        let error = service
            .client
            .deploy_model("model")
            .await
            .expect_err("非法快照");
        assert_eq!(error.kind, ErrorKind::InvalidResult);
        assert_eq!(
            error.operation,
            Operation::Initialize(PreparationStage::Deploying)
        );
    }
    for (status, body) in [
        (400, "{\"error\":\"路径无效\"}"),
        (503, "bad"),
        (418, "bad"),
        (302, "bad"),
    ] {
        let service = server(
            vec![(status, body.into())],
            Duration::ZERO,
            Duration::from_secs(2),
        )
        .await;
        let error = service
            .client
            .deploy_model("model")
            .await
            .expect_err("HTTP 错误");
        assert_eq!(error.kind, ErrorKind::ExecutionFailed);
        assert_eq!(
            error.operation,
            Operation::Initialize(PreparationStage::Deploying)
        );
        assert!(!error.reason.is_empty());
        if status == 400 {
            assert_eq!(error.reason, "路径无效");
        }
        assert_eq!(service.requests.lock().expect("请求记录").len(), 1);
    }
}

#[tokio::test]
async fn deploy_http_rejects_bad_paths_and_times_out() {
    let service = server(
        vec![(200, deploy_snapshot("ready").to_string())],
        Duration::from_millis(300),
        Duration::from_millis(30),
    )
    .await;
    for path in ["", "a\0b"] {
        let error = service
            .client
            .deploy_model(path)
            .await
            .expect_err("非法路径");
        assert_eq!(error.kind, ErrorKind::InvalidParameter);
    }
    assert!(service.requests.lock().expect("请求记录").is_empty());
    let error = service
        .client
        .deploy_model("model")
        .await
        .expect_err("超时");
    assert_eq!(error.kind, ErrorKind::Timeout);
    assert_eq!(
        error.operation,
        Operation::Initialize(PreparationStage::Deploying)
    );
}

#[tokio::test]
async fn deploy_http_does_not_follow_redirects() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("监听");
    let base = format!("http://{}", listener.local_addr().expect("地址"));
    let location = format!("{base}/redirected");
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("部署请求");
        let mut bytes = [0; 2048];
        let count = stream.read(&mut bytes).await.expect("读取请求");
        assert!(String::from_utf8_lossy(&bytes[..count])
            .starts_with("POST /console/api/node/deployments "));
        let response = format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        stream
            .write_all(response.as_bytes())
            .await
            .expect("返回重定向");
        tokio::time::timeout(Duration::from_millis(150), listener.accept())
            .await
            .is_err()
    });
    let client = CatalogClient::new(&base, Duration::from_secs(1)).expect("客户端");
    let error = client
        .deploy_model("model")
        .await
        .expect_err("不接受重定向");
    assert_eq!(error.kind, ErrorKind::ExecutionFailed);
    assert!(task.await.expect("测试任务"));
}

#[tokio::test]
async fn deploy_http_connection_failure_is_not_connected() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("监听");
    let base = format!("http://{}", listener.local_addr().expect("地址"));
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("连接");
        drop(stream);
    });
    let client = CatalogClient::new(&base, Duration::from_secs(2)).expect("客户端");
    let error = client
        .deploy_model("model.gguf")
        .await
        .expect_err("连接失败");
    assert_eq!(error.kind, ErrorKind::NotConnected);
    assert_eq!(
        error.operation,
        Operation::Initialize(PreparationStage::Deploying)
    );
    server.await.expect("服务端任务");
}

#[tokio::test]
async fn verified_catalog_variant_is_used_without_posting_a_download() {
    let mut request = download_request(DOWNLOAD_DIGEST);
    let v = &request.variant;
    let response = json!({"status":"downloadable", "model":"Qwen/Qwen3-8B", "source":"modelscope", "revision":"master", "resource_url":version().resource_url, "artifact_status":"downloaded", "downloaded":true, "local_path":"/node/m.gguf", "actual_sha256":DOWNLOAD_DIGEST,
        "variants":[{"id":v.id,"quantization":v.quantization,"runtime":v.runtime,"artifact_source":v.artifact_source,"expected_sha256":DOWNLOAD_DIGEST,"ram_bytes":1,"vram_bytes":0,"disk_bytes":1,"downloaded":true,"artifact_status":"downloaded","local_path":"/node/m.gguf","actual_sha256":DOWNLOAD_DIGEST}]});
    let service = json_server(vec![response]).await;
    let model: CatalogModel = serde_json::from_value(item(true, "modelscope")).expect("目录项");
    let inspection = service.client.inspect(&model).await.expect("已下载工件");
    let mut other = v.clone();
    other.artifact_source.push_str(".other");
    assert!(inspection.version.verified_path(&other).is_none());
    request
        .model
        .catalog_selection
        .as_mut()
        .expect("目录选择")
        .version = inspection.version;
    let mut events = Vec::new();
    let prepared = service
        .client
        .download_model(&request, |e| events.push(e))
        .await
        .expect("复用工件");
    assert!(prepared.is_valid());
    assert_eq!(prepared.local_path, "/node/m.gguf");
    assert!(matches!(
        events.as_slice(),
        [
            PreparationEvent::Downloaded,
            PreparationEvent::Verified { .. }
        ]
    ));
    assert_eq!(service.requests.lock().expect("请求记录").len(), 1);
}
