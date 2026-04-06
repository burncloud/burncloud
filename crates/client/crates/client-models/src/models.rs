use burncloud_client_shared::channel_service::{Channel, ChannelService};
use burncloud_client_shared::components::{BCButton, BCInput, ButtonVariant};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;
use serde_json::json;

#[derive(PartialEq, Clone, Copy)]
enum ProviderType {
    OpenAI = 1,
    Anthropic = 14,
    Google = 24,
    Aws = 99,   // Custom mapping for UI
    Azure = 98, // Custom mapping for UI
    Local = 97, // Custom mapping for UI
}

impl ProviderType {
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

#[component]
pub fn ChannelPage() -> Element {
    let page = use_signal(|| 1);
    let limit = 10;

    let mut channels =
        use_resource(
            move || async move { ChannelService::list(page(), limit).await.unwrap_or(vec![]) },
        );

    let mut is_modal_open = use_signal(|| false);
    let mut modal_step = use_signal(|| 0); // 0: Select Provider, 1: Configure
    let mut selected_provider = use_signal(|| ProviderType::OpenAI);

    let mut is_delete_modal_open = use_signal(|| false);
    let mut delete_channel_id = use_signal(|| 0i64);
    let mut delete_channel_name = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let toast = use_toast();

    // Form Fields
    let mut form_id = use_signal(|| 0i64);
    let mut form_name = use_signal(String::new);
    // Common fields
    let mut form_key = use_signal(String::new); // OpenAI Key, AWS AK, Azure Key
                                                // AWS specific
    let mut form_aws_sk = use_signal(String::new);
    let mut form_aws_region = use_signal(|| "us-east-1".to_string());
    let mut form_aws_model_id =
        use_signal(|| "anthropic.claude-sonnet-4-5-20250929-v1:0".to_string());
    // Azure specific
    let mut form_azure_resource = use_signal(String::new);
    let mut form_azure_deployment = use_signal(String::new);
    let mut form_azure_api_version = use_signal(|| "2023-05-15".to_string());
    // Local specific
    let mut form_local_url = use_signal(|| "http://localhost:8080".to_string());
    // Google specific
    let mut form_google_auth_type = use_signal(|| "api_key".to_string()); // "api_key" or "vertex"
    let mut form_google_region = use_signal(|| "us-central1".to_string());
    let mut form_google_project_id = use_signal(String::new);

    let open_create_modal = move |_| {
        form_id.set(0);
        modal_step.set(0); // Start at selection
                           // Reset fields
        form_name.set(String::new());
        form_key.set(String::new());
        form_aws_sk.set(String::new());
        form_aws_region.set("us-east-1".to_string());
        form_aws_model_id.set("anthropic.claude-sonnet-4-5-20250929-v1:0".to_string());
        form_azure_resource.set(String::new());
        form_azure_deployment.set(String::new());
        form_google_auth_type.set("api_key".to_string());
        form_google_region.set("us-central1".to_string());
        form_google_project_id.set(String::new());
        is_modal_open.set(true);
    };

    let mut select_provider = move |p: ProviderType| {
        selected_provider.set(p);

        // Google-Style Name Generator: {Adjective}-{Noun}-{Number}
        let adjectives = vec![
            "cosmic",
            "fluent",
            "quantum",
            "hyper",
            "silent",
            "pure",
            "rapid",
            "steady",
            "active",
            "neural",
            "prime",
            "noble",
            "swift",
            "calm",
            "wild",
            "bright",
            "ancient",
            "azure",
            "bold",
            "brave",
            "crimson",
            "crystal",
            "dawn",
            "dusk",
            "early",
            "faint",
            "frozen",
            "gentle",
            "global",
            "golden",
            "hollow",
            "infinite",
            "inner",
            "jade",
            "keen",
            "late",
            "living",
            "lost",
            "lucky",
            "misty",
            "morning",
            "neon",
            "ocean",
            "orange",
            "pale",
            "proud",
            "purple",
            "quiet",
            "red",
            "rising",
            "round",
            "royal",
            "sharp",
            "shining",
            "small",
            "snowy",
            "solar",
            "sparkling",
            "spring",
            "still",
            "summer",
            "super",
            "sweet",
            "throbbing",
            "tight",
            "tiny",
            "twilight",
            "vast",
            "violet",
            "wandering",
            "falling",
            "flying",
            "hidden",
            "broken",
            "empty",
            "heavy",
        ];
        let nouns = vec![
            "flow",
            "grid",
            "core",
            "nexus",
            "pulse",
            "link",
            "node",
            "sphere",
            "spark",
            "wave",
            "beam",
            "edge",
            "mind",
            "field",
            "stream",
            "gate",
            "aurora",
            "base",
            "bird",
            "block",
            "boat",
            "breeze",
            "brook",
            "bush",
            "canopy",
            "canyon",
            "cell",
            "cloud",
            "cliff",
            "creek",
            "data",
            "dew",
            "dream",
            "drive",
            "dust",
            "feather",
            "fire",
            "flame",
            "forest",
            "frog",
            "frost",
            "fume",
            "glade",
            "glen",
            "grass",
            "haze",
            "hill",
            "ice",
            "island",
            "lake",
            "leaf",
            "limit",
            "log",
            "loop",
            "marsh",
            "meadow",
            "mode",
            "moon",
            "moss",
            "mountain",
            "network",
            "night",
            "oasis",
            "paper",
            "path",
            "peak",
            "pebble",
            "pine",
            "pond",
            "port",
            "rain",
            "range",
            "reef",
            "resonance",
            "river",
            "rock",
            "sea",
            "shadow",
            "shape",
            "silence",
            "sky",
            "smoke",
            "snow",
            "sound",
            "space",
            "star",
            "stone",
            "storm",
            "sun",
            "sunset",
            "surf",
            "tide",
            "tree",
            "truth",
            "union",
            "valley",
            "view",
            "voice",
            "water",
            "way",
            "web",
            "wind",
            "wing",
            "wolf",
            "wood",
            "world",
        ];

        let existing_names: Vec<String> = channels
            .read()
            .as_ref()
            .map(|list| list.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();

        let mut generated_name = String::new();

        let mut rng = rand::thread_rng();
        use rand::seq::SliceRandom;
        use rand::Rng;

        for _ in 0..10 {
            let adj = adjectives.choose(&mut rng).unwrap_or(&"zen");
            let noun = nouns.choose(&mut rng).unwrap_or(&"mode");
            let suffix: u16 = rng.gen_range(100..999);

            let candidate = format!(
                "{} {} {}",
                adj[0..1].to_uppercase() + &adj[1..],
                noun[0..1].to_uppercase() + &noun[1..],
                suffix
            );

            if !existing_names.contains(&candidate) {
                generated_name = candidate;
                break;
            }
        }

        // Fallback
        if generated_name.is_empty() {
            let suffix: u16 = rng.gen_range(1000..9999);
            generated_name = format!("{} Link {}", p.name(), suffix);
        }

        form_name.set(generated_name);
        modal_step.set(1);
    };

    let handle_save = move |_| {
        spawn(async move {
            is_loading.set(true);

            let provider = selected_provider();
            let final_type;
            let final_base_url;
            let final_models;
            let mut final_param_override = None;
            let mut final_header_override = None;
            let final_key = form_key();

            match provider {
                ProviderType::OpenAI => {
                    final_type = 1;
                    final_base_url = "https://api.openai.com".to_string();
                    final_models = "gpt-4,gpt-4-turbo,gpt-3.5-turbo".to_string();
                }
                ProviderType::Anthropic => {
                    final_type = 14;
                    final_base_url = "https://api.anthropic.com".to_string();
                    final_models = "claude-3-opus-20240229,claude-3-sonnet-20240229".to_string();
                }
                ProviderType::Google => {
                    if form_google_auth_type() == "vertex" {
                        final_type = 41; // VertexAi
                        final_base_url = "https://aiplatform.googleapis.com".to_string();
                        final_models = "gemini-pro,gemini-1.5-pro".to_string();

                        let mut params_map = serde_json::Map::new();
                        params_map.insert("region".to_string(), json!(form_google_region()));
                        if !form_google_project_id().is_empty() {
                            params_map
                                .insert("project_id".to_string(), json!(form_google_project_id()));
                        }
                        final_param_override =
                            Some(serde_json::Value::Object(params_map).to_string());
                    } else {
                        final_type = 24; // Gemini
                        final_base_url = "https://generativelanguage.googleapis.com".to_string();
                        final_models = "gemini-pro,gemini-1.5-pro".to_string();
                    }
                }
                ProviderType::Aws => {
                    final_type = 1;
                    final_base_url = format!(
                        "https://bedrock-runtime.{}.amazonaws.com",
                        form_aws_region()
                    );
                    final_models = form_aws_model_id();

                    let params = json!({
                        "aws_secret_key": form_aws_sk(),
                        "region": form_aws_region(),
                        "auth_type": "aws_sigv4"
                    });
                    final_param_override = Some(params.to_string());
                }
                ProviderType::Azure => {
                    final_type = 1;
                    final_base_url = format!(
                        "https://{}.openai.azure.com/openai/deployments/{}",
                        form_azure_resource(),
                        form_azure_deployment()
                    );
                    final_models = form_azure_deployment();

                    let params = json!({
                        "api_version": form_azure_api_version(),
                        "auth_type": "azure_ad"
                    });
                    final_param_override = Some(params.to_string());

                    let headers = json!({
                        "api-key": form_key()
                    });
                    final_header_override = Some(headers.to_string());
                }
                ProviderType::Local => {
                    final_type = 1;
                    final_base_url = form_local_url();
                    final_models = "local-model".to_string();
                }
            }

            let ch = Channel {
                id: form_id(),
                type_: final_type,
                name: form_name(),
                key: final_key,
                base_url: final_base_url,
                models: final_models,
                group: Some("default".to_string()),
                status: 1,
                priority: 0,
                weight: 0,
                param_override: final_param_override,
                header_override: final_header_override,
            };

            let result = if ch.id == 0 {
                ChannelService::create(&ch).await
            } else {
                ChannelService::update(&ch).await
            };

            match result {
                Ok(_) => {
                    is_modal_open.set(false);
                    channels.restart();
                    toast.success("保存成功");
                }
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

    let handle_toggle_status = move |c: Channel| {
        let mut new_c = c.clone();
        new_c.status = if c.status == 1 { 0 } else { 1 };
        spawn(async move {
            if ChannelService::update(&new_c).await.is_ok() {
                channels.restart();
            } else {
                toast.error("Failed to update status");
            }
        });
    };

    let channels_data = channels.read().clone();

    let _any_modal_open = is_modal_open() || is_delete_modal_open();

    rsx! {
        // Main Container for Page
        div { class: "relative h-full",
            div {
                class: "flex flex-col h-full gap-xl transition-all duration-300 ease-out",

                // Header
                div { class: "flex justify-between items-end px-xs",
                    div {
                        h1 { class: "text-title font-semibold text-primary mb-xs tracking-tight", "模型网络" }
                        p { class: "text-caption text-secondary font-medium", "您的 AI 算力中枢" }
                    }
                    div { class: "flex gap-md",
                        BCButton {
                            class: "btn-neutral btn-sm px-lg shadow-sm text-white",
                            onclick: open_create_modal,
                            "添加连接"
                        }
                    }
                }

                // Cards Grid
                    div { class: "flex-1 overflow-y-auto min-h-0",
                        match channels_data {
                            Some(list) => {
                                if list.is_empty() {
                                    rsx! {
                                        div { class: "flex flex-col items-center justify-center h-full text-center pb-xxl",
                                            div { class: "p-lg rounded-full mb-lg",
                                                style: "background: var(--bc-bg-hover);",
                                                svg {
                                                    class: "w-12 h-12",
                                                    style: "color: var(--bc-text-disabled);",
                                                    fill: "none",
                                                    view_box: "0 0 24 24",
                                                    stroke: "currentColor",
                                                    stroke_width: "1.5",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.428 15.428a2 2 0 00-1.022-.547l-2.384-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" }
                                                }
                                            }
                                            h3 { class: "text-title font-bold text-primary mb-sm tracking-tight", "暂无模型网络" }
                                            p { class: "text-body text-secondary max-w-sm mb-xl leading-relaxed",
                                                "连接您的第一个 AI 服务提供商，构建专属的神经中枢。"
                                            }
                                            BCButton {
                                                class: "btn-primary btn-md px-xl shadow-lg shadow-primary/20",
                                                onclick: open_create_modal,
                                                "开始连接"
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div { class: "w-full overflow-x-auto",
                                            table { class: "table table-lg",
                                                thead {
                                                    tr {
                                                        th { "Status" }
                                                        th { "Name" }
                                                        th { "Replicas" }
                                                        th { "Actions" }
                                                    }
                                                }
                                                tbody {
                                                    for channel in list {
                                                        tr {
                                                            td {
                                                                if channel.status == 1 {
                                                                    div { class: "badge badge-success gap-sm",
                                                                        "Running"
                                                                    }
                                                                } else {
                                                                    div { class: "badge badge-ghost gap-sm",
                                                                        "Stopped"
                                                                    }
                                                                }
                                                            }
                                                            td {
                                                                div { class: "font-bold", "{channel.name}" }
                                                                div { class: "text-caption text-tertiary", "{channel.models}" }
                                                            }
                                                            td {
                                                                "1"
                                                            }
                                                            td {
                                                                div { class: "flex gap-sm",
                                                                    {
                                                                        let c_stop = channel.clone();
                                                                        let c_start = channel.clone();
                                                                        if channel.status == 1 {
                                                                            rsx! {
                                                                                button {
                                                                                    class: "btn btn-sm btn-warning",
                                                                                    onclick: move |_| handle_toggle_status(c_stop.clone()),
                                                                                    "Stop"
                                                                                }
                                                                            }
                                                                        } else {
                                                                            rsx! {
                                                                                button {
                                                                                    class: "btn btn-sm btn-success",
                                                                                    onclick: move |_| handle_toggle_status(c_start.clone()),
                                                                                    "Start"
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    {
                                                                        let c_delete = channel.clone();
                                                                        rsx! {
                                                                            button {
                                                                                class: "btn btn-sm btn-error btn-outline",
                                                                                onclick: move |_| {
                                                                                    delete_channel_id.set(c_delete.id);
                                                                                    delete_channel_name.set(c_delete.name.clone());
                                                                                    is_delete_modal_open.set(true);
                                                                                },
                                                                                "Delete"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            None => rsx! {
                                div { class: "flex flex-col items-center justify-center h-full gap-md pb-xxl animate-pulse",
                                    style: "opacity: 0.5;",
                                    div { class: "w-12 h-12 rounded-full", style: "background: var(--bc-bg-hover);" }
                                    div { class: "text-caption font-medium text-secondary", "正在搜索神经网络..." }
                                }
                            }
                        }
                    }
                }

            // Modal (Custom Implementation for stability)
            if is_modal_open() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-0 sm:p-4",
                    // Backdrop (Global Blur)
                    div {
                        class: "absolute inset-0 transition-opacity",
                        style: "background: rgba(0,0,0,0.30); backdrop-filter: blur(5px); -webkit-backdrop-filter: blur(5px);",
                        onclick: move |_| is_modal_open.set(false)
                    }

                    // Modal Content
                    div {
                        class: "relative w-full h-full sm:h-auto sm:max-h-[90vh] sm:max-w-2xl flex flex-col overflow-hidden animate-scale-in pointer-events-auto overscroll-contain",
                        style: "background: var(--bc-bg-card-solid); border-radius: var(--bc-radius-lg); box-shadow: var(--bc-shadow-xl); border: 1px solid var(--bc-border);",
                        onclick: |e| e.stop_propagation(),

                        // Header
                        div { class: "flex justify-between items-center px-md py-sm sm:px-lg sm:py-md border-b shrink-0",
                            style: "background: var(--bc-bg-card-solid);",
                            h3 { class: "text-subtitle font-bold text-primary tracking-tight",
                                if modal_step() == 0 { "选择供应商" } else { "配置连接" }
                            }
                            button {
                                class: "btn btn-sm btn-circle btn-ghost text-secondary",
                                onclick: move |_| is_modal_open.set(false),
                                "✕"
                            }
                        }

                        // Body
                        div { class: "flex-1 overflow-y-auto p-md sm:p-lg min-h-0 overscroll-y-contain",
                            if modal_step() == 0 {
                                // Step 1: Provider Selection Grid
                                div { class: "grid grid-cols-2 sm:grid-cols-3 gap-md",
                                    for p in [ProviderType::OpenAI, ProviderType::Anthropic, ProviderType::Google, ProviderType::Aws, ProviderType::Azure, ProviderType::Local] {
                                        button {
                                            class: "bc-card-solid group flex flex-col items-center justify-center gap-md p-lg h-36 transition-all duration-300 ease-out cursor-pointer",
                                            style: "cursor: pointer;",
                                            onclick: move |_| select_provider(p),
                                            // Icon render
                                            div { class: "text-secondary group-hover:text-primary transition-colors duration-300 transform group-hover:scale-110",
                                                {p.icon()}
                                            }
                                            span { class: "font-medium text-caption text-secondary group-hover:text-primary", "{p.name()}" }
                                        }
                                    }
                                }
                            } else {
                                // Step 2: Configuration Form
                                div { class: "flex flex-col gap-md",
                                    BCInput {
                                        label: Some("连接名称".to_string()),
                                        value: "{form_name}",
                                        oninput: move |e: FormEvent| form_name.set(e.value())
                                    }

                                    if selected_provider() == ProviderType::OpenAI {
                                        BCInput {
                                            label: Some("API Key".to_string()),
                                            value: "{form_key}",
                                            placeholder: "sk-...".to_string(),
                                            error: if !form_key().is_empty() && !form_key().starts_with("sk-") { Some("OpenAI Key 通常以 sk- 开头".to_string()) } else { None },
                                            oninput: move |e: FormEvent| form_key.set(e.value())
                                        }
                                    }

                                    if selected_provider() == ProviderType::Anthropic {
                                        BCInput {
                                            label: Some("API Key".to_string()),
                                            value: "{form_key}",
                                            placeholder: "sk-ant-...".to_string(),
                                            oninput: move |e: FormEvent| form_key.set(e.value())
                                        }
                                    }

                                    if selected_provider() == ProviderType::Google {
                                        div { class: "flex flex-col gap-sm mb-sm",
                                            label { class: "text-body font-medium text-secondary", "认证类型 (Auth Type)" }
                                            div { class: "join w-full",
                                                button {
                                                    class: if form_google_auth_type() == "api_key" { "join-item btn btn-sm btn-primary flex-1" } else { "join-item btn btn-sm btn-ghost flex-1" },
                                                    onclick: move |_| form_google_auth_type.set("api_key".to_string()),
                                                    "Gemini API"
                                                }
                                                button {
                                                    class: if form_google_auth_type() == "vertex" { "join-item btn btn-sm btn-primary flex-1" } else { "join-item btn btn-sm btn-ghost flex-1" },
                                                    onclick: move |_| form_google_auth_type.set("vertex".to_string()),
                                                    "Vertex AI"
                                                }
                                            }
                                        }

                                        if form_google_auth_type() == "api_key" {
                                            BCInput {
                                                label: Some("API Key".to_string()),
                                                value: "{form_key}",
                                                placeholder: "AIza...".to_string(),
                                                oninput: move |e: FormEvent| form_key.set(e.value())
                                            }
                                        } else {
                                            div { class: "flex flex-col gap-xs",
                                                label { class: "text-body font-medium text-secondary",
                                                    "Service Account JSON Key"
                                                    span { class: "text-xxs font-normal text-tertiary ml-sm", "(Copied from Google Cloud Console)" }
                                                }
                                                textarea {
                                                    class: "textarea textarea-bordered h-32 text-xs font-mono leading-tight",
                                                    style: "background: var(--bc-bg-card-solid); border-color: var(--bc-border);",
                                                    placeholder: "{{\n  \"type\": \"service_account\",\n  \"project_id\": ...\n}}",
                                                    value: "{form_key}",
                                                    oninput: move |e| form_key.set(e.value())
                                                }
                                            }

                                            div { class: "grid grid-cols-2 gap-md",
                                                div { class: "flex flex-col gap-xs",
                                                    label { class: "text-body font-medium text-secondary", "区域 (Region)" }
                                                    select { class: "select select-bordered w-full select-sm",
                                                        style: "background: var(--bc-bg-card-solid); border-color: var(--bc-border);",
                                                        value: "{form_google_region}",
                                                        onchange: move |e: FormEvent| form_google_region.set(e.value()),
                                                        option { value: "us-central1", "US Central (Iowa)" }
                                                        option { value: "us-east4", "US East (N. Virginia)" }
                                                        option { value: "us-west1", "US West (Oregon)" }
                                                        option { value: "asia-northeast1", "Asia (Tokyo)" }
                                                        option { value: "asia-southeast1", "Asia (Singapore)" }
                                                        option { value: "europe-west1", "Europe (Belgium)" }
                                                    }
                                                }
                                                BCInput {
                                                    label: Some("Project ID (Optional)".to_string()),
                                                    value: "{form_google_project_id}",
                                                    placeholder: "Override JSON project_id".to_string(),
                                                    oninput: move |e: FormEvent| form_google_project_id.set(e.value())
                                                }
                                            }
                                        }
                                    }

                                    if selected_provider() == ProviderType::Aws {
                                        div { class: "alert alert-info text-xs",
                                            style: "background: var(--bc-info-light); color: var(--bc-info);",
                                            "注意: 您的密钥仅保存在本地，且通过 SigV4 签名请求，我们不会存储明文。"
                                        }
                                        BCInput {
                                            label: Some("Access Key ID".to_string()),
                                            value: "{form_key}",
                                            placeholder: "AKIA...".to_string(),
                                            error: if !form_key().is_empty() && !form_key().starts_with("AKIA") && !form_key().starts_with("ASIA") { Some("无效的 Access Key ID 格式".to_string()) } else { None },
                                            oninput: move |e: FormEvent| form_key.set(e.value())
                                        }
                                        BCInput {
                                            label: Some("Secret Access Key".to_string()),
                                            value: "{form_aws_sk}",
                                            placeholder: "wJalrX...".to_string(),
                                            oninput: move |e: FormEvent| form_aws_sk.set(e.value())
                                        }
                                        div { class: "flex flex-col gap-xs",
                                            label { class: "text-body font-medium text-secondary", "区域 (Region)" }
                                            select { class: "select select-bordered w-full select-sm",
                                                style: "background: var(--bc-bg-card-solid); border-color: var(--bc-border);",
                                                onchange: move |e: FormEvent| form_aws_region.set(e.value()),
                                                option { value: "us-east-1", "US East (N. Virginia)" }
                                                option { value: "us-west-2", "US West (Oregon)" }
                                                option { value: "ap-northeast-1", "Asia Pacific (Tokyo)" }
                                                option { value: "eu-central-1", "Europe (Frankfurt)" }
                                            }
                                        }
                                        BCInput {
                                            label: Some("Model ID".to_string()),
                                            value: "{form_aws_model_id}",
                                            placeholder: "anthropic.claude-sonnet-4-5...".to_string(),
                                            oninput: move |e: FormEvent| form_aws_model_id.set(e.value())
                                        }
                                    }

                                    if selected_provider() == ProviderType::Azure {
                                        BCInput {
                                            label: Some("Resource Name".to_string()),
                                            value: "{form_azure_resource}",
                                            placeholder: "my-openai-resource".to_string(),
                                            oninput: move |e: FormEvent| form_azure_resource.set(e.value())
                                        }
                                        BCInput {
                                            label: Some("Deployment Name".to_string()),
                                            value: "{form_azure_deployment}",
                                            placeholder: "gpt-4-deployment".to_string(),
                                            oninput: move |e: FormEvent| form_azure_deployment.set(e.value())
                                        }
                                        BCInput {
                                            label: Some("API Key".to_string()),
                                            value: "{form_key}",
                                            placeholder: "32-char hex string".to_string(),
                                            oninput: move |e: FormEvent| form_key.set(e.value())
                                        }
                                        div { class: "flex flex-col gap-xs",
                                            label { class: "text-body font-medium text-secondary", "API Version" }
                                            select { class: "select select-bordered w-full select-sm",
                                                style: "background: var(--bc-bg-card-solid); border-color: var(--bc-border);",
                                                onchange: move |e: FormEvent| form_azure_api_version.set(e.value()),
                                                option { value: "2023-05-15", "2023-05-15" }
                                                option { value: "2023-12-01-preview", "2023-12-01-preview" }
                                                option { value: "2024-02-15-preview", "2024-02-15-preview" }
                                            }
                                        }
                                    }

                                    if selected_provider() == ProviderType::Local {
                                        BCInput {
                                            label: Some("Local Server URL".to_string()),
                                            value: "{form_local_url}",
                                            placeholder: "http://localhost:8080".to_string(),
                                            oninput: move |e: FormEvent| form_local_url.set(e.value())
                                        }
                                    }
                                }
                            }
                        }

                        // Footer
                        div { class: "flex justify-end gap-md px-lg py-md border-t shrink-0",
                            style: "background: var(--bc-bg-hover);",
                            if modal_step() == 1 {
                                BCButton {
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| modal_step.set(0),
                                    "上一步"
                                }
                                BCButton {
                                    class: "btn-ghost text-success",
                                    onclick: move |_| {
                                        spawn(async move {
                                            is_loading.set(true);
                                            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                                            is_loading.set(false);
                                            toast.success("连接测试成功: 延迟 45ms");
                                        });
                                    },
                                    "⚡ 测试连接"
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

            // Delete Confirmation Modal
            if is_delete_modal_open() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-md",
                    div {
                        class: "absolute inset-0 transition-opacity",
                        style: "background: rgba(0,0,0,0.30); backdrop-filter: blur(5px); -webkit-backdrop-filter: blur(5px);",
                        onclick: move |_| is_delete_modal_open.set(false)
                    }

                    // Modal Content
                    div {
                        class: "relative w-full max-w-md overflow-hidden animate-scale-in",
                        style: "background: var(--bc-bg-card-solid); border-radius: var(--bc-radius-lg); box-shadow: var(--bc-shadow-xl); border: 1px solid var(--bc-border);",
                        onclick: |e| e.stop_propagation(),

                        // Header with Warning Icon
                        div { class: "flex items-center gap-md px-lg py-lg border-b",
                            style: "background: var(--bc-danger-light); border-color: var(--bc-danger-light);",
                            div { class: "w-12 h-12 rounded-full flex items-center justify-center",
                                style: "background: var(--bc-danger-light);",
                                svg { class: "w-6 h-6", style: "color: var(--bc-danger);", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
                                }
                            }
                            div { class: "flex-1",
                                h3 { class: "text-subtitle font-bold text-primary", "确认删除" }
                                p { class: "text-caption text-secondary mt-xs", "此操作无法撤销" }
                            }
                        }

                        // Message
                        div { class: "px-lg py-md",
                            p { class: "text-secondary",
                                "确定要删除连接 \""
                                span { class: "font-semibold text-primary", "{delete_channel_name()}" }
                                "\" 吗？删除后所有相关配置将被永久清除。"
                            }
                        }

                        // Footer
                        div { class: "flex justify-end gap-md px-lg py-md border-t",
                            style: "background: var(--bc-bg-hover);",
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
}
