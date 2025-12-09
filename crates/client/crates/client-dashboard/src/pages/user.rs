use burncloud_client_shared::components::{BCButton, BCCard, BCInput};
use burncloud_client_shared::use_toast;
use burncloud_client_shared::user_service::UserService;
use dioxus::prelude::*;

#[component]
pub fn UserPage() -> Element {
    let mut users =
        use_resource(move || async move { UserService::list().await.unwrap_or(vec![]) });

    let mut is_topup_open = use_signal(|| false);
    let mut selected_user_id = use_signal(|| String::new());
    let mut topup_amount = use_signal(|| 0.0);
    let mut is_loading = use_signal(|| false);
    let toast = use_toast();

    // Fix: make closure mutable to satisfy EventHandler requirements if needed,
    // though for simple signal updates it might not be strictly required by logic, the compiler complains.
    let mut handle_topup_click = move |id: String| {
        selected_user_id.set(id);
        topup_amount.set(0.0);
        is_topup_open.set(true);
    };

    let handle_confirm_topup = move |_| {
        is_loading.set(true);
        spawn(async move {
            match UserService::topup(&selected_user_id(), topup_amount()).await {
                Ok(new_balance) => {
                    toast.success(&format!("充值成功，当前余额: ¥ {:.2}", new_balance));
                    is_topup_open.set(false);
                    users.restart();
                }
                Err(e) => toast.error(&format!("充值失败: {}", e)),
            }
            is_loading.set(false);
        });
    };

    // Fix: Clone data to avoid holding Ref across match arms or lifetime issues
    let users_data = users.read().clone();

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0", "用户管理" }
                    p { class: "text-secondary m-0 mt-sm", "查看用户列表、余额与权限" }
                }
                BCButton {
                    "邀请用户"
                }
            }
        }

        div { class: "page-content mt-lg",
            BCCard {
                class: "p-0 overflow-hidden",
                table { class: "w-full border-collapse",
                    thead {
                        tr {
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "ID" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "data-testid": "th-username", "用户名" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "角色" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "分组" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "data-testid": "th-balance", "余额" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "状态" }
                            th { class: "text-right p-md border-b border-subtle text-secondary font-medium", "操作" }
                        }
                    }
                    tbody {
                        match users_data {
                            Some(list) if !list.is_empty() => rsx! {
                                for user in list {
                                    tr { class: "hover:bg-subtle transition-colors", "data-testid": "user-row",
                                        td { class: "p-md text-secondary border-b border-subtle", "{user.id}" }
                                        td { class: "p-md font-medium border-b border-subtle",
                                            div { class: "flex items-center gap-sm", "data-testid": "user-username",
                                                div { class: "w-8 h-8 rounded-full bg-primary-light text-primary flex items-center justify-center font-bold",
                                                    "{user.username.chars().next().unwrap_or('?')}"
                                                }
                                                "{user.username}"
                                            }
                                        }
                                        td { class: "p-md border-b border-subtle",
                                            span { class: "px-sm py-xs rounded bg-surface-variant text-caption", "{user.role}" }
                                        }
                                        td { class: "p-md text-secondary border-b border-subtle", "{user.group}" }
                                        td { class: "p-md font-medium text-primary border-b border-subtle", "¥ {user.balance:.2}" }
                                        td { class: "p-md border-b border-subtle",
                                            if user.status == 1 {
                                                span { class: "inline-flex items-center gap-xs text-success", span{class:"w-2 h-2 rounded-full bg-success"}, "正常" }
                                            } else {
                                                span { class: "inline-flex items-center gap-xs text-error", span{class:"w-2 h-2 rounded-full bg-error"}, "禁用" }
                                            }
                                        }
                                        td { class: "p-md text-right border-b border-subtle",
                                            BCButton {
                                                variant: burncloud_client_shared::components::ButtonVariant::Ghost,
                                                onclick: move |_| handle_topup_click(user.id.clone()),
                                                "充值"
                                            }
                                            BCButton { variant: burncloud_client_shared::components::ButtonVariant::Ghost, class: "text-secondary", "✏️" }
                                        }
                                    }
                                }
                            },
                            Some(_) => rsx! { tr { td { colspan: "7", class: "p-xl text-center text-secondary", "暂无用户" } } },
                            None => rsx! { tr { td { colspan: "7", class: "p-xl text-center text-secondary", "加载中..." } } }
                        }
                    }
                }
            }

            // Topup Modal (Inline)
            div {
                class: "modal-overlay",
                style: if is_topup_open() { "display: flex" } else { "display: none" },
                onclick: move |_| is_topup_open.set(false),

                div {
                    class: "modal-content",
                    onclick: |e| e.stop_propagation(),

                    div { class: "modal-header",
                        h3 { class: "modal-title-text text-title font-bold m-0", "用户充值" }
                    }
                    div { class: "modal-body",
                        div { class: "vstack gap-3",
                            div { class: "text-secondary mb-2", "用户 ID: {selected_user_id}" }

                            BCInput {
                                label: Some("充值金额 (¥)".to_string()),
                                value: "{topup_amount}",
                                placeholder: "请输入金额".to_string(),
                                oninput: move |e: FormEvent| topup_amount.set(e.value().parse().unwrap_or(0.0))
                            }
                        }
                    }
                    div { class: "modal-footer",
                        BCButton {
                            variant: burncloud_client_shared::components::ButtonVariant::Secondary,
                            class: "me-2",
                            onclick: move |_| is_topup_open.set(false),
                            "取消"
                        }
                        BCButton {
                            loading: is_loading(),
                            onclick: handle_confirm_topup,
                            "确认充值"
                        }
                    }
                }
            }
        }
    }
}
