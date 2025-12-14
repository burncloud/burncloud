use burncloud_client_shared::{ChannelDto, API_CLIENT};
use dioxus::prelude::*;

#[component]
pub fn ApiManagement() -> Element {
    // Resource to fetch channels
    let mut channels =
        use_resource(move || async move { API_CLIENT.list_channels().await.unwrap_or_default() });

    let mut show_create_modal = use_signal(|| false);

    // Delete handler
    let delete_channel = move |id: String| async move {
        if API_CLIENT.delete_channel(&id).await.is_ok() {
            channels.restart();
        }
    };

    rsx! {
        div { class: "w-full h-full flex flex-col bg-base-100",
            // Sticky Header with Glassmorphism
            div { class: "sticky top-0 z-40 border-b border-base-200 bg-base-100/80 backdrop-blur-md",
                div { class: "flex justify-between items-center max-w-7xl mx-auto px-8 py-6",
                    div {
                        h1 { class: "text-2xl font-bold tracking-tight text-base-content m-0", "渠道管理" }
                        p { class: "text-base-content/60 mt-1 text-sm", "配置和管理上游模型服务商 (Providers)" }
                    }
                    div { class: "flex items-center gap-3",
                        // Subtle Refresh Button
                        button {
                            class: "btn btn-ghost btn-circle btn-sm tooltip tooltip-bottom",
                            "data-tip": "刷新列表",
                            onclick: move |_| { channels.restart(); },
                            // SVG Icon for Refresh
                            svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                            }
                        }
                        // Primary Action
                        button {
                            class: "btn btn-primary btn-sm px-4 shadow-sm",
                            onclick: move |_| { show_create_modal.set(true); },
                            span { class: "mr-1", "+" }
                            "新建渠道"
                        }
                    }
                }
            }

            // Main Content Area
            div { class: "flex-1 overflow-auto bg-base-200/30 p-8",
                div { class: "max-w-7xl mx-auto",
                    div { class: "card bg-base-100 shadow-sm border border-base-200 overflow-hidden",
                        // Table Header
                        div { class: "grid grid-cols-12 gap-4 p-4 border-b border-base-200 bg-base-50/50 text-xs font-semibold text-base-content/50 uppercase tracking-wider",
                            div { class: "col-span-4", "名称 / 提供商" }
                            div { class: "col-span-5", "API 端点 & 鉴权" }
                            div { class: "col-span-2", "状态" }
                            div { class: "col-span-1 text-right", "操作" }
                        }

                        // Table Body
                        if let Some(list) = channels.read().as_ref() {
                            div { class: "flex flex-col divide-y divide-base-200",
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
                                    div { class: "flex flex-col items-center justify-center py-20 text-center",
                                        div { class: "w-16 h-16 bg-base-200 rounded-full flex items-center justify-center mb-4",
                                            svg { class: "w-8 h-8 text-base-content/30", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" }
                                            }
                                        }
                                        h3 { class: "text-lg font-medium text-base-content", "暂无渠道" }
                                        p { class: "text-sm text-base-content/60 max-w-xs mt-1", "添加您的第一个模型渠道以开始路由流量。" }
                                    }
                                }
                            }
                        } else {
                            div { class: "flex justify-center items-center py-20",
                                span { class: "loading loading-spinner loading-md text-primary" }
                            }
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
        div { class: "grid grid-cols-12 gap-4 p-4 items-center hover:bg-base-50 transition-colors duration-200 group",
            // Name Column
            div { class: "col-span-4 flex items-center gap-3",
                div { class: "w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary",
                    // Provider Icon Placeholder (First letter)
                    span { class: "font-bold text-sm", "{&channel.name[0..1].to_uppercase()}" }
                }
                div {
                    div { class: "font-medium text-sm text-base-content", "{channel.name}" }
                    div { class: "text-xs text-base-content/50", "ID: {&channel.id[0..8]}..." }
                }
            }

            // Endpoint Column
            div { class: "col-span-5 flex flex-col justify-center",
                div { class: "flex items-center gap-2",
                    span { class: "badge badge-ghost badge-xs font-mono", "{channel.auth_type}" }
                }
                div { class: "text-xs text-base-content/60 font-mono mt-1 truncate", title: "{channel.base_url}",
                    "{channel.base_url}"
                }
            }

            // Status Column
            div { class: "col-span-2",
                div { class: "flex items-center gap-2",
                    span { class: "w-2 h-2 rounded-full bg-success ring-4 ring-success/20" }
                    span { class: "text-sm font-medium text-base-content/80", "Active" }
                }
            }

            // Actions Column
            div { class: "col-span-1 flex justify-end",
                button {
                    class: "btn btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100 transition-opacity",
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
        div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 sm:p-6",
            // Backdrop
            div {
                class: "absolute inset-0 bg-black/40 backdrop-blur-sm transition-opacity",
                onclick: move |_| on_close.call(())
            }

            // Modal Panel
            div { class: "relative bg-base-100 rounded-xl shadow-2xl w-full max-w-lg overflow-hidden border border-base-200 transform transition-all",
                // Header
                div { class: "px-6 py-4 border-b border-base-200 bg-base-50/50 flex justify-between items-center",
                    div {
                        h2 { class: "text-lg font-bold text-base-content", "新建渠道" }
                        p { class: "text-xs text-base-content/60", "添加新的 API 上游提供商" }
                    }
                    button {
                        class: "btn btn-sm btn-circle btn-ghost",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Body
                div { class: "p-6 flex flex-col gap-5",
                    div { class: "form-control w-full",
                        label { class: "label pt-0", span { class: "label-text font-medium text-base-content", "名称 (Name)" } }
                        input { class: "input input-bordered w-full focus:input-primary transition-all bg-base-50 focus:bg-base-100", value: "{name}", oninput: move |e| name.set(e.value()), placeholder: "例如: OpenAI Main" }
                    }

                    div { class: "grid grid-cols-2 gap-4",
                        div { class: "form-control w-full",
                            label { class: "label pt-0", span { class: "label-text font-medium text-base-content", "Base URL" } }
                            input { class: "input input-bordered w-full focus:input-primary font-mono text-sm bg-base-50 focus:bg-base-100", value: "{base_url}", oninput: move |e| base_url.set(e.value()) }
                        }
                        div { class: "form-control w-full",
                            label { class: "label pt-0", span { class: "label-text font-medium text-base-content", "鉴权类型" } }
                            select { class: "select select-bordered w-full focus:select-primary bg-base-50 focus:bg-base-100", value: "{auth_type}", onchange: move |e| auth_type.set(e.value()),
                                option { value: "Bearer", "Bearer Token (OpenAI)" }
                                option { value: "XApiKey", "X-Api-Key (Claude)" }
                                option { value: "GoogleAI", "Google AI" }
                                option { value: "Azure", "Azure OpenAI" }
                                option { value: "AwsSigV4", "AWS Bedrock" }
                            }
                        }
                    }

                    div { class: "form-control w-full",
                        label { class: "label pt-0", span { class: "label-text font-medium text-base-content", "API Key" } }
                        div { class: "relative",
                            input { class: "input input-bordered w-full focus:input-primary font-mono text-sm pr-10 bg-base-50 focus:bg-base-100", type: "password", value: "{api_key}", oninput: move |e| api_key.set(e.value()), placeholder: "sk-..." }
                            div { class: "absolute inset-y-0 right-0 flex items-center pr-3 pointer-events-none text-base-content/40",
                                svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" }
                                }
                            }
                        }
                    }

                    div { class: "form-control w-full",
                        label { class: "label pt-0", span { class: "label-text font-medium text-base-content", "匹配路径 (Match Path)" } }
                        input { class: "input input-bordered w-full focus:input-primary font-mono text-sm bg-base-50 focus:bg-base-100", value: "{match_path}", oninput: move |e| match_path.set(e.value()) }
                    }
                }

                // Footer
                div { class: "px-6 py-4 bg-base-50/50 border-t border-base-200 flex justify-end gap-3",
                    button { class: "btn btn-ghost", onclick: move |_| on_close.call(()), "取消" }
                    button { class: "btn btn-primary px-8 shadow-sm", onclick: handle_submit, "创建渠道" }
                }
            }
        }
    }
}
