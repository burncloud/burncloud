use burncloud_client_shared::channel_service::{Channel, ChannelService};
use burncloud_client_shared::components::{BCButton, BCInput, BCModal, ButtonVariant};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn ChannelPage() -> Element {
    let mut page = use_signal(|| 1);
    let limit = 10;

    let mut channels =
        use_resource(
            move || async move { ChannelService::list(page(), limit).await.unwrap_or(vec![]) },
        );

    let mut is_modal_open = use_signal(|| false);
    let mut form_id = use_signal(|| 0i64);
    let mut form_name = use_signal(|| String::new());
    let mut form_type = use_signal(|| 1);
    let mut form_key = use_signal(|| String::new());
    let mut form_base_url = use_signal(|| String::new());
    let mut form_models = use_signal(|| String::new());
    let mut form_group = use_signal(|| "default".to_string());
    let mut form_param_override = use_signal(|| String::new());
    let mut form_header_override = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);
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
                param_override: if p_override.is_empty() { None } else { Some(p_override) },
                header_override: if h_override.is_empty() { None } else { Some(h_override) },
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

    let handle_delete = move |id: i64| {
        spawn(async move {
            if ChannelService::delete(id).await.is_ok() {
                channels.restart();
                toast.success("渠道已删除");
            } else {
                toast.error("删除失败");
            }
        });
    };

    let channels_data = channels.read().clone();

    rsx! {
        div { class: "flex flex-col h-full gap-8",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-2xl font-semibold text-base-content mb-1 tracking-tight", "模型网络" }
                    p { class: "text-sm text-base-content/60 font-medium", "配置与管理您的 AI 算力来源" }
                }
                BCButton {
                    class: "btn-neutral btn-sm px-6 shadow-sm text-white",
                    onclick: open_create_modal,
                    "添加连接"
                }
            }

            // Active Connections List
            div { class: "flex flex-col gap-4",
                h3 { class: "text-sm font-medium text-base-content/80 border-b border-base-content/10 pb-2", "活跃连接" }

                div { class: "overflow-x-auto border border-base-200 rounded-lg",
                    table { class: "table w-full text-sm",
                        thead { class: "bg-base-50 text-base-content/60",
                            tr {
                                th { class: "font-medium", "名称" }
                                th { class: "font-medium", "协议" }
                                th { class: "font-medium", "模型" }
                                th { class: "font-medium", "优先级" }
                                th { class: "font-medium", "状态" }
                                th { class: "text-right font-medium", "操作" }
                            }
                        }
                        tbody {
                            match channels_data {
                                Some(list) if !list.is_empty() => rsx! {
                                    for channel in list {
                                        tr { class: "hover:bg-base-50/50 transition-colors group",
                                            td {
                                                div { class: "font-semibold text-base-content", "{channel.name}" }
                                                div { class: "text-xs text-base-content/40", "ID: {channel.id}" }
                                            }
                                            td {
                                                match channel.type_ {
                                                    1 => "OpenAI",
                                                    14 => "Anthropic",
                                                    24 => "Google Gemini",
                                                    _ => "Custom"
                                                }
                                            }
                                            td { class: "font-mono text-xs text-base-content/70 max-w-xs truncate", title: "{channel.models}", "{channel.models}" }
                                            td { class: "font-mono font-medium", "{channel.priority}" }
                                            td {
                                                if channel.status == 1 {
                                                    span { class: "inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium bg-emerald-50 text-emerald-700",
                                                        span { class: "w-1.5 h-1.5 rounded-full bg-emerald-500" }
                                                        "正常"
                                                    }
                                                } else {
                                                    span { class: "inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium bg-base-200 text-base-content/60",
                                                        span { class: "w-1.5 h-1.5 rounded-full bg-base-400" }
                                                        "禁用"
                                                    }
                                                }
                                            }
                                            td { class: "text-right",
                                                button {
                                                    class: "btn btn-ghost btn-xs text-base-content/40 group-hover:text-error transition-colors",
                                                    onclick: move |_| handle_delete(channel.id),
                                                    "删除"
                                                }
                                            }
                                        }
                                    }
                                },
                                Some(_) => rsx! { tr { td { colspan: "6", class: "p-8 text-center text-base-content/40", "暂无连接数据" } } },
                                None => rsx! { tr { td { colspan: "6", class: "p-8 text-center text-base-content/40", "加载中..." } } }
                            }
                        }
                    }
                }
            }

            // Modal
            BCModal {
                open: is_modal_open(),
                title: "添加供应商渠道".to_string(),
                onclose: move |_| is_modal_open.set(false),

                div { class: "flex flex-col gap-4 py-2",
                    BCInput {
                        label: Some("渠道名称".to_string()),
                        value: "{form_name}",
                        placeholder: "例如: OpenAI 生产环境".to_string(),
                        oninput: move |e: FormEvent| form_name.set(e.value())
                    }

                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-sm font-medium text-base-content/80", "供应商类型" }
                        select { class: "select select-bordered w-full select-sm",
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

                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-sm font-medium text-base-content/80", "参数覆写 (JSON)" }
                        textarea { 
                            class: "textarea textarea-bordered h-24 font-mono text-xs",
                            value: "{form_param_override}",
                            placeholder: "{{ \"temperature\": 0.5 }}",
                            oninput: move |e: FormEvent| form_param_override.set(e.value())
                        }
                    }

                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-sm font-medium text-base-content/80", "Header 覆写 (JSON)" }
                        textarea { 
                            class: "textarea textarea-bordered h-24 font-mono text-xs",
                            value: "{form_header_override}",
                            placeholder: "{{ \"X-Custom-Header\": \"value\" }}",
                            oninput: move |e: FormEvent| form_header_override.set(e.value())
                        }
                    }
                }

                div { class: "modal-footer flex justify-end gap-3 mt-6",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| is_modal_open.set(false),
                        "取消"
                    }
                    BCButton {
                        class: "btn-neutral text-white",
                        loading: is_loading(),
                        onclick: handle_save,
                        "保存配置"
                    }
                }
            }
        }
    }
}
