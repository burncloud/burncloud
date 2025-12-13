use burncloud_client_shared::components::{
    BCBadge, BCButton, BCInput, BCModal, BadgeVariant, ButtonVariant,
};
use burncloud_client_shared::{ChannelDto, API_CLIENT};
use dioxus::prelude::*;

#[component]
pub fn ApiManagement() -> Element {
    // Resource to fetch channels
    let mut channels =
        use_resource(move || async move { API_CLIENT.list_channels().await.unwrap_or_default() });

    let mut show_create_modal = use_signal(|| false);

    // Form signals
    let mut form_name = use_signal(|| String::new());
    let mut form_base_url = use_signal(|| String::from("https://api.openai.com"));
    let mut form_api_key = use_signal(|| String::new());
    let mut form_match_path = use_signal(|| String::from("/v1/chat/completions"));
    let mut form_auth_type = use_signal(|| String::from("Bearer"));

    // Mock Traffic Data
    let throughput = "452 req/s";
    let latency = "124 ms";
    let success_rate = "99.98%";

    let open_modal = move |_| {
        form_name.set(String::new());
        form_api_key.set(String::new());
        show_create_modal.set(true);
    };

    let handle_submit = move |_| {
        // Generate a pseudo-random ID using timestamp since we don't have uuid crate directly
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string();

        let new_channel = ChannelDto {
            id,
            name: form_name(),
            base_url: form_base_url(),
            api_key: form_api_key(),
            match_path: form_match_path(),
            auth_type: form_auth_type(),
            priority: 0,
        };

        spawn(async move {
            if API_CLIENT.create_channel(new_channel).await.is_ok() {
                show_create_modal.set(false);
                channels.restart();
            }
        });
    };

    let delete_channel = move |id: String| async move {
        if API_CLIENT.delete_channel(&id).await.is_ok() {
            channels.restart();
        }
    };

    let channels_data = channels.read().clone();

    rsx! {
        div { class: "flex flex-col h-full gap-8",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-2xl font-semibold text-base-content mb-1 tracking-tight", "API 网关" }
                    p { class: "text-sm text-base-content/60 font-medium", "流量控制与路由分发" }
                }
                div { class: "flex gap-3",
                    BCButton {
                        class: "btn-ghost btn-sm text-base-content/70",
                        onclick: move |_| { channels.restart(); },
                        "刷新状态"
                    }
                    BCButton {
                        class: "btn-neutral btn-sm px-6 text-white shadow-sm",
                        onclick: open_modal,
                        "新建路由"
                    }
                }
            }

            // HUD: Traffic Stats
            div { class: "grid grid-cols-3 gap-6",
                // Throughput
                div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-1",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "实时吞吐量 (Throughput)" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-3xl font-bold text-base-content tracking-tight", "{throughput}" }
                        div { class: "w-2 h-2 rounded-full bg-emerald-500 animate-pulse" }
                    }
                }
                // Latency
                div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-1",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "平均延迟 (Avg Latency)" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-3xl font-bold text-base-content tracking-tight", "{latency}" }
                        span { class: "text-xs font-medium text-base-content/40", "P99: 450ms" }
                    }
                }
                // Success Rate
                div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-1",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "成功率 (Success Rate)" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-3xl font-bold text-base-content tracking-tight", "{success_rate}" }
                    }
                }
            }

            // Routing Table
            div { class: "flex flex-col gap-4",
                h3 { class: "text-sm font-medium text-base-content/80 border-b border-base-content/10 pb-2", "上游路由规则 (Upstream Routing)" }

                div { class: "overflow-x-auto border border-base-200 rounded-lg",
                    table { class: "table w-full text-sm",
                        thead { class: "bg-base-50 text-base-content/60",
                            tr {
                                th { class: "font-medium", "路由名称" }
                                th { class: "font-medium", "目标地址 (Target)" }
                                th { class: "font-medium", "负载 (Load)" }
                                th { class: "font-medium", "健康状态" }
                                th { class: "text-right font-medium", "操作" }
                            }
                        }
                        tbody {
                            if let Some(list) = channels_data.as_ref() {
                                if !list.is_empty() {
                                    for channel in list {
                                        {
                                            let channel_id = channel.id.clone();
                                            rsx! {
                                                tr { class: "hover:bg-base-50/50 transition-colors group",
                                                    td {
                                                        div { class: "font-semibold text-base-content", "{channel.name}" }
                                                        div { class: "text-xs text-base-content/40 font-mono", "{channel.match_path}" }
                                                    }
                                                    td { class: "font-mono text-base-content/80 text-xs", "{channel.base_url}" }
                                                    // Mock Load
                                                    td {
                                                        div { class: "flex items-center gap-2",
                                                            progress { class: "progress progress-primary w-16 h-1.5", value: "45", max: "100" }
                                                            span { class: "text-xs text-base-content/60", "45%" }
                                                        }
                                                    }
                                                    td {
                                                        BCBadge {
                                                            variant: BadgeVariant::Success,
                                                            dot: true,
                                                            "12ms"
                                                        }
                                                    }
                                                    td { class: "text-right",
                                                        button {
                                                            class: "btn btn-ghost btn-xs text-base-content/40 group-hover:text-error transition-colors",
                                                            onclick: move |_| {
                                                                let id = channel_id.clone();
                                                                let delete_fn = delete_channel.clone();
                                                                spawn(async move { delete_fn(id).await; });
                                                            },
                                                            "移除"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    tr { td { colspan: "5", class: "p-8 text-center text-base-content/40", "暂无活跃路由" } }
                                }
                            } else {
                                tr { td { colspan: "5", class: "p-8 text-center text-base-content/40", "连接网关中..." } }
                            }
                        }
                    }
                }
            }

            // Create Modal
            BCModal {
                open: show_create_modal(),
                title: "新建路由规则".to_string(),
                onclose: move |_| show_create_modal.set(false),

                div { class: "flex flex-col gap-4 py-2",
                    BCInput {
                        label: Some("路由名称".to_string()),
                        value: "{form_name}",
                        placeholder: "e.g. GPT-4 Primary Route".to_string(),
                        oninput: move |e: FormEvent| form_name.set(e.value())
                    }
                    BCInput {
                        label: Some("目标地址 (Base URL)".to_string()),
                        value: "{form_base_url}",
                        placeholder: "https://api.openai.com".to_string(),
                        oninput: move |e: FormEvent| form_base_url.set(e.value())
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-sm font-medium text-base-content/80", "鉴权类型" }
                        select { class: "select select-bordered w-full select-sm",
                            value: "{form_auth_type}",
                            onchange: move |e: FormEvent| form_auth_type.set(e.value()),
                            option { value: "Bearer", "Bearer Token" }
                            option { value: "XApiKey", "X-Api-Key" }
                            option { value: "GoogleAI", "Google AI" }
                        }
                    }
                    BCInput {
                        label: Some("API Key".to_string()),
                        value: "{form_api_key}",
                        placeholder: "sk-...".to_string(),
                        oninput: move |e: FormEvent| form_api_key.set(e.value())
                    }
                    BCInput {
                        label: Some("匹配路径".to_string()),
                        value: "{form_match_path}",
                        placeholder: "/v1/chat/completions".to_string(),
                        oninput: move |e: FormEvent| form_match_path.set(e.value())
                    }
                }

                div { class: "modal-footer flex justify-end gap-3 mt-6",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| show_create_modal.set(false),
                        "取消"
                    }
                    BCButton {
                        class: "btn-neutral text-white",
                        onclick: handle_submit,
                        "创建路由"
                    }
                }
            }
        }
    }
}
