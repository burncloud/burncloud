use burncloud_client_shared::components::{
    BCButton, BCInput, BCModal, ButtonSize, ButtonVariant, PageHeader,
};
use burncloud_client_shared::{ChannelDto, API_CLIENT};
use dioxus::prelude::*;

#[component]
pub fn ApiManagement() -> Element {
    let mut channels =
        use_resource(move || async move { API_CLIENT.list_channels().await.unwrap_or_default() });

    let mut show_create_modal = use_signal(|| false);

    let delete_channel = move |id: String| async move {
        if API_CLIENT.delete_channel(&id).await.is_ok() {
            channels.restart();
        }
    };

    rsx! {
        PageHeader {
            title: "渠道管理".to_string(),
            subtitle: Some("配置和管理上游模型服务商 (Providers)".to_string()),
            actions: rsx! {
                button {
                    class: "btn btn-ghost btn-circle",
                    title: "刷新列表",
                    onclick: move |_| { channels.restart(); },
                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                    }
                }
                BCButton {
                    size: ButtonSize::Small,
                    onclick: move |_| { show_create_modal.set(true); },
                    span { class: "mr-xs", "+" }
                    "新建渠道"
                }
            },
        }

        div { class: "page-content flex-1 min-h-0 overflow-auto",
                div { class: "bc-card-solid overflow-hidden",
                    div { class: "grid grid-cols-12 gap-md p-md border-b text-xxs font-semibold text-bc-text-tertiary uppercase tracking-wider bc-api-table-header",
                        div { class: "col-span-4", "名称 / 提供商" }
                        div { class: "col-span-5", "API 端点 & 鉴权" }
                        div { class: "col-span-2", "状态" }
                        div { class: "col-span-1 text-right", "操作" }
                    }

                    if let Some(list) = channels.read().as_ref() {
                        div { class: "flex flex-col",
                            for channel in list {
                                ChannelRow {
                                    key: "{channel.id}",
                                    channel: channel.clone(),
                                    on_delete: move |id| {
                                        spawn(async move {
                                            delete_channel(id).await;
                                        });
                                    }
                                }
                            }
                            if list.is_empty() {
                                div { class: "flex flex-col items-center justify-center py-xxxl text-center",
                                    div { class: "bc-empty-channel-icon flex items-center justify-center mb-md",
                                        svg { class: "w-8 h-8 text-bc-text-tertiary", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" }
                                        }
                                    }
                                    h3 { class: "text-subtitle font-medium text-bc-text", "暂无渠道" }
                                    p { class: "text-caption text-bc-text-secondary mt-xs", "添加您的第一个模型渠道以开始路由流量。" }
                                }
                            }
                        }
                    } else {
                        div { class: "flex justify-center items-center py-xxxl",
                            span { class: "bc-spinner bc-spinner--md" }
                        }
                    }
                }
            }

            CreateChannelModal {
                open: show_create_modal(),
                on_close: move |_| show_create_modal.set(false),
                on_create: move |_| {
                    show_create_modal.set(false);
                    channels.restart();
                },
            }
    }
}

#[component]
fn ChannelRow(channel: ChannelDto, on_delete: EventHandler<String>) -> Element {
    rsx! {
        div { class: "grid grid-cols-12 gap-md p-md items-center border-t group bc-channel-row",
            div { class: "col-span-4 flex items-center gap-md",
                div { class: "bc-avatar-sm font-bold text-caption",
                    "{&channel.name[0..1].to_uppercase()}"
                }
                div {
                    div { class: "font-medium text-caption text-bc-text", "{channel.name}" }
                    div { class: "text-xxs text-bc-text-tertiary", "ID: {&channel.id[0..8]}..." }
                }
            }

            div { class: "col-span-5 flex flex-col justify-center",
                div { class: "flex items-center gap-sm",
                    span { class: "inline-flex bc-badge-neutral bc-badge-compact bc-mono-sm", "{channel.auth_type}" }
                }
                div { class: "text-xxs text-bc-text-secondary mt-xs truncate bc-mono-sm", title: "{channel.base_url}",
                    "{channel.base_url}"
                }
            }

            div { class: "col-span-2",
                div { class: "flex items-center gap-sm",
                    span { class: "bc-status-dot" }
                    span { class: "text-caption font-medium text-bc-text", "Active" }
                }
            }

            div { class: "col-span-1 flex justify-end",
                button {
                    class: "btn btn-ghost btn-xs btn-danger-sm",
                    title: "删除渠道",
                    onclick: move |_| on_delete.call(channel.id.clone()),
                    svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                    }
                }
            }
        }
    }
}

#[component]
fn CreateChannelModal(
    open: bool,
    on_close: EventHandler<()>,
    on_create: EventHandler<()>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut base_url = use_signal(|| String::from("https://api.openai.com"));
    let mut api_key = use_signal(String::new);
    let mut match_path = use_signal(|| String::from("/v1/chat/completions"));
    let mut auth_type = use_signal(|| String::from("Bearer"));
    let id = use_signal(String::new);

    let handle_submit = move |_| {
        let new_channel = ChannelDto {
            id: if id().is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                id()
            },
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
                println!("Failed to create channel");
            }
        });
    };

    rsx! {
        BCModal {
            title: "新建渠道".to_string(),
            open,
            onclose: move |_| on_close.call(()),
            footer: rsx! {
                BCButton {
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| on_close.call(()),
                    "取消"
                }
                BCButton {
                    onclick: handle_submit,
                    "创建渠道"
                }
            },

            div { class: "flex flex-col gap-md",
                p { class: "bc-detail-line m-0", "添加新的 API 上游提供商" }

                BCInput {
                    label: Some("名称 (Name)".to_string()),
                    value: name(),
                    placeholder: "例如: OpenAI Main".to_string(),
                    oninput: move |e: FormEvent| name.set(e.value()),
                }

                div { class: "bc-grid-2col",
                    BCInput {
                        class: "bc-mono-sm".to_string(),
                        label: Some("Base URL".to_string()),
                        value: base_url(),
                        oninput: move |e: FormEvent| base_url.set(e.value()),
                    }
                    div { class: "bc-field",
                        label { class: "bc-field-label font-medium", "鉴权类型" }
                        select {
                            class: "select bc-mono-sm",
                            value: "{auth_type}",
                            onchange: move |e| auth_type.set(e.value()),
                            option { value: "Bearer", "Bearer Token (OpenAI)" }
                            option { value: "XApiKey", "X-Api-Key (Claude)" }
                            option { value: "GoogleAI", "Google AI" }
                            option { value: "Azure", "Azure OpenAI" }
                            option { value: "AwsSigV4", "AWS Bedrock" }
                        }
                    }
                }

                BCInput {
                    class: "bc-mono-sm".to_string(),
                    label: Some("API Key".to_string()),
                    r#type: "password".to_string(),
                    value: api_key(),
                    placeholder: "sk-...".to_string(),
                    oninput: move |e: FormEvent| api_key.set(e.value()),
                }

                BCInput {
                    class: "bc-mono-sm".to_string(),
                    label: Some("匹配路径 (Match Path)".to_string()),
                    value: match_path(),
                    oninput: move |e: FormEvent| match_path.set(e.value()),
                }
            }
        }
    }
}
