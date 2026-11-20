//! 默认后端本身的目录、三平台硬件与占位路径回归，不启动计时或外部操作。
use super::*;

#[test]
fn local_platform_fixtures_have_consistent_resources_and_placeholder_paths() {
    for (platform, prefix, cpu_fragment) in [
        (Platform::Windows, r"C:\BurnCloud\models\", "x64"),
        (Platform::MacOs, "/Users/user/BurnCloud/models/", "Apple"),
        (Platform::Linux, "/home/user/BurnCloud/models/", "x64"),
    ] {
        let detected = hardware(platform);
        assert_eq!(detected.platform, Some(platform));
        assert!(detected
            .cpu
            .as_deref()
            .expect("固定 CPU")
            .contains(cpu_fragment));
        assert_eq!(detected.ram_bytes, Some(24 * GIB));
        assert_eq!(detected.max_vram(), Some(8 * GIB));
        assert_eq!(detected.disk_bytes, Some(128 * GIB));
        let path = example_path(platform, "qwen3:8b", "q4");
        assert!(path.starts_with(prefix));
        assert!(path.ends_with("qwen3-8b-q4.gguf"));
    }
}

#[test]
fn local_catalog_contract_has_artifact_identity_and_one_resource_failure_sample() {
    let models = catalog();
    assert_eq!(models.len(), 14);
    let hardware = hardware(Platform::Windows);
    for model in models {
        assert!(!model.id.is_empty());
        assert!(model.updated_at.is_some());
        assert!(!model.variants.is_empty());
        assert_eq!(
            model.variants.iter().any(|variant| variant.fits(&hardware)),
            model.id != "qwen3.6-max"
        );
        for variant in model.variants {
            assert!(valid_digest(&variant.expected_sha256));
            assert!(!variant.runtime.is_empty());
            assert!(!variant.artifact_source.is_empty());
        }
    }
}

#[tokio::test]
async fn local_search_matches_names_ids_descriptions_and_trims_ascii_case() {
    for (query, expected) in [
        (" QWEN3:8B ", Some("qwen3:8b")),
        ("代码编写", Some("qwen2.5-coder:7b")),
        ("日常问答", Some("llama3.2:1b")),
        ("不存在的模型", None),
    ] {
        let models = LocalBackend.search(query.into()).await.expect("本地搜索");
        assert_eq!(models.len(), usize::from(expected.is_some()));
        assert_eq!(models.first().map(ChatModel::id), expected);
    }
    let models = LocalBackend.search("   ".into()).await.expect("空查询");
    assert_eq!(models, catalog());
}

#[tokio::test]
async fn backend_deploy_default_and_client_initialization_errors_are_explicit() {
    for path in ["", "a\0b"] {
        let error = LocalBackend
            .deploy_model(path.into())
            .await
            .expect_err("非法路径");
        assert_eq!(error.kind, ErrorKind::InvalidParameter);
    }
    let error = LocalBackend
        .deploy_model("model.gguf".into())
        .await
        .expect_err("本地替身不部署");
    assert_eq!(error.kind, ErrorKind::NotConnected);
    assert_eq!(
        error.operation,
        Operation::Initialize(PreparationStage::Deploying)
    );
    let backend = HttpBackend {
        local: LocalBackend,
        hardware_client: Err(error.clone()),
    };
    let received = backend
        .deploy_model("model.gguf".into())
        .await
        .expect_err("客户端错误");
    assert_eq!(received, error);
}

#[tokio::test]
async fn http_backend_deploy_delegates_and_preserves_health_error() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("监听");
    let base = format!("http://{}", listener.local_addr().expect("地址"));
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("请求");
        let mut bytes = [0; 2048];
        let count = stream.read(&mut bytes).await.expect("读取");
        assert!(String::from_utf8_lossy(&bytes[..count])
            .starts_with("POST /console/api/node/deployments "));
        let body = r#"{"model_id":"m","status":"unhealthy","stage":null,"state":null,"endpoint":null,"pid":null,"last_heartbeat_unix_ms":null,"error":"健康异常","events":[]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.expect("响应");
    });
    let backend = HttpBackend {
        local: LocalBackend,
        hardware_client: CatalogClient::new(&base, std::time::Duration::from_secs(2)),
    };
    let error = backend
        .deploy_model("model.gguf".into())
        .await
        .expect_err("健康失败");
    assert_eq!(error.kind, ErrorKind::ExecutionFailed);
    assert_eq!(
        error.operation,
        Operation::Initialize(PreparationStage::HealthChecking)
    );
    assert_eq!(error.reason, "健康异常");
    task.await.expect("测试任务");
}
