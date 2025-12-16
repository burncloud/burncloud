use burncloud_client_shared::channel_service::{Channel, ChannelService};
use burncloud_client_shared::components::{BCButton, BCInput, ButtonVariant};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn ChannelPage() -> Element {
    let page = use_signal(|| 1);
    let limit = 10;

    let mut channels =
        use_resource(
            move || async move { ChannelService::list(page(), limit).await.unwrap_or(vec![]) },
        );

    let mut is_modal_open = use_signal(|| false);
    let mut is_delete_modal_open = use_signal(|| false);
    let mut delete_channel_id = use_signal(|| 0i64);
    let mut delete_channel_name = use_signal(String::new);
    let mut form_id = use_signal(|| 0i64);
    let mut form_name = use_signal(String::new);
    let mut form_type = use_signal(|| 1);
    let mut form_key = use_signal(String::new);
    let mut form_base_url = use_signal(String::new);
    let mut form_models = use_signal(String::new);
    let mut form_group = use_signal(|| "default".to_string());
    let mut form_param_override = use_signal(String::new);
    let mut form_header_override = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut show_advanced = use_signal(|| false);
    let toast = use_toast();

    let open_create_modal = move |_| {
        form_id.set(0);
        form_name.set(String::new());
        form_type.set(1);
        form_key.set(String::new());
        form_base_url.set("https://api.openai.com".to_string());
        form_models.set("gpt-3.5-turbo,gpt-4".to_string());
        form_group.set("default".to_string());
        form_param_override.set(String::new());
        form_header_override.set(String::new());
        is_modal_open.set(true);
    };

    let handle_save = move |_| {
        spawn(async move {
            is_loading.set(true);

            let p_override = form_param_override();
            let h_override = form_header_override();

            let ch = Channel {
                id: form_id(),
                type_: form_type(),
                name: form_name(),
                key: form_key(),
                base_url: form_base_url(),
                models: form_models(),
                group: Some(form_group()),
                status: 1,
                priority: 0,
                weight: 0,
                param_override: if p_override.is_empty() {
                    None
                } else {
                    Some(p_override)
                },
                header_override: if h_override.is_empty() {
                    None
                } else {
                    Some(h_override)
                },
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
            let id = delete_channel_id();
            if ChannelService::delete(id).await.is_ok() {
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
                                                1 => "OpenAI",
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

                    // Modal Content - Widened for better data presentation
                    div {
                        class: "fixed inset-0 sm:relative w-full h-full sm:h-auto sm:max-h-[90vh] sm:max-w-2xl bg-base-100 sm:rounded-2xl shadow-2xl border border-base-200 flex flex-col overflow-hidden",
                        onclick: |e| e.stop_propagation(), // Prevent click through

                        // Header
                        div { class: "flex justify-between items-center px-6 py-4 border-b border-base-200 shrink-0 bg-base-100",
                            h3 { class: "text-lg font-bold text-base-content tracking-tight", "添加供应商渠道" }
                            button {
                                class: "btn btn-sm btn-circle btn-ghost text-base-content/50 hover:bg-base-200",
                                onclick: move |_| is_modal_open.set(false),
                                "✕"
                            }
                        }

                        // Form Body - Improved vertical spacing using 16px/24px grid
                        div { class: "flex-1 overflow-y-auto px-6 py-6 flex flex-col gap-6 min-h-0",
                            BCInput {
                                label: Some("渠道名称".to_string()),
                                value: "{form_name}",
                                placeholder: "例如: OpenAI 生产环境".to_string(),
                                oninput: move |e: FormEvent| form_name.set(e.value())
                            }

                            // Unified select styling to match BCInput
                            div { class: "flex flex-col gap-2",
                                label { class: "text-sm font-medium text-base-content/70", "供应商类型" }
                                select { 
                                    class: "w-full h-12 px-4 rounded-xl text-[15px] bg-white/5 hover:bg-white/8 focus:bg-white/80 text-base-content border border-transparent focus:border-accent focus:ring-4 focus:ring-accent/15 focus:outline-none transition-all duration-200 ease-out focus:scale-[1.02]",
                                    onchange: move |e: FormEvent| form_type.set(e.value().parse().unwrap_or(1)),
                                    option { value: "1", "OpenAI" }
                                    option { value: "14", "Anthropic Claude" }
                                    option { value: "24", "Google Gemini" }
                                }
                            }

                            BCInput {
                                label: Some("API Key".to_string()),
                                value: "{form_key}",
                                placeholder: "sk-xxxxxxxx".to_string(),
                                oninput: move |e: FormEvent| form_key.set(e.value())
                            }

                            BCInput {
                                label: Some("代理地址 (Base URL)".to_string()),
                                value: "{form_base_url}",
                                placeholder: "https://api.openai.com".to_string(),
                                oninput: move |e: FormEvent| form_base_url.set(e.value())
                            }

                            BCInput {
                                label: Some("可用模型".to_string()),
                                value: "{form_models}",
                                placeholder: "gpt-4,gpt-3.5-turbo".to_string(),
                                oninput: move |e: FormEvent| form_models.set(e.value())
                            }

                            // Advanced Settings - Collapsible section
                            div { class: "flex flex-col gap-4 mt-2",
                                // Collapsible header
                                button {
                                    class: "flex items-center gap-2 text-sm font-medium text-base-content/70 hover:text-base-content transition-colors",
                                    r#type: "button",
                                    onclick: move |_| show_advanced.set(!show_advanced()),
                                    // Chevron icon
                                    svg {
                                        class: if show_advanced() { "w-4 h-4 transform rotate-90 transition-transform" } else { "w-4 h-4 transition-transform" },
                                        fill: "none",
                                        view_box: "0 0 24 24",
                                        stroke: "currentColor",
                                        stroke_width: "2",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 5l7 7-7 7" }
                                    }
                                    span { "高级设置" }
                                }

                                // Advanced fields - shown only when expanded
                                if show_advanced() {
                                    div { class: "flex flex-col gap-6 pl-6 border-l-2 border-base-200",
                                        div { class: "flex flex-col gap-2",
                                            label { class: "text-sm font-medium text-base-content/70", "参数覆写 (JSON)" }
                                            textarea {
                                                class: "w-full h-24 px-4 py-3 rounded-xl text-xs font-mono leading-relaxed bg-white/5 hover:bg-white/8 focus:bg-white/80 text-base-content border border-transparent focus:border-accent focus:ring-4 focus:ring-accent/15 focus:outline-none transition-all duration-200 ease-out resize-none",
                                                value: "{form_param_override}",
                                                placeholder: "{{ \"temperature\": 0.5 }}",
                                                oninput: move |e: FormEvent| form_param_override.set(e.value())
                                            }
                                        }

                                        div { class: "flex flex-col gap-2",
                                            label { class: "text-sm font-medium text-base-content/70", "Header 覆写 (JSON)" }
                                            textarea {
                                                class: "w-full h-24 px-4 py-3 rounded-xl text-xs font-mono leading-relaxed bg-white/5 hover:bg-white/8 focus:bg-white/80 text-base-content border border-transparent focus:border-accent focus:ring-4 focus:ring-accent/15 focus:outline-none transition-all duration-200 ease-out resize-none",
                                                value: "{form_header_override}",
                                                placeholder: "{{ \"X-Custom-Header\": \"value\" }}",
                                                oninput: move |e: FormEvent| form_header_override.set(e.value())
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Footer
                        div { class: "flex justify-end gap-3 px-6 py-4 border-t border-base-200 bg-base-50/50 shrink-0",
                            BCButton {
                                variant: ButtonVariant::Ghost,
                                onclick: move |_| is_modal_open.set(false),
                                "取消"
                            }
                            BCButton {
                                class: "btn-neutral text-white shadow-md",
                                loading: is_loading(),
                                onclick: handle_save,
                                "保存配置"
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
