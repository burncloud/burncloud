use burncloud_client_shared::components::{
    BCButton, BCModal, FormMode, PageHeader, SchemaForm,
    StatKpi, StatusPill, Chip, ColumnDef, PageTable,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::user_service::{User, UserService};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;
use serde_json::json;

fn status_label(status: i32) -> &'static str {
    match status {
        1 => "active",
        0 => "disabled",
        _ => "unknown",
    }
}

fn role_label(role: &str) -> &str {
    match role {
        "admin" => "admin",
        "vip" => "vip",
        _ => "user",
    }
}

fn format_balance(nano: i64) -> String {
    format!("{:.2}", nano as f64 / 1_000_000_000.0)
}

#[component]
pub fn UserPage() -> Element {
    let mut show_form = use_signal(|| false);
    let mut form_mode = use_signal(|| FormMode::Create);
    let mut form_data = use_signal(|| json!({}));
    let mut active_filter = use_signal(|| "all".to_string());
    let toast = use_toast();

    let users = use_resource(move || async move {
        UserService::list().await.unwrap_or_default()
    });

    let user_list = users.read().clone().unwrap_or_default();
    let loading = users.read().is_none();
    let total = user_list.len();
    let active_count = user_list.iter().filter(|u| u.status == 1).count();
    let vip_count = user_list.iter().filter(|u| u.role == "vip").count();

    let columns = vec![
        ColumnDef { key: "id".to_string(), label: "ID".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "name".to_string(), label: "Name".to_string(), width: None },
        ColumnDef { key: "role".to_string(), label: "Role".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "Status".to_string(), width: Some("120px".to_string()) },
        ColumnDef { key: "balance".to_string(), label: "Balance".to_string(), width: Some("120px".to_string()) },
        ColumnDef { key: "actions".to_string(), label: "".to_string(), width: Some("80px".to_string()) },
    ];

    let filtered_users: Vec<&User> = user_list.iter().filter(|u| {
        match active_filter().as_str() {
            "active" => u.status == 1,
            "vip" => u.role == "vip",
            _ => true,
        }
    }).collect();

    rsx! {
        PageHeader {
            title: "客户列表",
            subtitle: Some("用户增长与留存管理".to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| {
                        form_mode.set(FormMode::Create);
                        form_data.set(json!({}));
                        show_form.set(true);
                    },
                    "邀请用户"
                }
            }
        }

        div { class: "page-content",
            // Stats
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "总用户数",
                        value: "{total}",
                    }
                    StatKpi {
                        label: "活跃用户",
                        value: "{active_count}",
                    }
                    StatKpi {
                        label: "VIP 用户",
                        value: "{vip_count}",
                    }
                    StatKpi {
                        label: "今日新增",
                        value: "—",
                    }
                }
            }

            // Filter chips
            div { class: "chip-row", style: "margin: 20px 0;",
                Chip {
                    label: "全部".to_string(),
                    count: Some(total as i64),
                    active: Some(active_filter() == "all"),
                    onclick: move |_| active_filter.set("all".to_string()),
                }
                Chip {
                    label: "活跃".to_string(),
                    count: Some(active_count as i64),
                    active: Some(active_filter() == "active"),
                    onclick: move |_| active_filter.set("active".to_string()),
                }
                Chip {
                    label: "VIP".to_string(),
                    count: Some(vip_count as i64),
                    active: Some(active_filter() == "vip"),
                    onclick: move |_| active_filter.set("vip".to_string()),
                }
            }

            // Table
            if loading {
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
            } else {
                PageTable {
                columns: columns,
                for user in filtered_users {
                    tr {
                        key: "{user.id}",
                        td { class: "mono", "{user.id}" }
                        td { "{user.username}" }
                        td { "{role_label(&user.role)}" }
                        td {
                            StatusPill {
                                value: status_label(user.status).to_string()
                            }
                        }
                        td { class: "mono", style: "font-size:13px",
                            "¥{format_balance(user.balance_cny)}"
                        }
                        td {
                            button {
                                class: "btn-ghost",
                                onclick: {
                                    let user = user.clone();
                                    move |_| {
                                        form_data.set(json!({
                                            "id": user.id,
                                            "username": user.username,
                                            "role": user.role,
                                        }));
                                        form_mode.set(FormMode::Edit);
                                        show_form.set(true);
                                    }
                                },
                                "编辑"
                            }
                        }
                    }
                }
            }
            }
        }

        // Modal
        if show_form() {
            BCModal {
                open: show_form(),
                onclose: move |_| show_form.set(false),
                SchemaForm {
                    schema: json!({
                        "fields": [
                            {"key": "username", "label": "Name", "type": "text", "required": true},
                            {"key": "role", "label": "Role", "type": "select", "options": ["user", "vip", "admin"]},
                        ]
                    }),
                    data: form_data,
                    mode: *form_mode.read(),
                    on_submit: move |_data| {
                        toast.success("保存成功");
                        show_form.set(false);
                    },
                }
            }
        }
    }
}
