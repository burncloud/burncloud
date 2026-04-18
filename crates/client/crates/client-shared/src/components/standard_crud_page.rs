// Generic CRUD page driven by JSON schema — Value required for schema, form data, and table rows.
#![allow(clippy::disallowed_types)]

use crate::components::{
    ActionDef, ActionEvent, BCButton, ButtonVariant, FormMode, SchemaForm, SchemaTable,
};
use crate::use_toast;
use dioxus::prelude::*;
use serde_json::{json, Value};

/// 通用的 CRUD 页面容器
///
/// 功能：
/// 1. 渲染 SchemaTable 显示列表
/// 2. 渲染 SchemaForm 弹窗进行新增/编辑
/// 3. 自动处理 Loading 和 API 错误提示
/// 4. 封装标准的 CRUD 操作流
#[component]
pub fn StandardCrudPage(schema: Value, api_endpoint: String) -> Element {
    let mut show_form = use_signal(|| false);
    let mut form_mode = use_signal(|| FormMode::Create);
    let mut form_data = use_signal(|| json!({}));
    let items = use_signal(Vec::<Value>::new);
    let mut loading = use_signal(|| true);
    let toast = use_toast();

    // 1. 获取列表数据
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            // 这里以后可以抽象成标准的 ApiClient 调用
            // 暂时用占位逻辑，实际开发中会对接后端
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            loading.set(false);
        });
    });

    // 2. 表格操作定义
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

    // 3. 处理表格事件
    let on_action = move |evt: ActionEvent| {
        match evt.action_id.as_str() {
            "edit" => {
                form_data.set(evt.row.clone());
                form_mode.set(FormMode::Edit);
                show_form.set(true);
            }
            "delete" => {
                // TODO: 确认弹窗 & 接口调用
                toast.success("删除成功 (模拟)");
            }
            _ => {}
        }
    };

    rsx! {
        div { class: "flex flex-col gap-6 p-6 animate-fade-in",
            // Header: Title & New Button
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-2xl font-bold tracking-tight", "{schema[\"label\"].as_str().unwrap_or(\"Module\")}" }
                    p { class: "text-muted-foreground", "Manage and monitor your {schema[\"label\"].as_str().unwrap_or(\"entities\")}." }
                }
                BCButton {
                    variant: ButtonVariant::Primary,
                    onclick: move |_| {
                        form_mode.set(FormMode::Create);
                        form_data.set(json!({}));
                        show_form.set(true);
                    },
                    "新增 {schema[\"label\"].as_str().unwrap_or(\"项目\")}"
                }
            }

            // Body: The Table
            div { class: "bg-card border rounded-xl overflow-hidden shadow-sm",
                SchemaTable {
                    schema: schema.clone(),
                    data: items.read().clone(),
                    loading: *loading.read(),
                    actions: actions,
                    on_action: on_action
                }
            }

            // Modal: The Form
            if *show_form.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4",
                    div { class: "bg-canvas w-full max-w-2xl rounded-2xl shadow-2xl border animate-scale-in max-h-[90vh] overflow-y-auto",
                        div { class: "p-6 border-b flex justify-between items-center sticky top-0 bg-canvas z-10",
                            h2 { class: "text-xl font-semibold",
                                match *form_mode.read() {
                                    FormMode::Create => "新增 {schema[\"label\"]}",
                                    FormMode::Edit => "编辑 {schema[\"label\"]}",
                                    FormMode::View => "查看 {schema[\"label\"]}",
                                }
                            }
                            button {
                                class: "p-2 hover:bg-black/5 rounded-full transition-colors",
                                onclick: move |_| show_form.set(false),
                                "✕"
                            }
                        }
                        div { class: "p-6",
                            SchemaForm {
                                schema: schema.clone(),
                                data: form_data,
                                mode: *form_mode.read(),
                                on_submit: move |_data| {
                                    // TODO: API 调用
                                    toast.success("提交成功 (模拟)");
                                    show_form.set(false);
                                    spawn(async move {
                                        loading.set(true);
                                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                        loading.set(false);
                                    });
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
