//! 使用本地站点替身验证统一接口，不访问公网。

#![allow(
    clippy::disallowed_types,
    reason = "HTTP 黑盒测试需要构造和检查站点协议响应"
)]

use super::*;
use axum::{
    body::to_bytes,
    extract::Query,
    http::{Request, Uri},
};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};
use tower::ServiceExt;

type Calls = Arc<Mutex<Vec<String>>>;

struct Harness {
    router: Router,
    state: Arc<AppState>,
    calls: Calls,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn upstream(
    State(calls): State<Calls>,
    uri: Uri,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    calls.lock().expect("请求记录锁").push(uri.to_string());
    let path = uri.path();
    let is_modelscope_tree = path.contains("/api/v1/models/") && path.ends_with("/repo/files");
    let is_huggingface_tree = path.contains("/api/models/") && path.contains("/tree/");
    if is_modelscope_tree || is_huggingface_tree {
        if path.contains("/missing/") {
            return StatusCode::NOT_FOUND.into_response();
        }
        if path.contains("/denied/") {
            return StatusCode::BAD_GATEWAY.into_response();
        }
        if path.contains("/empty/") {
            return if is_modelscope_tree {
                Json(json!({"Code":200,"Data":{"Files":[{"Path":"model.safetensors","Size":3,"Sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","Type":"blob"}]}})).into_response()
            } else {
                Json(json!([{"type":"file","path":"model.safetensors","size":3,"lfs":{"oid":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}])).into_response()
            };
        }
        return if is_modelscope_tree {
            Json(json!({"Code":200,"Data":{"Files":[{"Path":"model-Q4_K_M.gguf","Size":3,"Sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","Type":"blob"}]}})).into_response()
        } else {
            Json(json!([{"type":"file","path":"model-Q4_K_M.gguf","size":3,"lfs":{"oid":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}])).into_response()
        };
    }
    if !params.contains_key("search") {
        let status = if path.contains("/missing/") {
            StatusCode::NOT_FOUND
        } else if path.contains("/denied/") {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::OK
        };
        return status.into_response();
    }
    let query = params["search"].as_str();
    let ms = path.starts_with("/ms/");
    let hf = path.starts_with("/hf/");
    if query == "timeout" && !hf {
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    if query == "normal" && ms {
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    if query == "failed" || (query == "partial" && ms) {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    if query == "malformed" {
        return Json(json!({"unexpected":true})).into_response();
    }
    let models = if query == "empty" {
        json!([])
    } else {
        json!([{"id":"Qwen/Qwen3-8B", "title":if ms {"国内模型"} else {"镜像模型"}, "display_name":"国内模型"}])
    };
    if ms {
        Json(json!({"success":true,"data":{"models":models}})).into_response()
    } else {
        Json(models).into_response()
    }
}

impl Harness {
    async fn new() -> Self {
        let calls = Calls::default();
        let mock = Router::new().fallback(upstream).with_state(calls.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("监听测试站点");
        let base = format!("http://{}", listener.local_addr().expect("站点地址"));
        let config = || Config {
            modelscope: format!("{base}/ms"),
            hf_mirror: format!("{base}/mirror"),
            huggingface: format!("{base}/hf"),
            search_timeout: Duration::from_millis(250),
            probe_timeout: Duration::from_millis(250),
        };
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("测试客户端");
        let router = app(config(), client.clone(), None);
        let state = Arc::new(AppState {
            config: config(),
            client,
            database: None,
        });
        let task = tokio::spawn(async move {
            axum::serve(listener, mock).await.expect("测试站点运行");
        });
        Self {
            router,
            state,
            calls,
            task,
        }
    }

    async fn get(&self, path: &str) -> (StatusCode, String) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("测试请求"),
            )
            .await
            .expect("测试路由");
        let status = response.status();
        let body = to_bytes(response.into_body(), 1_000_000)
            .await
            .expect("读取响应");
        (status, String::from_utf8(body.to_vec()).expect("响应编码"))
    }

    async fn json(&self, path: &str) -> Value {
        let (status, body) = self.get(path).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        serde_json::from_str(&body).expect("JSON 响应")
    }

    async fn search(&self, query: &str) -> Vec<Value> {
        let (status, body) = self.get(&format!("/api/models/search?q={query}")).await;
        assert_eq!(status, StatusCode::OK);
        body.lines()
            .map(|line| serde_json::from_str(line).expect("NDJSON 快照"))
            .collect()
    }
}

#[tokio::test]
async fn presets_keep_spreadsheet_rows_links_and_hardware() {
    let h = Harness::new().await;
    let presets = h.json("/api/models/presets").await;
    let names: Vec<_> = presets
        .as_array()
        .expect("预置列表")
        .iter()
        .map(|p| p["name"].as_str().expect("名称"))
        .collect();
    assert_eq!(names.len(), 388);
    assert_eq!(
        &names[..3],
        [
            "llama3.1-8B",
            "llama3.1-70B",
            "Meta-Llama-3.1-405b-Instruct"
        ]
    );
    assert_eq!(names[22], "qwen3-8B");
    assert_eq!(names.last(), Some(&"Bespoke-Nimble-9B-v3-GGUF"));
    let items = presets.as_array().expect("列表");
    let mut ids: Vec<_> = items
        .iter()
        .map(|item| item["id"].as_str().expect("标识"))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 388);
    assert_eq!(
        items
            .iter()
            .filter(|item| item["source"] == "modelscope")
            .count(),
        96
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item["source"] == "hf-mirror")
            .count(),
        292
    );
    assert_eq!(
        items[0]["resource_url"],
        "https://www.modelscope.cn/models/LLM-Research/Meta-Llama-3.1-8b-Instruct"
    );
    assert_eq!(
        items[0]["hardware"],
        json!({
            "vram_min":"6.00G", "ram_min":"12.00G", "disk_min":"6.00G",
            "vram_recommended":"8.00G", "ram_recommended":"16.00G", "disk_recommended":"8.00G"
        })
    );
    for item in items {
        assert_eq!(item["preset"], true);
        assert_eq!(
            item["revision"],
            if item["source"] == "modelscope" {
                "master"
            } else {
                "main"
            }
        );
        let link = reqwest::Url::parse(item["resource_url"].as_str().expect("模型链接"))
            .expect("有效链接");
        assert_eq!(link.scheme(), "https");
        let model = item["model"].as_str().expect("仓库");
        assert_eq!(
            link.path()
                .strip_prefix("/models/")
                .unwrap_or_else(|| link.path().trim_start_matches('/')),
            model
        );
        for field in [
            "vram_min",
            "ram_min",
            "disk_min",
            "vram_recommended",
            "ram_recommended",
            "disk_recommended",
        ] {
            assert!(item["hardware"][field]
                .as_str()
                .expect("配置")
                .strip_suffix('G')
                .is_some_and(|number| {
                    let decimals = number.split_once('.').map(|(_, decimals)| decimals);
                    decimals.is_some_and(|decimals| decimals.len() == 2)
                }));
        }
        let bound = presets::find(item["id"].as_str().expect("标识"))
            .expect("有效目录")
            .expect("绑定");
        assert_eq!(bound.resource.model, model);
    }
    assert!(h.calls.lock().expect("请求锁").is_empty());
    let response = routes(Arc::new(
        burncloud_database::Database::new()
            .await
            .expect("测试数据库"),
    ))
    .expect("生产路由")
    .oneshot(
        Request::builder()
            .uri("/api/models/presets")
            .body(Body::empty())
            .expect("请求"),
    )
    .await
    .expect("路由");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn verified_gguf_presets_use_selected_repositories_and_preserve_catalog_metadata() {
    let h = Harness::new().await;
    let items = h
        .json("/api/models/presets")
        .await
        .as_array()
        .expect("预置列表")
        .clone();
    let find = |id: &str| {
        items
            .iter()
            .find(|item| item["id"] == id)
            .unwrap_or_else(|| panic!("缺少预置项：{id}"))
    };

    let qwen3 = find("qwen3-8b");
    assert_eq!(qwen3["id"], "qwen3-8b");
    assert_eq!(qwen3["name"], "qwen3-8B");
    assert_eq!(qwen3["model"], "Qwen/Qwen3-8B-GGUF");
    assert_eq!(qwen3["source"], "modelscope");
    assert_eq!(qwen3["revision"], "master");
    assert_eq!(
        qwen3["resource_url"],
        "https://www.modelscope.cn/models/Qwen/Qwen3-8B-GGUF"
    );
    assert_eq!(
        qwen3["hardware"],
        json!({
            "vram_min":"6.00G", "ram_min":"12.00G", "disk_min":"6.00G",
            "vram_recommended":"8.00G", "ram_recommended":"16.00G", "disk_recommended":"8.00G"
        })
    );

    let llama2_chinese = find("llama2-chinese-13b-chat");
    assert_eq!(llama2_chinese["id"], "llama2-chinese-13b-chat");
    assert_eq!(llama2_chinese["name"], "Llama2-Chinese-13b-Chat");
    assert_eq!(
        llama2_chinese["model"],
        "mradermacher/Llama2-Chinese-13b-Chat-GGUF"
    );
    assert_eq!(llama2_chinese["source"], "hf-mirror");
    assert_eq!(llama2_chinese["revision"], "main");
    assert_eq!(
        llama2_chinese["resource_url"],
        "https://hf-mirror.com/mradermacher/Llama2-Chinese-13b-Chat-GGUF"
    );
    assert_eq!(
        llama2_chinese["hardware"],
        json!({
            "vram_min":"12.00G", "ram_min":"16.00G", "disk_min":"12.00G",
            "vram_recommended":"12.00G", "ram_recommended":"24.00G", "disk_recommended":"12.00G"
        })
    );

    let dolphin = find("dolphin-2_6-phi-2");
    assert_eq!(dolphin["id"], "dolphin-2_6-phi-2");
    assert_eq!(dolphin["name"], "dolphin-2_6-phi-2");
    assert_eq!(dolphin["model"], "TheBloke/dolphin-2_6-phi-2-GGUF");
    assert_eq!(dolphin["source"], "hf-mirror");
    assert_eq!(dolphin["revision"], "main");
    assert_eq!(
        dolphin["resource_url"],
        "https://hf-mirror.com/TheBloke/dolphin-2_6-phi-2-GGUF"
    );
    assert_eq!(
        dolphin["hardware"],
        json!({
            "vram_min":"4.00G", "ram_min":"8.00G", "disk_min":"4.00G",
            "vram_recommended":"4.00G", "ram_recommended":"16.00G", "disk_recommended":"4.00G"
        })
    );

    let jev = find("jev-9b");
    assert_eq!(jev["id"], "jev-9b");
    assert_eq!(jev["name"], "JEV-9B");
    assert_eq!(jev["model"], "prithivMLmods/JEV-9B-GGUF");
    assert_eq!(jev["source"], "modelscope");
    assert_eq!(jev["revision"], "master");
    assert_eq!(
        jev["resource_url"],
        "https://www.modelscope.cn/models/prithivMLmods/JEV-9B-GGUF"
    );
    assert_eq!(
        jev["hardware"],
        json!({
            "vram_min":"6.00G", "ram_min":"12.00G", "disk_min":"6.00G",
            "vram_recommended":"12.00G", "ram_recommended":"16.00G", "disk_recommended":"12.00G"
        })
    );
}

#[tokio::test]
async fn preset_names_and_pending_identities_are_serialized() {
    let h = Harness::new().await;
    let items = h
        .json("/api/models/presets")
        .await
        .as_array()
        .expect("预置列表")
        .clone();
    let find = |id: &str| {
        items
            .iter()
            .find(|item| item["id"] == id)
            .unwrap_or_else(|| panic!("缺少预置 ID：{id}"))
    };

    let command_r = find("c4ai-command-r7b-12-2024");
    assert_eq!(command_r["name"], "command-r-7B");
    assert_eq!(command_r["model"], "CohereForAI/c4ai-command-r7b-12-2024");
    assert_eq!(find("mistral-7b-openorca")["name"], "Mistral-7B-OpenOrca");
    assert_eq!(
        find("olmo-3.1-32b-instruct")["name"],
        "Olmo-3.1-32B-Instruct"
    );
    assert_eq!(find("qwen3-vl-2b-instruct")["name"], "Qwen3-VL-2b-Instruct");

    for id in [
        "nsfw_image_detection",
        "nsfw_image_detection-row-202",
        "nsfw_image_detection-row-203",
        "tiny-random-aya-base",
        "tiny-random-aya-base-row-220",
    ] {
        assert_eq!(find(id)["identity_status"], "pending_validation");
    }
    assert_eq!(
        presets::all()
            .expect("有效目录")
            .iter()
            .filter(|preset| preset.identity_status == presets::IdentityStatus::PendingValidation)
            .count(),
        5
    );
}

#[tokio::test]
async fn status_uses_preset_binding_and_reports_live_artifacts() {
    let h = Harness::new().await;
    assert_eq!(
        h.get("/api/models/status?preset=qwen3-8b&model=evil/override&source=huggingface")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let value = h.json("/api/models/status?preset=qwen3-8b").await;
    assert_eq!(value["status"], "downloadable");
    assert_eq!(value["model"], "Qwen/Qwen3-8B-GGUF");
    assert_eq!(value["source"], "modelscope");
    assert_eq!(value["revision"], "master");
    assert!(value["resource_url"]
        .as_str()
        .expect("资源地址")
        .ends_with("/ms/models/Qwen/Qwen3-8B-GGUF/files?Revision=master"));
    assert_eq!(value["downloadable"], true);
    assert!(value["unavailable_reason"].is_null());
    assert_eq!(value["identity_status"], "verified");
    assert_eq!(value["artifact_status"], "not_downloaded");
    assert_eq!(value["downloaded"], false);
    assert!(value["local_path"].is_null());
    assert!(value["actual_sha256"].is_null());
    assert_eq!(value["variants"].as_array().expect("工件列表").len(), 1);
    assert_eq!(value["variants"][0]["artifact_status"], "not_downloaded");
    assert_eq!(value["variants"][0]["downloaded"], false);
    assert!(value["variants"][0]["local_path"].is_null());
    assert!(value["variants"][0]["actual_sha256"].is_null());
    assert_eq!(value["variants"][0]["id"], "model-Q4_K_M.gguf");
    assert_eq!(value["variants"][0]["quantization"], "Q4_K_M");
    assert_eq!(value["variants"][0]["runtime"], "llama.cpp");
    assert_eq!(value["variants"][0]["file_size_bytes"], 3);
    assert!(value["variants"][0]["artifact_source"]
        .as_str()
        .expect("直接工件地址")
        .contains("/models/Qwen/Qwen3-8B-GGUF/resolve/master/model-Q4_K_M.gguf"));
    assert_eq!(
        value["variants"][0]["ram_bytes"],
        16 * 1024 * 1024 * 1024u64
    );
    assert_eq!(
        value["variants"][0]["vram_bytes"],
        6 * 1024 * 1024 * 1024u64
    );
    assert_eq!(
        value["variants"][0]["disk_bytes"],
        6 * 1024 * 1024 * 1024u64
    );
    for (model, reason) in [
        ("owner/missing", "repository_not_found"),
        ("owner/denied", "repository_unavailable"),
    ] {
        let value = h
            .json(&format!(
                "/api/models/status?model={model}&source=hf-mirror"
            ))
            .await;
        assert_eq!(value["status"], "developing");
        assert_eq!(value["downloadable"], false);
        assert_eq!(value["unavailable_reason"]["code"], reason);
        assert_eq!(value["variants"].as_array().expect("空工件列表").len(), 0);
    }
    let calls = h.calls.lock().expect("请求锁");
    assert_eq!(calls.len(), 6);
    assert!(calls.iter().any(|call| call.contains("/api/v1/models/")));
    assert!(calls.iter().all(|call| !call.contains("search=")));
}

#[tokio::test]
async fn alternate_gguf_presets_discover_files_and_create_direct_downloads() {
    let h = Harness::new().await;
    let presets = [
        (
            "llama2-chinese-13b-chat",
            "mradermacher/Llama2-Chinese-13b-Chat-GGUF",
            "hf-mirror",
            "main",
            "/mradermacher/Llama2-Chinese-13b-Chat-GGUF/resolve/main/model-Q4_K_M.gguf",
        ),
        (
            "dolphin-2_6-phi-2",
            "TheBloke/dolphin-2_6-phi-2-GGUF",
            "hf-mirror",
            "main",
            "/TheBloke/dolphin-2_6-phi-2-GGUF/resolve/main/model-Q4_K_M.gguf",
        ),
        (
            "jev-9b",
            "prithivMLmods/JEV-9B-GGUF",
            "modelscope",
            "master",
            "/models/prithivMLmods/JEV-9B-GGUF/resolve/master/model-Q4_K_M.gguf",
        ),
    ];

    for (preset_id, model, source, revision, artifact_path) in presets {
        let value = h
            .json(&format!("/api/models/status?preset={preset_id}"))
            .await;

        assert_eq!(value["model"], model, "{preset_id} 仓库身份");
        assert_eq!(value["source"], source, "{preset_id} 来源");
        assert_eq!(value["revision"], revision, "{preset_id} 修订号");
        assert_eq!(value["downloadable"], true, "{preset_id} 可下载状态");
        assert_eq!(value["variants"].as_array().map(Vec::len), Some(1));
        assert!(
            value["variants"][0]["artifact_source"]
                .as_str()
                .expect("直接工件地址")
                .ends_with(artifact_path),
            "{preset_id} 直接工件地址"
        );
    }
}

#[tokio::test]
async fn status_discovers_remote_files_independent_of_builtin_manifests() {
    let h = Harness::new().await;
    for path in [
        "/api/models/status?preset=qwen3-8b",
        "/api/models/status?preset=gpt-oss-20b",
        "/api/models/status?preset=meta-llama-3.1-8b-instruct",
        "/api/models/status?model=owner/repo&source=huggingface",
    ] {
        let value = h.json(path).await;
        assert_eq!(value["downloadable"], true, "{path}");
        assert_eq!(value["variants"].as_array().expect("实时工件").len(), 1);
        assert_eq!(value["variants"][0]["file_size_bytes"], 3);
        if path.contains("model=owner/repo") {
            assert_eq!(value["variants"][0]["ram_bytes"], 3);
            assert_eq!(value["variants"][0]["disk_bytes"], 3);
        }
    }
}

#[tokio::test]
async fn pending_presets_are_not_discovered_or_exposed_as_candidates() {
    let h = Harness::new().await;
    let value = h
        .json("/api/models/status?preset=nsfw_image_detection")
        .await;

    assert_eq!(value["identity_status"], "pending_validation");
    assert_eq!(value["downloadable"], false);
    assert_eq!(
        value["unavailable_reason"]["code"],
        "preset_identity_pending"
    );
    assert_eq!(value["variants"].as_array().expect("空工件列表").len(), 0);
    assert!(h.calls.lock().expect("请求锁").is_empty());
}

#[tokio::test]
async fn catalog_fallback_requires_exact_success_receipt_and_keeps_unavailable_reason() {
    let h = Harness::new().await;
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let resource = ModelResource {
        model: "org/denied".into(),
        source: Source::HuggingFace,
        revision: "main".into(),
    };
    let good_uri = format!(
        "https://huggingface.co/{}/resolve/{}/nested/model-Q4_K_M.gguf",
        resource.model, resource.revision
    );
    let make_record = |variant_id: &str,
                       artifact_uri: String,
                       checksum: String,
                       status: DownloadTaskStatus,
                       local_path: Option<String>,
                       actual_sha256: Option<String>,
                       total_bytes: u64| CatalogDownload {
        variant_id: variant_id.into(),
        artifact_uri,
        checksum,
        total_bytes,
        status,
        local_path,
        actual_sha256,
    };
    let records = vec![
        make_record(
            "nested/model-Q4_K_M.gguf",
            good_uri.clone(),
            format!("sha256:{hash}"),
            DownloadTaskStatus::Succeeded,
            Some("C:/models/model.gguf".into()),
            Some(format!("sha256:{hash}")),
            4096,
        ),
        make_record(
            "nested/wrong-uri.gguf",
            "https://huggingface.co/org/other/resolve/main/nested/wrong-uri.gguf".into(),
            format!("sha256:{hash}"),
            DownloadTaskStatus::Succeeded,
            Some("C:/models/wrong-uri.gguf".into()),
            Some(format!("sha256:{hash}")),
            4096,
        ),
        make_record(
            "nested/wrong-digest.gguf",
            "https://huggingface.co/org/denied/resolve/main/nested/wrong-digest.gguf".into(),
            format!("sha256:{hash}"),
            DownloadTaskStatus::Succeeded,
            Some("C:/models/wrong-digest.gguf".into()),
            Some(format!("sha256:{}", "c".repeat(64))),
            4096,
        ),
        make_record(
            "nested/no-path.gguf",
            "https://huggingface.co/org/denied/resolve/main/nested/no-path.gguf".into(),
            format!("sha256:{hash}"),
            DownloadTaskStatus::Succeeded,
            None,
            Some(format!("sha256:{hash}")),
            4096,
        ),
        make_record(
            "nested/failed.gguf",
            "https://huggingface.co/org/denied/resolve/main/nested/failed.gguf".into(),
            format!("sha256:{hash}"),
            DownloadTaskStatus::Failed,
            Some("C:/models/failed.gguf".into()),
            Some(format!("sha256:{hash}")),
            4096,
        ),
    ];
    let result = catalog_variants(&h.state, &resource, &records, None)
        .await
        .expect("本地回退检查");

    assert!(!result.downloadable);
    assert_eq!(
        result.unavailable_reason.as_ref().map(|reason| reason.code),
        Some("repository_unavailable")
    );
    assert_eq!(result.variants.len(), 1);
    assert_eq!(result.variants[0].artifact_source, good_uri);
    assert_eq!(result.variants[0].file_size_bytes, 4096);
    assert_eq!(
        result.variants[0].artifact_status,
        ArtifactStatus::Downloaded
    );
    assert!(result.variants[0].downloaded);
    assert_eq!(
        result.variants[0].local_path.as_deref(),
        Some("C:/models/model.gguf")
    );
}

#[tokio::test]
async fn not_found_and_empty_remote_catalog_do_not_reuse_local_fallbacks() {
    let h = Harness::new().await;
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let record = CatalogDownload {
        variant_id: "model-Q4_K_M.gguf".into(),
        artifact_uri: "https://huggingface.co/org/missing/resolve/main/model-Q4_K_M.gguf".into(),
        checksum: format!("sha256:{hash}"),
        total_bytes: 3,
        status: DownloadTaskStatus::Succeeded,
        local_path: Some("C:/models/model.gguf".into()),
        actual_sha256: Some(format!("sha256:{hash}")),
    };
    let missing = ModelResource {
        model: "org/missing".into(),
        source: Source::HuggingFace,
        revision: "main".into(),
    };
    let not_found = catalog_variants(&h.state, &missing, &[record], None)
        .await
        .expect("NotFound 作为状态返回");
    assert!(!not_found.downloadable);
    assert!(not_found.variants.is_empty());
    assert_eq!(
        not_found
            .unavailable_reason
            .as_ref()
            .map(|reason| reason.code),
        Some("repository_not_found")
    );

    let empty = h
        .json("/api/models/status?model=org/empty&source=huggingface")
        .await;
    assert!(!empty["downloadable"].as_bool().expect("布尔状态"));
    assert_eq!(empty["unavailable_reason"]["code"], "no_gguf_artifact");
    assert!(empty["variants"].as_array().expect("空工件列表").is_empty());
}

#[tokio::test]
async fn duplicate_repository_rows_keep_independent_preset_bindings() {
    let h = Harness::new().await;
    let presets = h.json("/api/models/presets").await;
    let duplicates: Vec<_> = presets
        .as_array()
        .expect("清单")
        .iter()
        .filter(|item| item["model"] == "ggml-org/Bespoke-Nimble-9B-v3-GGUF")
        .collect();
    assert_eq!(duplicates.len(), 2);
    assert_ne!(duplicates[0]["id"], duplicates[1]["id"]);
    assert_eq!(duplicates[0]["name"], "Bespoke-Nimble-9B-v3-GGUF");
    assert_eq!(duplicates[1]["name"], "Bespoke-Nimble-9B-v3-GGUF");
    for preset in duplicates {
        let id = preset["id"].as_str().expect("标识");
        let value = h.json(&format!("/api/models/status?preset={id}")).await;
        assert_eq!(value["model"], preset["model"]);
        assert_eq!(value["source"], "hf-mirror");
        assert_eq!(value["revision"], "main");
        assert_eq!(value["downloadable"], true);
        assert_eq!(value["variants"].as_array().expect("实时工件").len(), 1);
        let bound = presets::find(id)
            .expect("目录有效")
            .expect("重复仓库预置存在");
        let expected_vram = bound
            .hardware
            .vram_min
            .strip_suffix('G')
            .and_then(|value| value.parse::<u64>().ok())
            .expect("预置显存")
            * 1024
            * 1024
            * 1024;
        assert_eq!(value["variants"][0]["vram_bytes"], expected_vram);
        assert!(value["resource_url"]
            .as_str()
            .expect("地址")
            .ends_with("/mirror/ggml-org/Bespoke-Nimble-9B-v3-GGUF/tree/main"));
    }
}

#[tokio::test]
async fn invalid_inputs_never_reach_upstream() {
    let h = Harness::new().await;
    for path in [
        "/api/models/search",
        "/api/models/search?q=%20",
        "/api/models/status",
        "/api/models/status?preset=unknown",
        "/api/models/status?preset=invalid%21id",
        "/api/models/status?preset=qwen3-8b&model=owner/model&source=huggingface",
        "/api/models/status?model=owner/model&source=huggingface&artifact_uri=https%3A%2F%2Fevil.invalid%2Fmodel.gguf",
        "/api/models/status?model=a/b&source=unknown",
        "/api/models/status?model=../x&source=modelscope",
        "/api/models/status?model=a/b/c&source=modelscope",
        "/api/models/status?model=a/b&source=modelscope&revision=..",
    ] {
        assert_eq!(h.get(path).await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    assert!(h.calls.lock().expect("请求锁").is_empty());
}

#[tokio::test]
async fn search_streams_before_slow_source_and_replaces_by_priority() {
    let h = Harness::new().await;
    let (tx, mut rx) = mpsc::channel(4);
    let task = tokio::spawn(run_search(h.state.clone(), "normal".into(), tx));
    let first: Value =
        serde_json::from_str(&rx.recv().await.expect("首帧").expect("序列化")).expect("JSON");
    assert_eq!(first["done"], false);
    assert_eq!(first["items"][0]["source"], "hf-mirror");
    let mut lines = vec![first];
    while let Some(line) = rx.recv().await {
        lines.push(serde_json::from_str::<Value>(&line.expect("序列化")).expect("JSON"));
    }
    task.await.expect("搜索完成");
    assert_eq!(lines.len(), 3);
    let last = lines.last().expect("最终帧");
    assert_eq!(last["done"], true);
    assert_eq!(last["type"], "models");
    assert_eq!(last["items"].as_array().expect("命中项").len(), 1);
    assert_eq!(last["items"][0]["source"], "modelscope");
}

#[tokio::test]
async fn only_domestic_timeout_starts_a_single_fallback() {
    for (query, kind, fallback) in [
        ("timeout", "models", 1),
        ("empty", "unavailable", 0),
        ("failed", "failed", 0),
        ("partial", "models", 0),
        ("malformed", "failed", 0),
    ] {
        let h = Harness::new().await;
        let lines = h.search(query).await;
        let last = lines.last().expect("完成帧");
        assert_eq!(last["type"], kind, "{query}");
        assert_eq!(last["done"], true);
        assert_eq!(
            h.calls
                .lock()
                .expect("请求锁")
                .iter()
                .filter(|call| call.starts_with("/hf/"))
                .count(),
            fallback,
            "{query}"
        );
        if query == "timeout" {
            assert_eq!(last["items"][0]["source"], "huggingface");
        }
    }
}

#[tokio::test]
async fn dropping_search_receiver_cancels_inflight_requests() {
    let h = Harness::new().await;
    let (tx, rx) = mpsc::channel(4);
    let task = tokio::spawn(run_search(h.state.clone(), "timeout".into(), tx));
    tokio::time::timeout(Duration::from_secs(1), async {
        while h.calls.lock().expect("请求锁").len() < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("国内请求已启动");
    drop(rx);
    tokio::time::timeout(Duration::from_millis(150), task)
        .await
        .expect("立即取消而非等待超时")
        .expect("搜索任务");
    tokio::time::timeout(Duration::from_millis(150), async {
        while Arc::strong_count(&h.state) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("子任务已释放状态");
    assert_eq!(h.calls.lock().expect("请求锁").len(), 2);
}

#[tokio::test]
async fn unreachable_resource_is_not_downloadable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("测试端口");
    let base = format!("http://{}", listener.local_addr().expect("地址"));
    drop(listener);
    let config = Config {
        modelscope: base,
        probe_timeout: Duration::from_millis(50),
        ..Config::default()
    };
    let response = app(config, reqwest::Client::new(), None)
        .oneshot(
            Request::builder()
                .uri("/api/models/status?preset=qwen3-8b")
                .body(Body::empty())
                .expect("请求"),
        )
        .await
        .expect("状态响应");
    let body = to_bytes(response.into_body(), 10000)
        .await
        .expect("读取响应");
    assert_eq!(
        serde_json::from_slice::<Value>(&body).expect("JSON")["status"],
        "developing"
    );
}

#[tokio::test]
async fn catalog_survives_restart_and_matches_current_remote_artifact() {
    let mut h = Harness::new().await;
    let dir = tempfile::tempdir().expect("临时目录");
    let path = dir.path().join("model.gguf");
    tokio::fs::write(&path, b"abc").await.expect("工件");
    let file = dir.path().join("catalog.db");
    let url = burncloud_database::sqlite_url(
        &file.to_string_lossy().replace('\\', "/"),
        cfg!(windows),
        true,
    );
    let digest = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let db = burncloud_database::create_database_with_url(&url)
        .await
        .expect("建库");
    db.execute_query_with_params("INSERT INTO sys_downloads (gid, uris, status, model, source, revision, variant_id, artifact_uri, checksum, actual_sha256, final_path, total_length, completed_length) VALUES ('test', '[]', 'succeeded', 'org/repo', 'huggingface', 'main', 'model-Q4_K_M.gguf', 'https://huggingface.co/org/repo/resolve/main/model-Q4_K_M.gguf', ?, ?, ?, 3, 3)", vec![digest.into(), digest.into(), path.to_string_lossy().into_owned()]).await.expect("写入校验凭据");
    drop(db);
    let db = Arc::new(
        burncloud_database::create_database_with_url(&url)
            .await
            .expect("重新连接同一数据库"),
    );
    let config = Config {
        huggingface: h.state.config.huggingface.clone(),
        ..Config::default()
    };
    h.router = app(config, h.state.client.clone(), Some(db.clone()));
    let query = "/api/models/status?model=org/repo&source=huggingface&revision=main";
    for (update, expected) in [
        ("", "downloaded"),
        ("UPDATE sys_downloads SET status='active'", "downloading"),
        ("UPDATE sys_downloads SET status='completed'", "verifying"),
        ("UPDATE sys_downloads SET status='succeeded', actual_sha256=NULL", "failed"),
        ("UPDATE sys_downloads SET actual_sha256=checksum, final_path='nonexistent-file'", "failed"),
        ("UPDATE sys_downloads SET checksum='sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'", "not_downloaded"),
    ] {
        if !update.is_empty() { db.execute_query_with_params(update, Vec::new()).await.expect("更新测试记录"); }
        let (code, body) = h.get(query).await;
        assert_eq!(code, StatusCode::OK);
        let body: Value = serde_json::from_str(&body).expect("目录响应");
        assert_eq!(body["status"], "downloadable");
        assert_eq!(body["artifact_status"], expected);
        assert_eq!(body["variants"][0]["artifact_status"], expected);
        assert_eq!(body["downloaded"], expected == "downloaded");
        assert_eq!(body["actual_sha256"].as_str(), (expected == "downloaded").then_some(digest));
    }
}
