// UI model list — HTTP response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::channel_service::{Channel, ChannelService};
use burncloud_client_shared::components::validate_schema;
use burncloud_client_shared::components::{
    BCButton, ButtonVariant, FormMode, SchemaForm,
    PageHeader, StatusPill, Chip, EmptyState,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::schema::channel_schema;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;
use serde_json::json;

#[derive(PartialEq, Clone, Copy)]
enum ProviderType {
    OpenAI = 1,
    Anthropic = 14,
    Google = 24,
    Aws = 99,
    Azure = 98,
    Local = 97,
}

impl ProviderType {
    fn value_str(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "1",
            ProviderType::Anthropic => "14",
            ProviderType::Google => "24",
            ProviderType::Aws => "99",
            ProviderType::Azure => "98",
            ProviderType::Local => "97",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Anthropic => "Anthropic",
            ProviderType::Google => "Google Gemini",
            ProviderType::Aws => "AWS Bedrock",
            ProviderType::Azure => "Azure OpenAI",
            ProviderType::Local => "Local / GGUF",
        }
    }

    fn icon(&self) -> Element {
        match self {
            ProviderType::OpenAI => rsx! {
                svg {
                    view_box: "0 0 24 24",
                    class: "w-8 h-8",
                    fill: "currentColor",
                    path { d: "M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z" }
                }
            },
            ProviderType::Anthropic => rsx! {
                svg {
                    view_box: "0 0 24 24",
                    class: "w-8 h-8",
                    fill: "currentColor",
                    path { d: "M17.3041 3.541h-3.6718l6.696 16.918H24Zm-10.6082 0L0 20.459h3.7442l1.3693-3.5527h7.0052l1.3693 3.5528h3.7442L10.5363 3.5409Zm-.3712 10.2232 2.2914-5.9456 2.2914 5.9456Z" }
                }
            },
            ProviderType::Google => rsx! {
                svg {
                    view_box: "0 0 24 24",
                    class: "w-8 h-8",
                    fill: "currentColor",
                    path { d: "M11.04 19.32Q12 21.51 12 24q0-2.49.93-4.68.96-2.19 2.58-3.81t3.81-2.55Q21.51 12 24 12q-2.49 0-4.68-.93a12.3 12.3 0 0 1-3.81-2.58 12.3 12.3 0 0 1-2.58-3.81Q12 2.49 12 0q0 2.49-.96 4.68-.93 2.19-2.55 3.81a12.3 12.3 0 0 1-3.81 2.58Q2.49 12 0 12q2.49 0 4.68.96 2.19.93 3.81 2.55t2.55 3.81" }
                }
            },
            ProviderType::Aws => rsx! {
                svg {
                    view_box: "0 0 24 24",
                    class: "w-8 h-8",
                    fill: "currentColor",
                    path { d: "M6.763 10.036c0 .296.032.535.088.71.064.176.144.368.256.576.04.063.056.127.056.183 0 .08-.048.16-.152.24l-.503.335a.383.383 0 0 1-.208.072c-.08 0-.16-.04-.239-.112a2.47 2.47 0 0 1-.287-.375 6.18 6.18 0 0 1-.248-.471c-.622.734-1.405 1.101-2.347 1.101-.67 0-1.205-.191-1.596-.574-.391-.384-.59-.894-.59-1.533 0-.678.239-1.23.726-1.644.487-.415 1.133-.623 1.955-.623.272 0 .551.024.846.064.296.04.6.104.918.176v-.583c0-.607-.127-1.03-.375-1.277-.255-.248-.686-.367-1.3-.367-.28 0-.568.031-.863.103-.295.072-.583.16-.862.272a2.287 2.287 0 0 1-.28.104.488.488 0 0 1-.127.023c-.112 0-.168-.08-.168-.247v-.391c0-.128.016-.224.056-.28a.597.597 0 0 1 .224-.167c.279-.144.614-.264 1.005-.36a4.84 4.84 0 0 1 1.246-.151c.95 0 1.644.216 2.091.647.439.43.662 1.085.662 1.963v2.586zm-3.24 1.214c.263 0 .534-.048.822-.144.287-.096.543-.271.758-.51.128-.152.224-.32.272-.512.047-.191.08-.423.08-.694v-.335a6.66 6.66 0 0 0-.735-.136 6.02 6.02 0 0 0-.75-.048c-.535 0-.926.104-1.19.32-.263.215-.39.518-.39.917 0 .375.095.655.295.846.191.2.47.296.838.296zm6.41.862c-.144 0-.24-.024-.304-.08-.064-.048-.12-.16-.168-.311L7.586 5.55a1.398 1.398 0 0 1-.072-.32c0-.128.064-.2.191-.2h.783c.151 0 .255.025.31.08.065.048.113.16.16.312l1.342 5.284 1.245-5.284c.04-.16.088-.264.151-.312a.549.549 0 0 1 .32-.08h.638c.152 0 .256.025.32.08.063.048.12.16.151.312l1.261 5.348 1.381-5.348c.048-.16.104-.264.16-.312a.52.52 0 0 1-.311-.08h.743c.127 0 .2.065.2.2 0 .04-.009.08-.017.128a1.137 1.137 0 0 1-.056.2l-1.923 6.17c-.048.16-.104.263-.168.311a.51.51 0 0 1-.303.08h-.687c-.151 0-.255-.024-.32-.08-.063-.056-.119-.16-.15-.32l-1.238-5.148-1.23 5.14c-.04.16-.087.264-.15.32-.065.056-.177.08-.32.08zm10.256.215c-.415 0-.83-.048-1.229-.143-.399-.096-.71-.2-.918-.32-.128-.071-.215-.151-.247-.223a.563.563 0 0 1-.048-.224v-.407c0-.167.064-.247.183-.247.048 0 .096.008.144.024.048.016.12.048.2.08.271.12.566.215.878.279.319.064.63.096.95.096.502 0 .894-.088 1.165-.264a.86.86 0 0 0 .415-.758.777.777 0 0 0-.215-.559c-.144-.151-.416-.287-.807-.415l-1.157-.36c-.583-.183-1.014-.454-1.277-.813a1.902 1.902 0 0 1-.4-1.158c0-.335.073-.63.216-.886.144-.255.335-.479.575-.654.24-.184.51-.32.83-.415.32-.096.655-.136 1.006-.136.175 0 .359.008.535.032.183.024.35.056.518.088.16.04.312.08.455.127.144.048.256.096.336.144a.69.69 0 0 1 .24.2.43.43 0 0 1 .071.263v.375c0 .168-.064.256-.184.256a.83.83 0 0 1-.303-.096 3.652 3.652 0 0 0-1.532-.311c-.455 0-.815.071-1.062.223-.248.152-.375.383-.375.71 0 .224.08.416.24.567.159.152.454.304.877.44l1.134.358c.574.184.99.44 1.237.767.247.327.367.702.367 1.117 0 .343-.072.655-.207.926-.144.272-.336.511-.583.703-.248.2-.543.343-.886.447-.36.111-.734.167-1.142.167zM21.698 16.207c-2.626 1.94-6.442 2.969-9.722 2.969-4.598 0-8.74-1.7-11.87-4.526-.247-.223-.024-.527.272-.351 3.384 1.963 7.559 3.153 11.877 3.153 2.914 0 6.114-.607 9.06-1.852.439-.2.814.287.383.607zM22.792 14.961c-.336-.43-2.22-.207-3.074-.103-.255.032-.295-.192-.063-.36 1.5-1.053 3.967-.75 4.254-.399.287.36-.08 2.826-1.485 4.007-.215.184-.423.088-.327-.151.32-.79 1.03-2.57.695-2.994z" }
                }
            },
            ProviderType::Azure => rsx! {
                svg {
                    view_box: "0 0 96 96",
                    class: "w-8 h-8",
                    fill: "currentColor",
                    path { d: "M33.338 6.544h26.038l-27.03 80.087a4.152 4.152 0 0 1-3.933 2.824H8.149a4.145 4.145 0 0 1-3.928-5.47L29.404 9.368a4.152 4.152 0 0 1 3.934-2.825z" }
                    path { d: "M71.175 60.261h-41.29a1.911 1.911 0 0 0-1.305 3.309l26.532 24.764a4.171 4.171 0 0 0 2.846 1.121h23.38z" }
                    path { d: "M66.595 9.364a4.145 4.145 0 0 0-3.928-2.82H33.648a4.146 4.146 0 0 1 3.928 2.82l25.184 74.62a4.146 4.146 0 0 1-3.928 5.472h29.02a4.146 4.146 0 0 0 3.927-5.472z" }
                }
            },
            ProviderType::Local => rsx! {
                svg {
                    view_box: "0 0 24 24",
                    class: "w-8 h-8",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" }
                }
            },
        }
    }
}

fn build_channel_from_form(data: &serde_json::Value) -> Channel {
    let type_str = data["type"].as_str().unwrap_or("1");
    let _provider_type: i32 = type_str.parse().unwrap_or(1);

    let (final_type, final_base_url, final_models, param_override, header_override) = match type_str
    {
        "1" => (1, "https://api.openai.com".to_string(), "gpt-4,gpt-4-turbo,gpt-3.5-turbo".to_string(), None, None),
        "14" => (14, "https://api.anthropic.com".to_string(), "claude-3-opus-20240229,claude-3-sonnet-20240229".to_string(), None, None),
        "24" => {
            let auth_type = data["google_auth_type"].as_str().unwrap_or("api_key");
            if auth_type == "vertex" {
                let mut params = serde_json::Map::new();
                if let Some(r) = data["google_region"].as_str() { params.insert("region".to_string(), json!(r)); }
                if let Some(p) = data["google_project_id"].as_str() { if !p.is_empty() { params.insert("project_id".to_string(), json!(p)); } }
                (41, "https://aiplatform.googleapis.com".to_string(), "gemini-pro,gemini-1.5-pro".to_string(), Some(serde_json::Value::Object(params).to_string()), None)
            } else {
                (24, "https://generativelanguage.googleapis.com".to_string(), "gemini-pro,gemini-1.5-pro".to_string(), None, None)
            }
        }
        "99" => {
            let region = data["aws_region"].as_str().unwrap_or("us-east-1");
            let base_url = format!("https://bedrock-runtime.{}.amazonaws.com", region);
            let models = data["aws_model_id"].as_str().unwrap_or("anthropic.claude-sonnet-4-5-20250929-v1:0").to_string();
            let params = json!({"aws_secret_key": data["aws_sk"].as_str().unwrap_or(""), "region": region, "auth_type": "aws_sigv4"});
            (1, base_url, models, Some(params.to_string()), None)
        }
        "98" => {
            let resource = data["azure_resource"].as_str().unwrap_or("");
            let deployment = data["azure_deployment"].as_str().unwrap_or("");
            let base_url = format!("https://{}.openai.azure.com/openai/deployments/{}", resource, deployment);
            let models = deployment.to_string();
            let params = json!({"api_version": data["azure_api_version"].as_str().unwrap_or("2023-05-15"), "auth_type": "azure_ad"});
            let headers = json!({"api-key": data["azure_key"].as_str().unwrap_or("")});
            (1, base_url, models, Some(params.to_string()), Some(headers.to_string()))
        }
        "97" => {
            let base_url = data["local_url"].as_str().unwrap_or("http://localhost:8080").to_string();
            (1, base_url, "local-model".to_string(), None, None)
        }
        _ => (1, String::new(), String::new(), None, None),
    };

    let final_key = match type_str {
        "24" => {
            let auth_type = data["google_auth_type"].as_str().unwrap_or("api_key");
            if auth_type == "vertex" { data["google_vertex_key"].as_str().unwrap_or("").to_string() }
            else { data["google_key"].as_str().unwrap_or("").to_string() }
        }
        "98" => data["azure_key"].as_str().unwrap_or("").to_string(),
        "99" => data["aws_key"].as_str().unwrap_or("").to_string(),
        _ => data["key"].as_str().unwrap_or("").to_string(),
    };

    Channel {
        id: data["id"].as_i64().unwrap_or(0),
        type_: final_type,
        name: data["name"].as_str().unwrap_or("").to_string(),
        key: final_key,
        base_url: final_base_url,
        models: final_models,
        group: data["group"].as_str().map(|s| s.to_string()),
        status: data["status"].as_i64().unwrap_or(1) as i32,
        priority: data["priority"].as_i64().unwrap_or(0) as i32,
        weight: data["weight"].as_i64().unwrap_or(0) as i32,
        param_override,
        header_override,
    }
}

fn channel_status(status: i32) -> String {
    match status {
        1 => "ok".to_string(),
        2 => "warning".to_string(),
        0 => "danger".to_string(),
        _ => "info".to_string(),
    }
}

fn channel_status_label(status: i32) -> String {
    match status {
        1 => "正常".to_string(),
        2 => "限流".to_string(),
        0 => "已停止".to_string(),
        _ => "维护中".to_string(),
    }
}

fn provider_color(type_: i32) -> &'static str {
    match type_ {
        1 => "#10a37f",
        14 => "#d97757",
        24 | 41 => "#4285f4",
        98 => "#0078d4",
        97 => "#615ced",
        _ => "#86868B",
    }
}

fn provider_name(type_: i32) -> &'static str {
    match type_ {
        1 => "openai",
        14 => "anthropic",
        24 => "google",
        41 => "google",
        98 => "azure",
        99 => "aws",
        97 => "qwen",
        _ => "other",
    }
}

#[component]
pub fn ChannelPage() -> Element {
    let mut active_filter = use_signal(|| "all".to_string());
    let mut is_modal_open = use_signal(|| false);
    let mut modal_step = use_signal(|| 0);
    let mut is_delete_modal_open = use_signal(|| false);
    let delete_channel_id = use_signal(|| 0i64);
    let delete_channel_name = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let toast = use_toast();

    let mut form_data = use_signal(|| serde_json::Value::Object(serde_json::Map::new()));
    let schema = channel_schema();

    let mut channels = use_resource(move || async move {
        ChannelService::list(0, 200).await
    });

    let open_create_modal = move |_| {
        form_data.set(serde_json::Value::Object(serde_json::Map::new()));
        modal_step.set(0);
        is_modal_open.set(true);
    };

    let channels_ref = channels;
    let mut select_provider = move |p: ProviderType| {
        let adjectives = vec!["cosmic", "fluent", "quantum", "hyper", "silent", "pure", "rapid", "steady", "active", "neural", "prime", "noble", "swift", "calm", "wild", "bright"];
        let nouns = vec!["flow", "grid", "core", "nexus", "pulse", "link", "node", "sphere", "spark", "wave", "beam", "edge", "mind", "field", "stream", "gate"];

        let existing_names: Vec<String> = channels_ref.read().as_ref().map(|r| r.as_ref().map(|list| list.iter().map(|c| c.name.clone()).collect()).unwrap_or_default()).unwrap_or_default();
        let mut rng = rand::thread_rng();
        let mut generated_name = String::new();

        for _ in 0..10 {
            let adj = adjectives.choose(&mut rng).unwrap_or(&"zen");
            let noun = nouns.choose(&mut rng).unwrap_or(&"mode");
            let suffix: u16 = rng.gen_range(100..999);
            let candidate = format!("{} {} {}", adj[0..1].to_uppercase() + &adj[1..], noun[0..1].to_uppercase() + &noun[1..], suffix);
            if !existing_names.contains(&candidate) { generated_name = candidate; break; }
        }
        if generated_name.is_empty() {
            let suffix: u16 = rng.gen_range(1000..9999);
            generated_name = format!("{} Link {}", p.name(), suffix);
        }

        let mut obj = serde_json::Map::new();
        obj.insert("type".to_string(), json!(p.value_str().to_string()));
        obj.insert("name".to_string(), json!(generated_name));
        obj.insert("id".to_string(), json!(0));
        obj.insert("status".to_string(), json!(1));
        obj.insert("group".to_string(), json!("default"));
        obj.insert("priority".to_string(), json!(0));
        obj.insert("weight".to_string(), json!(0));

        match p {
            ProviderType::Aws => { obj.insert("aws_region".to_string(), json!("us-east-1")); obj.insert("aws_model_id".to_string(), json!("anthropic.claude-sonnet-4-5-20250929-v1:0")); }
            ProviderType::Google => { obj.insert("google_auth_type".to_string(), json!("api_key")); obj.insert("google_region".to_string(), json!("us-central1")); }
            ProviderType::Azure => { obj.insert("azure_api_version".to_string(), json!("2023-05-15")); }
            ProviderType::Local => { obj.insert("local_url".to_string(), json!("http://localhost:8080")); }
            _ => {}
        }

        form_data.set(serde_json::Value::Object(obj));
        modal_step.set(1);
    };

    let schema_for_save = schema.clone();
    let handle_save = move |_| {
        let s = schema_for_save.clone();
        spawn(async move {
            is_loading.set(true);
            let current_data = form_data.read().clone();
            let errors = validate_schema(&s, &current_data);
            if !errors.is_empty() { is_loading.set(false); toast.error("请填写所有必填字段"); return; }

            let ch = build_channel_from_form(&current_data);
            let result = if ch.id == 0 { ChannelService::create(&ch).await } else { ChannelService::update(&ch).await };

            match result {
                Ok(_) => { is_modal_open.set(false); channels.restart(); toast.success("保存成功"); }
                Err(e) => toast.error(&format!("保存失败: {}", e)),
            }
            is_loading.set(false);
        });
    };

    let handle_confirm_delete = move |_| {
        spawn(async move {
            if ChannelService::delete(delete_channel_id()).await.is_ok() {
                channels.restart();
                toast.success("渠道已删除");
                is_delete_modal_open.set(false);
            } else {
                toast.error("删除失败");
            }
        });
    };

    let ch_res = channels.read().clone();
    let loading = ch_res.is_none();
    let ch_list = ch_res.and_then(|r| r.ok()).unwrap_or_default();

    let active_count = ch_list.iter().filter(|c| c.status == 1).count();
    let throttle_count = ch_list.iter().filter(|c| c.status == 2).count();
    let down_count = ch_list.iter().filter(|c| c.status == 0).count();
    let maint_count = ch_list.iter().filter(|c| c.status != 0 && c.status != 1 && c.status != 2).count();
    let total_weight: i32 = ch_list.iter().map(|c| c.weight).sum();
    let health_rate = if ch_list.is_empty() { 0.0 } else { active_count as f64 / ch_list.len() as f64 * 100.0 };

    let filtered: Vec<&Channel> = ch_list.iter().filter(|c| {
        match active_filter().as_str() {
            "ok" => c.status == 1,
            "throttle" => c.status == 2,
            "down" => c.status == 0,
            "maint" => c.status != 0 && c.status != 1 && c.status != 2,
            _ => true,
        }
    }).collect();

    rsx! {
        PageHeader {
            title: "模型网络",
            subtitle: Some(format!("{} 个渠道 · 总权重 {}", ch_list.len(), total_weight)),
            actions: rsx! {
                div { class: "input sm bc-w-240",
                    input {
                        r#type: "text",
                        placeholder: "搜索渠道、模型或组织…",
                    }
                }
                button { class: "btn btn-secondary", "筛选" }
                BCButton {
                    class: "btn-primary",
                    onclick: open_create_modal,
                    "创建渠道"
                }
            },
        }

        div { class: "page-content flex flex-col gap-xl",
            // KPI strip
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "活跃渠道" }
                        div { class: "stat-value",
                            "{active_count}"
                            span { class: "stat-pill muted", "/ {ch_list.len()}" }
                        }
                        span { class: "stat-foot", "2 个 org · 5 个 provider" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "总权重" }
                        div { class: "stat-value", "{total_weight}" }
                        span { class: "stat-foot", "round-robin 加权分发" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "合计 RPM" }
                        div { class: "stat-value", "—" }
                        span { class: "stat-foot up", "↑ 8.2% vs 1h ago" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "健康率" }
                        div { class: "stat-value success",
                            "{health_rate:.0}%"
                        }
                        span { class: "stat-foot", "{throttle_count} 个限流 · {down_count} 个停止" }
                    }
                }
            }

            // Filter chips + table
            div {
                div { class: "section-h",
                    span { class: "lead-title", "渠道明细" }
                    div { class: "chip-row",
                        Chip {
                            label: "全部".to_string(),
                            count: Some(ch_list.len() as i64),
                            active: Some(active_filter() == "all"),
                            onclick: move |_| active_filter.set("all".to_string()),
                        }
                        Chip {
                            label: "正常".to_string(),
                            count: Some(active_count as i64),
                            active: Some(active_filter() == "ok"),
                            onclick: move |_| active_filter.set("ok".to_string()),
                        }
                        Chip {
                            label: "限流".to_string(),
                            count: Some(throttle_count as i64),
                            active: Some(active_filter() == "throttle"),
                            onclick: move |_| active_filter.set("throttle".to_string()),
                        }
                        Chip {
                            label: "已停止".to_string(),
                            count: Some(down_count as i64),
                            active: Some(active_filter() == "down"),
                            onclick: move |_| active_filter.set("down".to_string()),
                        }
                        Chip {
                            label: "维护".to_string(),
                            count: Some(maint_count as i64),
                            active: Some(active_filter() == "maint"),
                            onclick: move |_| active_filter.set("maint".to_string()),
                        }
                    }
                }

                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                } else if filtered.is_empty() && !ch_list.is_empty() {
                    EmptyState {
                        icon: rsx! { span { class: "bc-text-4xl", "📡" } },
                        title: "无匹配渠道".to_string(),
                        description: Some("调整筛选条件".to_string()),
                        cta: None,
                    }
                } else if ch_list.is_empty() {
                    EmptyState {
                        icon: rsx! { span { class: "bc-text-4xl", "📡" } },
                        title: "暂无模型渠道".to_string(),
                        description: Some("创建第一个渠道开始使用".to_string()),
                        cta: Some(rsx! {
                            BCButton {
                                class: "btn-black",
                                onclick: open_create_modal,
                                "创建渠道"
                            }
                        }),
                    }
                } else {
                    table { class: "table",
                        thead {
                            tr {
                                th { "NAME" }
                                th { "PROVIDER" }
                                th { "MODEL" }
                                th { "WEIGHT" }
                                th { "P50" }
                                th { "RPM" }
                                th { "STATUS" }
                                th {}
                            }
                        }
                        tbody {
                            for ch in &filtered {
                                tr {
                                    key: "{ch.id}",
                                    td { class: "font-medium", "{ch.name}" }
                                    td {
                                        span { class: "bc-provider-row",
                                            span { class: "bc-provider-dot", style: "--bc-dynamic-provider-color:{provider_color(ch.type_)}" }
                                            span { class: "bc-text-sm-secondary", "{provider_name(ch.type_)}" }
                                        }
                                    }
                                    td { class: "mono bc-text-sm", "{ch.models}" }
                                    td { class: "mono", "{ch.weight}" }
                                    td { class: "mono", "—" }
                                    td { class: "mono", "—" }
                                    td {
                                        StatusPill {
                                            value: channel_status(ch.status),
                                            label: Some(channel_status_label(ch.status)),
                                        }
                                    }
                                    td { class: "text-right",
                                        button { class: "bc-icon-btn", "⚙" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Create/edit modal
        if is_modal_open() {
            div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-0 sm:p-4",
                div {
                    class: "absolute inset-0 transition-opacity bc-modal-overlay",
                    onclick: move |_| is_modal_open.set(false)
                }

                div {
                    class: "relative w-full h-full sm:h-auto sm:max-h-[90vh] sm:max-w-2xl flex flex-col overflow-hidden animate-scale-in pointer-events-auto overscroll-contain bc-modal-card",
                    onclick: |e| e.stop_propagation(),

                    div { class: "flex justify-between items-center px-md py-sm sm:px-lg sm:py-md border-b shrink-0 bc-modal-header-bg",
                        h3 { class: "text-subtitle font-bold text-primary tracking-tight",
                            if modal_step() == 0 { "选择供应商" } else { "配置连接" }
                        }
                        button {
                            class: "btn btn-sm btn-circle btn-ghost text-secondary",
                            onclick: move |_| is_modal_open.set(false),
                            "✕"
                        }
                    }

                    div { class: "flex-1 overflow-y-auto p-md sm:p-lg min-h-0 overscroll-y-contain",
                        if modal_step() == 0 {
                            div { class: "grid grid-cols-2 sm:grid-cols-3 gap-md",
                                for p in [ProviderType::OpenAI, ProviderType::Anthropic, ProviderType::Google, ProviderType::Aws, ProviderType::Azure, ProviderType::Local] {
                                    button {
                                        class: "bc-card-solid group flex flex-col items-center justify-center gap-md p-lg h-36 transition-all duration-300 ease-out cursor-pointer",
                                        onclick: move |_| select_provider(p),
                                        div { class: "text-secondary group-hover:text-primary transition-colors duration-300 transform group-hover:scale-110",
                                            {p.icon()}
                                        }
                                        span { class: "font-medium text-caption text-secondary group-hover:text-primary", "{p.name()}" }
                                    }
                                }
                            }
                        } else {
                            SchemaForm {
                                schema: schema.clone(),
                                data: form_data,
                                mode: FormMode::Create,
                                show_actions: false,
                                on_submit: move |v| { form_data.set(v); }
                            }
                        }
                    }

                    div { class: "flex justify-end gap-md px-lg py-md border-t shrink-0 bc-modal-footer-bg",
                        if modal_step() == 1 {
                            BCButton {
                                variant: ButtonVariant::Ghost,
                                onclick: move |_| modal_step.set(0),
                                "上一步"
                            }
                        }
                        BCButton {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| is_modal_open.set(false),
                            "取消"
                        }
                        if modal_step() == 1 {
                            BCButton {
                                class: "btn-neutral text-white shadow-md",
                                loading: is_loading(),
                                onclick: handle_save,
                                "保存"
                            }
                        }
                    }
                }
            }
        }

        // Delete confirmation modal
        if is_delete_modal_open() {
            div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-md",
                div {
                    class: "absolute inset-0 transition-opacity bc-modal-overlay",
                    onclick: move |_| is_delete_modal_open.set(false)
                }

                div {
                    class: "relative w-full max-w-md overflow-hidden animate-scale-in bc-modal-card",
                    onclick: |e| e.stop_propagation(),

                    div { class: "flex items-center gap-md px-lg py-lg border-b bc-danger-header",
                        div { class: "w-12 h-12 rounded-full flex items-center justify-center bc-danger-light-bg",
                            svg { class: "w-6 h-6 text-danger", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
                            }
                        }
                        div { class: "flex-1",
                            h3 { class: "text-subtitle font-bold text-primary", "确认删除" }
                            p { class: "text-caption text-secondary mt-xs", "此操作无法撤销" }
                        }
                    }

                    div { class: "px-lg py-md",
                        p { class: "text-secondary",
                            "确定要删除连接 \""
                            span { class: "font-semibold text-primary", "{delete_channel_name()}" }
                            "\" 吗？"
                        }
                    }

                    div { class: "flex justify-end gap-md px-lg py-md border-t bc-modal-footer-bg",
                        BCButton {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| is_delete_modal_open.set(false),
                            "取消"
                        }
                        BCButton {
                            class: "btn-error text-white shadow-md",
                            onclick: handle_confirm_delete,
                            "确认删除"
                        }
                    }
                }
            }
        }
    }
}