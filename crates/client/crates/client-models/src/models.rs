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
    AWS = 99,   // Custom mapping for UI
    Azure = 98, // Custom mapping for UI
    Local = 97, // Custom mapping for UI
}

impl ProviderType {
    fn name(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Anthropic => "Anthropic",
            ProviderType::Google => "Google Gemini",
            ProviderType::AWS => "AWS Bedrock",
            ProviderType::Azure => "Azure OpenAI",
            ProviderType::Local => "Local / GGUF",
        }
    }

    fn icon(&self) -> &'static str {
        // Simple SVG paths or unicode for now
        match self {
            ProviderType::OpenAI => "https://upload.wikimedia.org/wikipedia/commons/4/4d/OpenAI_Logo.svg", // Placeholder, will use text if image fails
            _ => "",
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
    // Azure specific
    let mut form_azure_resource = use_signal(String::new);
    let mut form_azure_deployment = use_signal(String::new);
    let mut form_azure_api_version = use_signal(|| "2023-05-15".to_string());
    // Local specific
    let mut form_local_url = use_signal(|| "http://localhost:8080".to_string());

    let open_create_modal = move |_| {
        form_id.set(0);
        modal_step.set(0); // Start at selection
        // Reset fields
        form_name.set(String::new());
        form_key.set(String::new());
        form_aws_sk.set(String::new());
        form_aws_region.set("us-east-1".to_string());
        form_azure_resource.set(String::new());
        form_azure_deployment.set(String::new());
        is_modal_open.set(true);
    };

    let mut select_provider = move |p: ProviderType| {
        selected_provider.set(p);
        form_name.set(p.name().to_string()); // Default name
        modal_step.set(1);
    };

    let handle_save = move |_| {
        spawn(async move {
            is_loading.set(true);

            let provider = selected_provider();
            let mut final_type = provider as i32;
            let mut final_base_url = String::new();
            let mut final_models = String::new();
            let mut final_param_override = None;
            let mut final_header_override = None;
            let mut final_key = form_key();

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
                    final_type = 24;
                    final_base_url = "https://generativelanguage.googleapis.com".to_string();
                    final_models = "gemini-pro,gemini-1.5-pro".to_string();
                }
                ProviderType::AWS => {
                    // Map AWS to Custom type for now, or Reuse 1 (OpenAI compatible) if router handles it?
                    // Assuming Router handles AWS SigV4 via a specific flag.
                    // For now, we use type=1 (OpenAI) but with backend magic, OR type=99 if backend supports it.
                    // Reverting to generic type=1 for "OpenAI Compatible" interface usually used by adapters
                    final_type = 1; 
                    final_base_url = format!("https://bedrock-runtime.{}.amazonaws.com", form_aws_region());
                    final_models = "anthropic.claude-3-sonnet-20240229-v1:0".to_string();
                    
                    // Pack secret into params
                    let params = json!({
                        "aws_secret_key": form_aws_sk(),
                        "region": form_aws_region(),
                        "auth_type": "aws_sigv4"
                    });
                    final_param_override = Some(params.to_string());
                }
                ProviderType::Azure => {
                    final_type = 1; // Azure is often OpenAI compatible
                    // https://{resource}.openai.azure.com/openai/deployments/{deployment}
                    final_base_url = format!(
                        "https://{}.openai.azure.com/openai/deployments/{}",
                        form_azure_resource(),
                        form_azure_deployment()
                    );
                    final_models = form_azure_deployment(); // Model name usually matches deployment
                    
                    let params = json!({
                        "api_version": form_azure_api_version(),
                        "auth_type": "azure_ad"
                    });
                    final_param_override = Some(params.to_string());
                    
                    let headers = json!({
                        "api-key": form_key() // Azure expects api-key header, not Bearer usually
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

    let channels_data = channels.read().clone();

    rsx! {
        div { class: "flex flex-col h-full gap-8",
            // Header
            div { class: "flex justify-between items-end px-1",
                div {
                    h1 { class: "text-2xl font-semibold text-base-content mb-1 tracking-tight", "模型网络" }
                    p { class: "text-sm text-base-content/60 font-medium", "您的 AI 算力中枢" }
                }
                div { class: "flex gap-3",
                    BCButton {
                        class: "btn-neutral btn-sm px-6 shadow-sm text-white",
                        onclick: open_create_modal,
                        "添加连接"
                    }
                }
            }

            // Cards Grid
            div { class: "flex-1 overflow-y-auto min-h-0", // Scroll container
                match channels_data {
                    Some(list) => rsx! {
                        div { class: "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6 pb-10",
                            for channel in list {
                                div { class: "group relative flex flex-col justify-between p-6 h-[200px] bg-base-100 rounded-2xl border border-base-200 hover:border-base-300 hover:shadow-[0_8px_30px_rgb(0,0,0,0.04)] transition-all duration-300 ease-out cursor-default",
                                    // Status Indicator (Breathing Light)
                                    div { class: "absolute top-6 right-6",
                                        if channel.status == 1 {
                                            span { class: "relative flex h-3 w-3",
                                                span { class: "animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" }
                                                span { class: "relative inline-flex rounded-full h-3 w-3 bg-emerald-500" }
                                            }
                                        } else {
                                            span { class: "h-3 w-3 rounded-full bg-base-300" }
                                        }
                                    }

                                    // Card Header
                                    div {
                                        div { class: "text-[10px] font-bold tracking-widest text-base-content/30 uppercase mb-3",
                                            match channel.type_ {
                                                1 => "OpenAI / Bedrock / Azure",
                                                14 => "Anthropic",
                                                24 => "Google",
                                                _ => "Custom"
                                            }
                                        }
                                        h3 { class: "text-xl font-bold text-base-content tracking-tight leading-tight pr-4", "{channel.name}" }
                                    }

                                    // Card Footer
                                    div { class: "flex items-end justify-between mt-4",
                                        div { class: "flex flex-col gap-1.5",
                                            span { class: "text-xs text-base-content/40 font-semibold tracking-wide", "AVAILABLE MODELS" }
                                            div { class: "font-mono text-xs text-base-content/70 bg-base-200/50 px-2 py-1 rounded w-fit max-w-[180px] truncate",
                                                "{channel.models}"
                                            }
                                        }

                                        // Actions (Delete)
                                        button {
                                            class: "btn btn-circle btn-sm btn-ghost text-base-content/20 hover:text-error hover:bg-error/5 transition-all opacity-0 group-hover:opacity-100 translate-y-2 group-hover:translate-y-0 duration-200",
                                            onclick: move |_| {
                                                delete_channel_id.set(channel.id);
                                                delete_channel_name.set(channel.name.clone());
                                                is_delete_modal_open.set(true);
                                            },
                                            title: "移除连接",
                                            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                                            }
                                        }
                                    }
                                }
                            }

                            // The "Add Connection" Card (Invitation)
                            div {
                                class: "flex flex-col items-center justify-center h-[200px] rounded-2xl border-2 border-dashed border-base-200 hover:border-primary/30 hover:bg-base-50/50 transition-all duration-300 cursor-pointer gap-4 group",
                                onclick: open_create_modal,
                                div { class: "h-12 w-12 rounded-full bg-base-100 group-hover:bg-white flex items-center justify-center shadow-sm border border-base-200 group-hover:scale-110 transition-transform duration-300",
                                    svg { class: "w-6 h-6 text-base-content/40 group-hover:text-primary transition-colors", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 4v16m8-8H4" }
                                    }
                                }
                                span { class: "text-sm font-semibold text-base-content/50 group-hover:text-primary transition-colors", "添加新连接" }
                            }
                        }
                    },
                    None => rsx! {
                        div { class: "flex flex-col items-center justify-center h-64 gap-4 opacity-50 animate-pulse",
                            div { class: "w-12 h-12 rounded-full bg-base-200" }
                            div { class: "text-sm font-medium", "正在搜索神经网络..." }
                        }
                    }
                }
            }

            // Modal (Custom Implementation for stability)
            if is_modal_open() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-0 sm:p-4",
                    // Backdrop
                    div {
                        class: "absolute inset-0 bg-black/50 backdrop-blur-md transition-opacity",
                        onclick: move |_| is_modal_open.set(false)
                    }

                    // Modal Content
                    div {
                        class: "fixed inset-0 sm:relative w-full h-full sm:h-auto sm:max-h-[90vh] sm:max-w-lg bg-base-100 sm:rounded-2xl shadow-2xl border border-base-200 flex flex-col overflow-hidden",
                        onclick: |e| e.stop_propagation(), // Prevent click through

                        // Header
                        div { class: "flex justify-between items-center px-6 py-4 border-b border-base-200 shrink-0 bg-base-100",
                            h3 { class: "text-lg font-bold text-base-content tracking-tight",
                                if modal_step() == 0 { "选择供应商" } else { "配置连接" }
                            }
                            button {
                                class: "btn btn-sm btn-circle btn-ghost text-base-content/50 hover:bg-base-200",
                                onclick: move |_| is_modal_open.set(false),
                                "✕"
                            }
                        }

                        // Form Body
                        div { class: "flex-1 overflow-y-auto p-6 min-h-0",
                            if modal_step() == 0 {
                                // Step 1: Provider Selection Grid
                                div { class: "grid grid-cols-2 gap-4",
                                    for p in [ProviderType::OpenAI, ProviderType::Anthropic, ProviderType::Google, ProviderType::AWS, ProviderType::Azure, ProviderType::Local] {
                                        button {
                                            class: "flex flex-col items-center justify-center gap-3 p-6 h-32 rounded-xl border border-base-200 bg-base-50/50 hover:border-primary/50 hover:bg-primary/5 hover:scale-[1.02] transition-all duration-200",
                                            onclick: move |_| select_provider(p),
                                            // Icon placeholder
                                            div { class: "text-2xl font-bold opacity-30",
                                                match p {
                                                    ProviderType::OpenAI | ProviderType::Azure | ProviderType::Local => "🤖",
                                                    ProviderType::Anthropic => "🧠",
                                                    ProviderType::Google => "🌟",
                                                    ProviderType::AWS => "☁️",
                                                }
                                            }
                                            span { class: "font-semibold text-sm", "{p.name()}" }
                                        }
                                    }
                                }
                            } else {
                                // Step 2: Configuration Form
                                div { class: "flex flex-col gap-4",
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
                                        BCInput {
                                            label: Some("API Key".to_string()),
                                            value: "{form_key}",
                                            placeholder: "AIza...".to_string(),
                                            oninput: move |e: FormEvent| form_key.set(e.value())
                                        }
                                    }

                                    if selected_provider() == ProviderType::AWS {
                                        div { class: "alert alert-info text-xs",
                                            "注意: 您的密钥仅保存在本地，且通过 SigV4 签名请求，我们不会存储明文。"
                                        }
                                        BCInput {
                                            label: Some("Access Key ID".to_string()),
                                            value: "{form_key}", 
                                            placeholder: "AKIA...".to_string(),
                                            oninput: move |e: FormEvent| form_key.set(e.value())
                                        }
                                        BCInput {
                                            label: Some("Secret Access Key".to_string()),
                                            value: "{form_aws_sk}",
                                            placeholder: "wJalrX...".to_string(),
                                            oninput: move |e: FormEvent| form_aws_sk.set(e.value())
                                        }
                                        div { class: "flex flex-col gap-1.5",
                                            label { class: "text-sm font-medium text-base-content/80", "区域 (Region)" }
                                            select { class: "select select-bordered w-full select-sm",
                                                onchange: move |e: FormEvent| form_aws_region.set(e.value()),
                                                option { value: "us-east-1", "US East (N. Virginia)" }
                                                option { value: "us-west-2", "US West (Oregon)" }
                                                option { value: "ap-northeast-1", "Asia Pacific (Tokyo)" }
                                                option { value: "eu-central-1", "Europe (Frankfurt)" }
                                            }
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
                                        div { class: "flex flex-col gap-1.5",
                                            label { class: "text-sm font-medium text-base-content/80", "API Version" }
                                            select { class: "select select-bordered w-full select-sm",
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
                        div { class: "flex justify-end gap-3 px-6 py-4 border-t border-base-200 bg-base-50/50 shrink-0",
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

            // Delete Confirmation Modal
            if is_delete_modal_open() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-4",
                    // Backdrop
                    div {
                        class: "absolute inset-0 bg-black/50 backdrop-blur-md transition-opacity",
                        onclick: move |_| is_delete_modal_open.set(false)
                    }

                    // Modal Content
                    div {
                        class: "relative w-full max-w-md bg-base-100 rounded-2xl shadow-2xl border border-base-200 overflow-hidden",
                        onclick: |e| e.stop_propagation(),

                        // Header with Warning Icon
                        div { class: "flex items-center gap-4 px-6 py-5 bg-error/5 border-b border-error/10",
                            div { class: "w-12 h-12 rounded-full bg-error/10 flex items-center justify-center",
                                svg { class: "w-6 h-6 text-error", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
                                }
                            }
                            div { class: "flex-1",
                                h3 { class: "text-lg font-bold text-base-content", "确认删除" }
                                p { class: "text-sm text-base-content/60 mt-1", "此操作无法撤销" }
                            }
                        }

                        // Message
                        div { class: "px-6 py-4",
                            p { class: "text-base-content/80",
                                "确定要删除连接 \""
                                span { class: "font-semibold text-base-content", "{delete_channel_name()}" }
                                "\" 吗？删除后所有相关配置将被永久清除。"
                            }
                        }

                        // Footer
                        div { class: "flex justify-end gap-3 px-6 py-4 bg-base-50/50 border-t border-base-200",
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
