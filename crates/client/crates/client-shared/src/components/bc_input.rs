use dioxus::prelude::*;

#[component]
pub fn BCInput(
    #[props(default)]
    value: String,
    #[props(default)]
    label: Option<String>,
    #[props(default = "text".to_string())]
    r#type: String,
    #[props(default)]
    placeholder: String,
    #[props(default)]
    error: Option<String>,
    #[props(default)]
    oninput: EventHandler<FormEvent>,
) -> Element {
    let error_class = if error.is_some() { "is-invalid" } else { "" };

    rsx! {
        div { class: "form-group mb-3",
            if let Some(l) = label {
                label { class: "form-label fw-bold mb-1", "{l}" }
            }
            input {
                class: "form-control {error_class}",
                r#type: "{r#type}",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |e| oninput.call(e),
            }
            if let Some(err) = error {
                div { class: "invalid-feedback", "{err}" }
            }
        }
    }
}
