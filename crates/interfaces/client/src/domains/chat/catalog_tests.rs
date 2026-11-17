//! 使用真实本地 HTTP 响应验证目录契约，不访问公网。
use super::super::model::ChatModel;
use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
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
        name: "Qwen3-8B".into(),
        model: "Qwen/Qwen3-8B".into(),
        source: "modelscope".into(),
        revision: "master".into(),
        resource_url: "https://modelscope.cn/models/Qwen/Qwen3-8B/files?Revision=master".into(),
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

#[allow(clippy::disallowed_types, reason = "测试需要构造完整状态协议")]
fn status(source: &str, status: &str) -> serde_json::Value {
    json!({"status":status,"model":"Qwen/Qwen3-8B","source":source,
        "revision":if source == "modelscope" {"master"} else {"main"}, "resource_url":"https://example.com/models/Qwen/Qwen3-8B"})
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
async fn presets_load_from_main_service_with_flat_resources() {
    let service = json_server(vec![json!([item(true, "modelscope")])]).await;
    let pages = collect(&service.client, " ").await.expect("预置列表");
    assert_eq!(pages.len(), 1);
    assert!(pages[0].done && pages[0].items[0].preset);
    assert_eq!(pages[0].items[0].model, "Qwen/Qwen3-8B");
    let calls = service.requests.lock().expect("请求记录");
    assert_eq!(calls.len(), 1);
    assert!(calls[0].starts_with("GET /api/models/presets"));
    assert!(!calls[0].contains("page="));
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
            let (resource, state) = service.client.inspect(&model).await.expect("状态");
            assert_eq!(state, expected);
            assert_eq!(resource.expect("资源").source, source);
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

async fn file_server(body: &'static [u8]) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("测试监听");
    let base = format!("http://{}", listener.local_addr().expect("地址"));
    let task = tokio::spawn(async move {
        loop {
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
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            if let Err(error) = stream.write_all(head.as_bytes()).await {
                eprintln!("测试文件连接已关闭：{error}");
                continue;
            }
            if let Err(error) = stream.write_all(body).await {
                // 超长测试中客户端提前中止，服务端允许写回失败。
                eprintln!("测试文件连接已关闭：{error}");
            }
        }
    });
    (base, task)
}
fn download_request(url: String, sha256: String, size: u64) -> InitializationRequest {
    let variant = ModelVariant {
        id: "q4".into(),
        quantization: "Q4_K_M".into(),
        runtime: "llama.cpp".into(),
        artifact_source: url,
        expected_sha256: sha256,
        ram_bytes: 1,
        vram_bytes: 0,
        disk_bytes: size,
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
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                found.extend(files(&path));
            } else {
                found.push(path);
            }
        }
    }
    found
}
const DIGEST: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const BODY: &[u8] = b"burncloud-model-bytes";
fn body_digest() -> String {
    format!("sha256:{:x}", Sha256::digest(BODY))
}
#[tokio::test]
async fn download_streams_verifies_and_stops_before_deployment() {
    let (base, task) = file_server(BODY).await;
    let root = tempfile::tempdir().expect("临时目录");
    let request = download_request(
        format!("{base}/files/q4.gguf"),
        body_digest(),
        BODY.len() as u64,
    );
    let mut events = Vec::new();
    let path = download(root.path(), &request, |event| events.push(event))
        .await
        .expect("下载成功");
    task.abort();
    let expected = root
        .path()
        .join("modelscope")
        .join("Qwen_Qwen3-8B")
        .join("master")
        .join("q4-q4.gguf");
    assert_eq!(PathBuf::from(&path), expected);
    assert_eq!(std::fs::read(&expected).expect("文件"), BODY);
    assert_eq!(files(root.path()), vec![expected]);
    assert!(matches!(
        events.first(),
        Some(PreparationEvent::DownloadProgress { .. })
    ));
    assert_eq!(
        events.last(),
        Some(&PreparationEvent::Verified {
            expected: body_digest(),
            actual: body_digest(),
            path: path.clone(),
        })
    );
    assert_eq!(events[events.len() - 2], PreparationEvent::Downloaded);
    let stop = deployment_missing(&path);
    assert_eq!(stop.kind, ErrorKind::NotConnected);
    assert!(stop.reason.contains("尚未接入部署") && stop.reason.contains(&path));
}
#[tokio::test]
async fn download_rejects_digest_and_size_mismatches_without_leftovers() {
    let (base, task) = file_server(BODY).await;
    let len = BODY.len() as u64;
    let cases = [
        (DIGEST.to_string(), len),
        (body_digest(), len - 1),
        (body_digest(), len + 1),
    ];
    for (sha256, size) in cases {
        let root = tempfile::tempdir().expect("临时目录");
        let request = download_request(format!("{base}/q4.gguf"), sha256, size);
        let mut events = Vec::new();
        let error = download(root.path(), &request, |event| events.push(event))
            .await
            .expect_err("校验失败");
        assert_eq!(error.kind, ErrorKind::VerificationFailed);
        assert!(files(root.path()).is_empty(), "不得残留文件");
        assert!(!events
            .iter()
            .any(|event| matches!(event, PreparationEvent::Verified { .. })));
    }
    task.abort();
}
