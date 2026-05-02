// Generic CRUD page driven by JSON schema — Value required for schema, form data, and table rows.
#![allow(clippy::disallowed_types)]

use crate::api_client::API_CLIENT;
use crate::components::{
    ActionDef, ActionEvent, BCButton, BCModal, ButtonVariant, EmptyState, FormMode, SchemaForm,
    SchemaTable,
};
use crate::use_toast;
use dioxus::prelude::*;
#[allow(clippy::disallowed_types)]
use serde_json::{json, Value};

/// 通用的 CRUD 页面容器
///
/// 功能：
/// 1. 渲染 SchemaTable 显示列表
/// 2. 渲染 SchemaForm 弹窗进行新增/编辑
/// 3. 删除确认弹窗（复用 BCModal）
/// 4. 真实 CRUD API 调用（RESTful URL 约定）
/// 5. 自动处理 Loading / Error / Empty 状态
#[component]
pub fn StandardCrudPage(
    schema: Value,
    api_endpoint: String,
    #[props(default = "id".to_string())] id_field: String,
) -> Element {
    let mut show_form = use_signal(|| false);
    let mut form_mode = use_signal(|| FormMode::Create);
    let mut form_data = use_signal(|| json!({}));
    let mut items = use_signal(Vec::<Value>::new);
    let mut loading = use_signal(|| true);
    let mut saving = use_signal(|| false);
    let toast = use_toast();

    // Delete confirmation state
    let mut show_delete_confirm = use_signal(|| false);
    let mut delete_target_id = use_signal(String::new);
    let mut delete_target_name = use_signal(String::new);

    let entity_label = schema["label"].as_str().unwrap_or("项目").to_string();

    // 1. Fetch list data from API
    let endpoint_for_fetch = api_endpoint.clone();
    use_effect(move || {
        let endpoint = endpoint_for_fetch.clone();
        spawn(async move {
            loading.set(true);
            match API_CLIENT.crud_list(&endpoint).await {
                Ok(data) => items.set(data),
                Err(e) => toast.error(&format!("加载失败: {}", e)),
            }
            loading.set(false);
        });
    });

    // 2. Table action definitions
    let actions = vec![
        ActionDef {
            action_id: "edit".to_string(),
            label: "编辑".to_string(),
            color: String::new(),
        },
        ActionDef {
            action_id: "delete".to_string(),
            label: "删除".to_string(),
            color: "danger".to_string(),
        },
    ];

    // 3. Handle table events
    let on_action = {
        let id_field = id_field.clone();
        move |evt: ActionEvent| {
            match evt.action_id.as_str() {
                "edit" => {
                    form_data.set(evt.row.clone());
                    form_mode.set(FormMode::Edit);
                    show_form.set(true);
                }
                "delete" => {
                    let id = evt.row[&id_field]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    let name = evt.row["name"]
                        .as_str()
                        .or_else(|| evt.row[&id_field].as_str())
                        .unwrap_or_default()
                        .to_string();
                    delete_target_id.set(id);
                    delete_target_name.set(name);
                    show_delete_confirm.set(true);
                }
                _ => {}
            }
        }
    };

    // 4. Confirm delete handler
    let confirm_delete = {
        let endpoint = api_endpoint.clone();
        let label = entity_label.clone();
        move |_| {
            let id = delete_target_id.read().clone();
            let name = delete_target_name.read().clone();
            let endpoint = endpoint.clone();
            let label = label.clone();
            spawn(async move {
                saving.set(true);
                match API_CLIENT.crud_delete(&endpoint, &id).await {
                    Ok(()) => {
                        toast.success(&format!("{} \"{}\" 已删除", label, name));
                        show_delete_confirm.set(false);
                        // Refresh list
                        match API_CLIENT.crud_list(&endpoint).await {
                            Ok(data) => items.set(data),
                            Err(e) => toast.error(&format!("刷新列表失败: {}", e)),
                        }
                    }
                    Err(e) => {
                        toast.error(&format!("删除失败: {}", e));
                    }
                }
                saving.set(false);
            });
        }
    };

    // 5. Form submit handler (create or update)
    let on_submit = {
        let endpoint = api_endpoint.clone();
        let label = entity_label.clone();
        move |data: Value| {
            // Guard against double-click
            if *saving.read() {
                return;
            }
            let endpoint = endpoint.clone();
            let label = label.clone();
            let id_field = id_field.clone();
            let is_edit = *form_mode.read() == FormMode::Edit;
            spawn(async move {
                saving.set(true);
                let result = if is_edit {
                    let id = data[&id_field]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    API_CLIENT.crud_update(&endpoint, &id, &data).await
                } else {
                    API_CLIENT.crud_create(&endpoint, &data).await
                };
                match result {
                    Ok(()) => {
                        toast.success(&format!("{}已保存", label));
                        show_form.set(false);
                        // Refresh list
                        match API_CLIENT.crud_list(&endpoint).await {
                            Ok(data) => items.set(data),
                            Err(e) => toast.error(&format!("刷新列表失败: {}", e)),
                        }
                    }
                    Err(e) => {
                        toast.error(&format!("保存失败: {}", e));
                    }
                }
                saving.set(false);
            });
        }
    };

    rsx! {
        div { class: "flex flex-col gap-6 p-6 animate-fade-in",
            // Header: Title & New Button
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-2xl font-bold tracking-tight", "{entity_label}" }
                    p { class: "text-muted-foreground", "Manage and monitor your {entity_label}." }
                }
                BCButton {
                    variant: ButtonVariant::Primary,
                    onclick: move |_| {
                        form_mode.set(FormMode::Create);
                        form_data.set(json!({}));
                        show_form.set(true);
                    },
                    "新增 {entity_label}"
                }
            }

            // Body: The Table or Empty State
            div { class: "bg-card border rounded-xl overflow-hidden shadow-sm",
                if !*loading.read() && items.read().is_empty() {
                    EmptyState {
                        icon: rsx! { span { style: "font-size:32px", "📭" } },
                        title: format!("暂无{}", entity_label),
                        description: Some(format!("点击上方按钮创建第一个{}", entity_label)),
                        cta: None,
                    }
                } else {
                    SchemaTable {
                        schema: schema.clone(),
                        data: items.read().clone(),
                        loading: *loading.read(),
                        actions: actions,
                        on_action: on_action
                    }
                }
            }

            // Modal: The Form (using BCModal)
            BCModal {
                open: *show_form.read(),
                title: match *form_mode.read() {
                    FormMode::Create => format!("新增 {}", entity_label),
                    FormMode::Edit => format!("编辑 {}", entity_label),
                    FormMode::View => format!("查看 {}", entity_label),
                },
                onclose: move |_| show_form.set(false),

                div { role: "document",
                    SchemaForm {
                        schema: schema.clone(),
                        data: form_data,
                        mode: *form_mode.read(),
                        on_submit: on_submit,
                    }
                }
            }

            // Delete Confirmation Modal (using BCModal)
            BCModal {
                open: *show_delete_confirm.read(),
                title: "确认删除",
                onclose: move |_| show_delete_confirm.set(false),

                div { role: "dialog", aria_modal: "true",
                    p { class: "mb-lg",
                        "确定要删除{entity_label} "
                        span { class: "font-bold", "{delete_target_name}" }
                        " 吗？此操作不可撤销。"
                    }
                    div { class: "flex justify-end gap-sm",
                        BCButton {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| show_delete_confirm.set(false),
                            "取消"
                        }
                        BCButton {
                            variant: ButtonVariant::Danger,
                            disabled: *saving.read(),
                            onclick: confirm_delete,
                            if *saving.read() { "删除中..." } else { "确认删除" }
                        }
                    }
                }
            }
        }
    }
}
