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
        div { class: "w-full h-full flex flex-col",
            style: "background: var(--bc-bg-canvas);",

            // Sticky header
            div { class: "sticky top-0 z-40 border-b",
                style: "background: var(--bc-bg-card-solid); border-color: var(--bc-border);",
                div { class: "flex justify-between items-center px-xxl py-lg",
                    div {
                        h1 { class: "text-large-title font-bold text-primary m-0", "渠道管理" }
                        p { class: "text-caption text-secondary mt-xs", "配置和管理上游模型服务商 (Providers)" }
                    }
                    div { class: "flex items-center gap-sm",
                        button {
                            class: "btn btn-ghost btn-circle",
                            title: "刷新列表",
                            onclick: move |_| { channels.restart(); },
                            svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                            }
                        }
                        button {
                            class: "btn btn-primary btn-sm",
                            onclick: move |_| { show_create_modal.set(true); },
                            span { class: "mr-xs", "+" }
                            "新建渠道"
                        }
                    }
                }
            }

            // Main content
            div { class: "flex-1 overflow-auto p-xxl",
                div { class: "bc-card-solid overflow-hidden",
                    // Table header
                    div { class: "grid grid-cols-12 gap-md p-md border-b text-xxs font-semibold text-tertiary uppercase tracking-wider",
                        style: "background: var(--bc-bg-hover);",
                        div { class: "col-span-4", "名称 / 提供商" }
                        div { class: "col-span-5", "API 端点 & 鉴权" }
                        div { class: "col-span-2", "状态" }
                        div { class: "col-span-1 text-right", "操作" }
                    }

                    // Table body
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
                                    div { class: "flex items-center justify-center mb-md",
                                        style: "width: 64px; height: 64px; border-radius: 9999px; background: var(--bc-bg-hover);",
                                        svg { class: "w-8 h-8", style: "color: var(--bc-text-tertiary);", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" }
                                        }
                                    }
                                    h3 { class: "text-subtitle font-medium text-primary", "暂无渠道" }
                                    p { class: "text-caption text-secondary mt-xs", "添加您的第一个模型渠道以开始路由流量。" }
                                }
                            }
                        }
                    } else {
                        div { class: "flex justify-center items-center py-xxxl",
                            span { class: "loading loading-spinner loading-md" }
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
}

#[component]
fn ChannelRow(channel: ChannelDto, on_delete: EventHandler<String>) -> Element {
    rsx! {
        div { class: "grid grid-cols-12 gap-md p-md items-center border-t group",
            style: "transition: background var(--bc-transition-fast);",
            // Name column
            div { class: "col-span-4 flex items-center gap-md",
                div { class: "flex items-center justify-center font-bold text-caption",
                    style: "width: 40px; height: 40px; border-radius: var(--bc-radius-sm); background: var(--bc-primary-light); color: var(--bc-primary);",
                    "{&channel.name[0..1].to_uppercase()}"
                }
                div {
                    div { class: "font-medium text-caption text-primary", "{channel.name}" }
                    div { class: "text-xxs text-tertiary", "ID: {&channel.id[0..8]}..." }
                }
            }

            // Endpoint column
            div { class: "col-span-5 flex flex-col justify-center",
                div { class: "flex items-center gap-sm",
                    span { class: "badge badge-ghost badge-xs", style: "font-family: monospace;", "{channel.auth_type}" }
                }
                div { class: "text-xxs text-secondary mt-xs truncate", style: "font-family: monospace;", title: "{channel.base_url}",
                    "{channel.base_url}"
                }
            }

            // Status column
            div { class: "col-span-2",
                div { class: "flex items-center gap-sm",
                    span { style: "width: 8px; height: 8px; border-radius: 9999px; background: var(--bc-success);" }
                    span { class: "text-caption font-medium text-primary", "Active" }
                }
            }

            // Actions column
            div { class: "col-span-1 flex justify-end",
                button {
                    class: "btn btn-ghost btn-xs",
                    style: "color: var(--bc-danger);",
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
fn CreateChannelModal(on_close: EventHandler<()>, on_create: EventHandler<()>) -> Element {
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
        div { class: "fixed inset-0 z-50 flex items-center justify-center p-md",
            // Backdrop
            div {
                class: "absolute inset-0",
                style: "background: rgba(0, 0, 0, 0.40); backdrop-filter: blur(4px);",
                onclick: move |_| on_close.call(())
            }

            // Modal panel
            div { class: "bc-card-solid relative w-full overflow-hidden",
                style: "max-width: 560px; box-shadow: var(--bc-shadow-xl);",
                // Header
                div { class: "flex justify-between items-center px-lg py-md border-b",
                    style: "background: var(--bc-bg-hover); border-color: var(--bc-border);",
                    div {
                        h2 { class: "text-subtitle font-bold text-primary m-0", "新建渠道" }
                        p { class: "text-xxs text-secondary", "添加新的 API 上游提供商" }
                    }
                    button {
                        class: "btn btn-ghost btn-circle btn-sm",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Body
                div { class: "p-lg flex flex-col gap-md",
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-medium", "名称 (Name)" }
                        }
                        input {
                            class: "input",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                            placeholder: "例如: OpenAI Main"
                        }
                    }

                    div { class: "grid grid-cols-2 gap-md",
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text font-medium", "Base URL" }
                            }
                            input {
                                class: "input",
                                style: "font-family: monospace; font-size: var(--bc-font-sm);",
                                value: "{base_url}",
                                oninput: move |e| base_url.set(e.value())
                            }
                        }
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text font-medium", "鉴权类型" }
                            }
                            select {
                                class: "select",
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

                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-medium", "API Key" }
                        }
                        input {
                            class: "input",
                            style: "font-family: monospace; font-size: var(--bc-font-sm);",
                            r#type: "password",
                            value: "{api_key}",
                            oninput: move |e| api_key.set(e.value()),
                            placeholder: "sk-..."
                        }
                    }

                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-medium", "匹配路径 (Match Path)" }
                        }
                        input {
                            class: "input",
                            style: "font-family: monospace; font-size: var(--bc-font-sm);",
                            value: "{match_path}",
                            oninput: move |e| match_path.set(e.value())
                        }
                    }
                }

                // Footer
                div { class: "flex justify-end gap-sm px-lg py-md border-t",
                    style: "background: var(--bc-bg-hover); border-color: var(--bc-border);",
                    button { class: "btn btn-ghost", onclick: move |_| on_close.call(()), "取消" }
                    button { class: "btn btn-primary", onclick: handle_submit, "创建渠道" }
                }
            }
        }
    }
}
