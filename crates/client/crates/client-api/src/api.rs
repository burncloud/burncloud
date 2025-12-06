use dioxus::prelude::*;
use burncloud_client_shared::{API_CLIENT, ChannelDto};

#[component]
pub fn ApiManagement() -> Element {
    // Resource to fetch channels
    let mut channels = use_resource(move || async move {
        API_CLIENT.list_channels().await.unwrap_or_default()
    });

    let mut show_create_modal = use_signal(|| false);

    // Delete handler
    let delete_channel = move |id: String| async move {
        if API_CLIENT.delete_channel(&id).await.is_ok() {
            channels.restart();
        }
    };

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0", "渠道管理 (Channels)" }
                    p { class: "text-secondary m-0 mt-sm", "管理上游模型渠道和API密钥" }
                }
                div { class: "flex gap-sm",
                    button { 
                        class: "btn btn-secondary",
                        onclick: move |_| { channels.restart(); },
                        "刷新"
                    }
                    button { 
                        class: "btn btn-primary",
                        onclick: move |_| { show_create_modal.set(true); },
                        "新建渠道"
                    }
                }
            }
        }

        div { class: "page-content",
            div { class: "card",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "已配置渠道" }
                    
                    if let Some(list) = channels.read().as_ref() {
                        div { class: "flex flex-col gap-md",
                            for channel in list {
                                ChannelRow { 
                                    key: "{channel.id}",
                                    channel: channel.clone(), 
                                    on_delete: move |id| {
                                        let delete_fn = delete_channel.clone(); // Clone closure logic? No, just spawn
                                        spawn(async move {
                                            delete_fn(id).await;
                                        });
                                    }
                                }
                            }
                            if list.is_empty() {
                                div { class: "text-center text-secondary p-lg", "暂无渠道，请添加。" }
                            }
                        }
                    } else {
                        div { class: "text-center p-lg", "加载中..." }
                    }
                }
            }
        }

        if show_create_modal() {
            CreateChannelModal {
                on_close: move |_| show_create_modal.set(false),
                on_create: move |_| {
                    show_create_modal.set(false);
                    channels.restart();
                }
            }
        }
    }
}

#[component]
fn ChannelRow(channel: ChannelDto, on_delete: EventHandler<String>) -> Element {
    rsx! {
        div { class: "flex justify-between items-center p-md border-b",
            div {
                div { class: "font-medium", "{channel.name}" }
                div { class: "text-caption text-secondary", "{channel.base_url} ({channel.auth_type})" }
            }
            div { class: "flex items-center gap-sm",
                span { class: "status-indicator status-running",
                    span { class: "status-dot" }
                    "Active"
                }
                button { 
                    class: "btn btn-sm btn-danger", 
                    onclick: move |_| on_delete.call(channel.id.clone()),
                    "删除" 
                }
            }
        }
    }
}

#[component]
fn CreateChannelModal(on_close: EventHandler<()>, on_create: EventHandler<()>) -> Element {
    let mut name = use_signal(|| String::new());
    let mut base_url = use_signal(|| String::from("https://api.openai.com"));
    let mut api_key = use_signal(|| String::new());
    let mut match_path = use_signal(|| String::from("/v1/chat/completions"));
    let mut auth_type = use_signal(|| String::from("Bearer"));
    let id = use_signal(|| String::new());

    let handle_submit = move |_| {
        let new_channel = ChannelDto {
            id: if id().is_empty() { uuid::Uuid::new_v4().to_string() } else { id() },
            name: name(),
            base_url: base_url(),
            api_key: api_key(),
            match_path: match_path(),
            auth_type: auth_type(),
            priority: 0,
        };

        spawn(async move {
            if API_CLIENT.create_channel(new_channel).await.is_ok() {
                on_create.call(());
            } else {
                // Handle error (todo: toast)
                println!("Failed to create channel");
            }
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            div { class: "card w-full max-w-md p-lg shadow-dialog",
                h2 { class: "text-subtitle font-bold mb-md", "新建渠道" }
                
                div { class: "flex flex-col gap-sm",
                    div { class: "form-group",
                        label { "名称 (Name)" }
                        input { class: "form-control", value: "{name}", oninput: move |e| name.set(e.value()) }
                    }
                    div { class: "form-group",
                        label { "Base URL" }
                        input { class: "form-control", value: "{base_url}", oninput: move |e| base_url.set(e.value()) }
                    }
                    div { class: "form-group",
                        label { "API Key" }
                        input { class: "form-control", type: "password", value: "{api_key}", oninput: move |e| api_key.set(e.value()) }
                    }
                    div { class: "form-group",
                        label { "匹配路径 (Match Path)" }
                        input { class: "form-control", value: "{match_path}", oninput: move |e| match_path.set(e.value()) }
                    }
                    div { class: "form-group",
                        label { "鉴权类型 (Auth Type)" }
                        select { class: "form-control", value: "{auth_type}", onchange: move |e| auth_type.set(e.value()),
                            option { value: "Bearer", "Bearer Token (OpenAI)" }
                            option { value: "XApiKey", "X-Api-Key (Claude)" }
                            option { value: "GoogleAI", "Google AI" }
                            option { value: "Azure", "Azure OpenAI" }
                            option { value: "AwsSigV4", "AWS Bedrock" }
                        }
                    }
                }

                div { class: "flex justify-end gap-sm mt-lg",
                    button { class: "btn btn-secondary", onclick: move |_| on_close.call(()), "取消" }
                    button { class: "btn btn-primary", onclick: handle_submit, "创建" }
                }
            }
        }
    }
}