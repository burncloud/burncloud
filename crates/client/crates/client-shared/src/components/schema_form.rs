// Dynamic form rendering from JSON schema — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use crate::i18n::{t, use_i18n, Language};
use dioxus::prelude::*;
use std::collections::HashMap;

use crate::components::{BCButton, BCInput, ButtonVariant};

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub enum FormMode {
    #[default]
    Create,
    Edit,
    View,
}

/// 检查字段是否可见
/// 支持三种格式：
/// 1. 单条件: {"field": "type", "in": ["1", "14"]}
/// 2. AND 组合: {"and": [{"field": "type", "in": ["24"]}, {"field": "auth_type", "in": ["api_key"]}]}
/// 3. OR 组合: {"or": [{"field": "type", "in": ["1"]}, {"field": "type", "in": ["14"]}]}
fn is_visible(data: &serde_json::Value, condition: &serde_json::Value) -> bool {
    // AND 组合条件
    if let Some(and_conditions) = condition.get("and").and_then(|v| v.as_array()) {
        return and_conditions.iter().all(|c| is_visible(data, c));
    }

    // OR 组合条件
    if let Some(or_conditions) = condition.get("or").and_then(|v| v.as_array()) {
        return or_conditions.iter().any(|c| is_visible(data, c));
    }

    // 单条件: {"field": "xxx", "in": ["a", "b"]}
    let field_name = condition["field"].as_str().unwrap_or("");
    let allowed = condition["in"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    let current = data.get(field_name).and_then(|v| v.as_str()).unwrap_or("");
    allowed.contains(&current)
}

/// 从 JSON Value 中提取字符串值（用于 input 显示）
fn value_to_string(val: Option<&serde_json::Value>) -> String {
    match val {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// 渲染单个 schema 字段
fn render_field(
    _index: usize,
    field: &serde_json::Value,
    mut data: Signal<serde_json::Value>,
    readonly: bool,
    errors: Signal<HashMap<String, String>>,
) -> Element {
    let key = field["key"].as_str().unwrap_or("").to_string();
    let label = field["label"].as_str().unwrap_or("").to_string();
    let field_type = field["type"].as_str().unwrap_or("text");
    let placeholder = field["placeholder"].as_str().unwrap_or("").to_string();
    let visibility = field["visibility"].as_str().unwrap_or("both");
    let field_readonly = field["readonly"].as_bool().unwrap_or(false) || readonly;

    // 跳过隐藏字段和仅表格显示的字段
    if visibility == "hidden" || visibility == "table_only" {
        return rsx! {};
    }

    // 检查条件显示
    let current_data = data.read();
    if let Some(cond) = field.get("visible_when") {
        if !is_visible(&current_data, cond) {
            return rsx! {};
        }
    }

    let error_msg = errors.read().get(&key).cloned();
    let value_str = value_to_string(current_data.get(&key));
    drop(current_data);

    match field_type {
        "text" | "password" => {
            let input_type = if field_type == "password" {
                "password"
            } else {
                "text"
            };
            let key_c = key.clone();
            rsx! {
                BCInput {
                    label: Some(label),
                    r#type: input_type.to_string(),
                    value: value_str,
                    placeholder: placeholder.clone(),
                    error: error_msg,
                    oninput: move |e: FormEvent| {
                        if let Some(m) = data.write().as_object_mut() {
                            m.insert(key_c.clone(), serde_json::Value::String(e.value()));
                        }
                    }
                }
            }
        }
        "number" => {
            let key_c = key.clone();
            rsx! {
                div { class: "bc-input-group",
                    if !label.is_empty() {
                        label { class: "bc-input-label", "{label}" }
                    }
                    div { class: "bc-input bc-input-field",
                        input {
                            class: "bc-input-native",
                            r#type: "number",
                            value: "{value_str}",
                            placeholder: "{placeholder}",
                            disabled: field_readonly,
                            oninput: move |e| {
                                let v = e.value();
                                let json_val = v.parse::<i64>()
                                    .map(serde_json::Value::from)
                                    .unwrap_or(serde_json::Value::String(v));
                                if let Some(m) = data.write().as_object_mut() {
                                    m.insert(key_c.clone(), json_val);
                                }
                            }
                        }
                    }
                    if let Some(err) = error_msg {
                        div { class: "bc-input-error-row",
                            div { class: "bc-input-error-dot" }
                            span { class: "bc-input-error-text", "{err}" }
                        }
                    }
                }
            }
        }
        "select" => {
            let options = field["options"].as_array().cloned().unwrap_or_default();
            let key_c = key.clone();
            let current_val = value_str.clone();
            rsx! {
                div { class: "bc-input-group",
                    if !label.is_empty() {
                        label { class: "bc-input-label", "{label}" }
                    }
                    div { class: "bc-input bc-input-field",
                        select {
                            class: "bc-input-native",
                            disabled: field_readonly,
                            onchange: move |e: FormEvent| {
                                if let Some(m) = data.write().as_object_mut() {
                                    m.insert(key_c.clone(), serde_json::Value::String(e.value()));
                                }
                            },
                            for opt in options.iter() {
                                {
                                    let opt_val = opt["value"].as_str().unwrap_or("");
                                    let opt_label = opt["label"].as_str().unwrap_or("");
                                    let selected = opt_val == current_val;
                                    rsx! {
                                        option { value: "{opt_val}", selected: "{selected}", "{opt_label}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "toggle" => {
            let checked = value_str == "true" || value_str == "1";
            let key_c = key.clone();
            rsx! {
                div { class: "bc-input-group flex items-center gap-sm",
                    if !label.is_empty() {
                        label { class: "bc-input-label", "{label}" }
                    }
                    input {
                        r#type: "checkbox",
                        checked: "{checked}",
                        disabled: field_readonly,
                        onchange: move |_| {
                            if let Some(m) = data.write().as_object_mut() {
                                let new_val = if checked { "false" } else { "true" };
                                m.insert(key_c.clone(), serde_json::Value::String(new_val.to_string()));
                            }
                        }
                    }
                }
            }
        }
        "textarea" => {
            let rows = field["rows"].as_u64().unwrap_or(3);
            let key_c = key.clone();
            rsx! {
                div { class: "bc-input-group",
                    if !label.is_empty() {
                        label { class: "bc-input-label", "{label}" }
                    }
                    div { class: "bc-input bc-input-field",
                        textarea {
                            class: "bc-input-native",
                            rows: "{rows}",
                            placeholder: "{placeholder}",
                            disabled: field_readonly,
                            oninput: move |e| {
                                if let Some(m) = data.write().as_object_mut() {
                                    m.insert(key_c.clone(), serde_json::Value::String(e.value()));
                                }
                            },
                            "{value_str}"
                        }
                    }
                }
            }
        }
        _ => rsx! {},
    }
}

/// 验证表单数据
pub fn validate_schema(
    schema: &serde_json::Value,
    data: &serde_json::Value,
    lang: Language,
) -> HashMap<String, String> {
    let mut errors = HashMap::new();
    let empty: Vec<serde_json::Value> = vec![];
    let fields = schema["fields"].as_array().unwrap_or(&empty);

    for field in fields {
        let key = field["key"].as_str().unwrap_or("");
        let visibility = field["visibility"].as_str().unwrap_or("both");
        if visibility == "hidden" || visibility == "table_only" {
            continue;
        }
        if let Some(cond) = field.get("visible_when") {
            if !is_visible(data, cond) {
                continue;
            }
        }
        if field["required"].as_bool().unwrap_or(false) {
            let val = data.get(key);
            let empty = match val {
                Some(serde_json::Value::String(s)) => s.trim().is_empty(),
                Some(serde_json::Value::Null) | None => true,
                _ => false,
            };
            if empty {
                errors.insert(key.to_string(), t(lang, "schema_form.field_required").to_string());
            }
        }
    }
    errors
}

/// JSON Schema 驱动的通用表单组件
///
/// 接收 JSON Schema 定义和响应式数据，自动渲染表单字段。
/// 支持 text/password/number/select/toggle/textarea 字段类型。
/// 支持 visible_when 条件显示（单条件、and/or 组合）和 required 验证。
#[component]
pub fn SchemaForm(
    schema: serde_json::Value,
    mut data: Signal<serde_json::Value>,
    #[props(default)] mode: FormMode,
    #[props(default)] on_submit: EventHandler<serde_json::Value>,
    #[props(default = true)] show_actions: bool,
    #[props(default)] class: String,
    #[props(default)] disabled: bool,
) -> Element {
    let i18n = use_i18n();
    let lang_signal = i18n.language;
    let mut errors: Signal<HashMap<String, String>> = use_signal(HashMap::new);

    let fields = schema["fields"].as_array().cloned().unwrap_or_default();
    let is_readonly = mode == FormMode::View;

    // 初始化默认值
    let mut init_data = data;
    let init_fields = fields.clone();
    use_hook(move || {
        let mut val = init_data.read().clone();
        if !val.is_object() {
            val = serde_json::Value::Object(serde_json::Map::new());
        }
        let Some(obj) = val.as_object_mut() else {
            return;
        };
        for field in &init_fields {
            let key = field["key"].as_str().unwrap_or("");
            if !obj.contains_key(key) {
                if let Some(default) = field.get("default") {
                    obj.insert(key.to_string(), default.clone());
                }
            }
        }
        init_data.set(val);
    });

    let _validate_fields = fields.clone();
    let handle_submit = move |_| {
        let current_data = data.read().clone();
        let validation_errors = validate_schema(&schema, &current_data, *lang_signal.read());

        if validation_errors.is_empty() {
            on_submit.call(current_data);
        }
        errors.set(validation_errors);
    };

    rsx! {
        div { class: "flex flex-col gap-md {class}",
            for (i, field) in fields.iter().enumerate() {
                {render_field(i, field, data, is_readonly, errors)}
            }
            if show_actions && !is_readonly {
                div { class: "flex gap-sm justify-end",
                    BCButton {
                        variant: ButtonVariant::Primary,
                        loading: disabled,
                        disabled: disabled,
                        onclick: handle_submit,
                        {t(*lang_signal.read(), "schema_form.submit")}
                    }
                }
            }
        }
    }
}