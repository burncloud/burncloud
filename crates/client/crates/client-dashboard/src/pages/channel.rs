use burncloud_client_shared::channel_service::{Channel, ChannelService};
use burncloud_client_shared::components::{
    BCBadge, BCButton, BCCard, BCInput, BCModal, BCPagination, BCTable, BadgeVariant, ButtonVariant,
};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn ChannelPage() -> Element {
    let mut page = use_signal(|| 1);
    let limit = 10; // Default items per page

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
        is_modal_open.set(true);
    };

    let handle_save = move |_| {
        spawn(async move {
            is_loading.set(true);

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

    // Clone data to avoid lifetime issues in RSX
    let channels_data = channels.read().clone();

    // Mock total pages for now since API might not return count yet
    // In real scenario, list() should return (Vec<Channel>, total_count)
    let total_pages = 5; // Placeholder

    rsx! {
        div { class: "channel-page-wrapper",
            div { class: "page-header",
                div { class: "flex justify-between items-center",
                    div {
                        h1 { class: "text-large-title font-bold text-primary m-0", "渠道管理" }
                        p { class: "text-secondary m-0 mt-sm", "管理上游模型供应商与API Key" }
                    }
                    BCButton {
                        class: "btn-create-channel",
                        onclick: open_create_modal,
                        "新建渠道"
                    }
                }
            }

            div { class: "page-content mt-lg",
                BCCard {
                    class: "p-0 overflow-hidden",
                    BCTable {
                        pagination: rsx! {
                            BCPagination {
                                page: page(),
                                total_pages: total_pages,
                                on_change: move |p| page.set(p)
                            }
                        },
                        thead {
                            tr {
                                th { "ID" }
                                th { "名称" }
                                th { "类型" }
                                th { "模型" }
                                th { "状态" }
                                th { class: "text-right", "操作" }
                            }
                        }
                        tbody {
                            match channels_data {
                                Some(list) if !list.is_empty() => rsx! {
                                    for channel in list {
                                        tr { class: "hover:bg-subtle transition-colors channel-row",
                                            td { class: "text-secondary", "{channel.id}" }
                                            td { class: "font-medium channel-name", "{channel.name}" }
                                            td {
                                                BCBadge {
                                                    variant: BadgeVariant::Info,
                                                    match channel.type_ {
                                                        1 => "OpenAI",
                                                        14 => "Anthropic",
                                                        24 => "Google Gemini",
                                                        _ => "Unknown"
                                                    }
                                                }
                                            }
                                            td { class: "text-secondary text-caption truncate", style: "max-width: 200px;", "{channel.models}" }
                                            td {
                                                if channel.status == 1 {
                                                    BCBadge { variant: BadgeVariant::Success, dot: true, "启用" }
                                                } else {
                                                    BCBadge { variant: BadgeVariant::Neutral, dot: true, "禁用" }
                                                }
                                            }
                                            td { class: "text-right",
                                                BCButton {
                                                    variant: ButtonVariant::Ghost,
                                                    class: "text-error btn-delete-channel",
                                                    onclick: move |_| handle_delete(channel.id),
                                                    "🗑️"
                                                }
                                            }
                                        }
                                    }
                                },
                                Some(_) => rsx! { tr { td { colspan: "6", class: "p-xl text-center text-secondary", "暂无渠道，请点击右上角创建" } } },
                                None => rsx! { tr { td { colspan: "6", class: "p-xl text-center text-secondary", "加载中..." } } }
                            }
                        }
                    }
                }
            }

            BCModal {
                open: is_modal_open(),
                title: "渠道配置".to_string(),
                onclose: move |_| is_modal_open.set(false),

                div { class: "vstack gap-3",
                    BCInput {
                        label: Some("渠道名称".to_string()),
                        value: "{form_name}",
                        placeholder: "例如: OpenAI 主账号".to_string(),
                        oninput: move |e: FormEvent| form_name.set(e.value())
                    }

                    div { class: "form-group",
                        label { class: "form-label fw-bold mb-1", "渠道类型" }
                        select { class: "form-control", // Changed from form-select to form-control for consistency
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
                        label: Some("Base URL".to_string()),
                        value: "{form_base_url}",
                        placeholder: "https://api.openai.com".to_string(),
                        oninput: move |e: FormEvent| form_base_url.set(e.value())
                    }

                    BCInput {
                        label: Some("支持模型".to_string()),
                        value: "{form_models}",
                        placeholder: "gpt-3.5-turbo,gpt-4".to_string(),
                        oninput: move |e: FormEvent| form_models.set(e.value())
                    }
                }

                div { class: "modal-footer",
                    BCButton {
                        variant: ButtonVariant::Secondary,
                        class: "me-2",
                        onclick: move |_| is_modal_open.set(false),
                        "取消"
                    }
                    BCButton {
                        class: "btn-save-channel",
                        loading: is_loading(),
                        onclick: handle_save,
                        "保存"
                    }
                }
            }
        }
    }
}
