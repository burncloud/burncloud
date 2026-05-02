// Generic CRUD page driven by JSON schema — Value required for schema, form data, and table rows.
#![allow(clippy::disallowed_types)]

use crate::api_client::API_CLIENT;
use crate::components::{
    ActionDef, ActionEvent, BCButton, BCModal, ButtonVariant, EmptyState, FormMode, SchemaForm,
    SchemaTable,
};
use crate::i18n::{t, t_fmt, use_i18n};
use crate::use_toast;
use dioxus::prelude::*;
#[allow(clippy::disallowed_types)]
use serde_json::{json, Value};

/// Generic CRUD page container
///
/// Features:
/// 1. Renders SchemaTable for list display
/// 2. Renders SchemaForm modal for create/edit
/// 3. Delete confirmation modal (reuses BCModal)
/// 4. Real CRUD API calls (RESTful URL convention)
/// 5. Auto handles Loading / Error / Empty states
#[component]
pub fn StandardCrudPage(
    schema: Value,
    api_endpoint: String,
    #[props(default = "id".to_string())] id_field: String,
) -> Element {
    let i18n = use_i18n();
    let lang = i18n.language;
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

    let entity_label = schema["label"].as_str().unwrap_or(t(*lang.read(), "crud.default_label")).to_string();

    // 1. Fetch list data from API
    let endpoint_for_fetch = api_endpoint.clone();
    use_effect(move || {
        let endpoint = endpoint_for_fetch.clone();
        spawn(async move {
            loading.set(true);
            match API_CLIENT.crud_list(&endpoint).await {
                Ok(data) => items.set(data),
                Err(e) => toast.error(&t_fmt(*lang.read(), "common.load_failed", &[("error", &e.to_string())])),
            }
            loading.set(false);
        });
    });

    // 2. Table action definitions
    let actions = vec![
        ActionDef {
            action_id: "edit".to_string(),
            label: t(*lang.read(), "common.edit").to_string(),
            color: String::new(),
        },
        ActionDef {
            action_id: "delete".to_string(),
            label: t(*lang.read(), "common.delete").to_string(),
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
                        toast.success(&t_fmt(*lang.read(), "common.entity_deleted", &[("label", &label), ("name", &name)]));
                        show_delete_confirm.set(false);
                        // Refresh list
                        match API_CLIENT.crud_list(&endpoint).await {
                            Ok(data) => items.set(data),
                            Err(e) => toast.error(&t_fmt(*lang.read(), "common.refresh_failed", &[("error", &e.to_string())])),
                        }
                    }
                    Err(e) => {
                        toast.error(&t_fmt(*lang.read(), "common.delete_failed", &[("error", &e.to_string())]));
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
                        toast.success(&t_fmt(*lang.read(), "common.entity_saved", &[("label", &label)]));
                        show_form.set(false);
                        // Refresh list
                        match API_CLIENT.crud_list(&endpoint).await {
                            Ok(data) => items.set(data),
                            Err(e) => toast.error(&t_fmt(*lang.read(), "common.refresh_failed", &[("error", &e.to_string())])),
                        }
                    }
                    Err(e) => {
                        toast.error(&t_fmt(*lang.read(), "common.save_failed", &[("error", &e.to_string())]));
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
                    {t_fmt(*lang.read(), "crud.new_entity", &[("label", &entity_label)])}
                }
            }

            // Body: The Table or Empty State
            div { class: "bg-card border rounded-xl overflow-hidden shadow-sm",
                if !*loading.read() && items.read().is_empty() {
                    EmptyState {
                        icon: rsx! { span { style: "font-size:32px", "📭" } },
                        title: t_fmt(*lang.read(), "crud.no_entity", &[("label", &entity_label)]),
                        description: Some(t_fmt(*lang.read(), "crud.create_first", &[("label", &entity_label)])),
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
                    FormMode::Create => t_fmt(*lang.read(), "crud.create_entity", &[("label", &entity_label)]),
                    FormMode::Edit => t_fmt(*lang.read(), "crud.edit_entity", &[("label", &entity_label)]),
                    FormMode::View => t_fmt(*lang.read(), "crud.view_entity", &[("label", &entity_label)]),
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
                title: t(*lang.read(), "common.confirm_delete").to_string(),
                onclose: move |_| show_delete_confirm.set(false),

                div { role: "dialog", aria_modal: "true",
                    p { class: "mb-lg",
                        {t_fmt(*lang.read(), "common.delete_confirm_msg", &[("label", &entity_label)])}
                        span { class: "font-bold", "{delete_target_name}" }
                        {t(*lang.read(), "common.delete_confirm_suffix")}
                    }
                    div { class: "flex justify-end gap-sm",
                        BCButton {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| show_delete_confirm.set(false),
                            {t(*lang.read(), "common.cancel")}
                        }
                        BCButton {
                            variant: ButtonVariant::Danger,
                            disabled: *saving.read(),
                            onclick: confirm_delete,
                            {if *saving.read() { t(*lang.read(), "common.deleting").to_string() } else { t(*lang.read(), "common.confirm").to_string() }}
                        }
                    }
                }
            }
        }
    }
}