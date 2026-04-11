use dioxus::prelude::*;

#[component]
pub fn BCInput(
    #[props(default)] value: String,
    #[props(default)] label: Option<String>,
    #[props(default = "text".to_string())] r#type: String,
    #[props(default)] placeholder: String,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
    #[props(default)] class: String,
) -> Element {
    let state_class = if error.is_some() { "bc-input-error" } else { "" };

    rsx! {
        div { class: "bc-input-group {class}",
            if let Some(l) = label {
                label { class: "bc-input-label", "{l}" }
            }

            div { class: "bc-input bc-input-field {state_class}",
                input {
                    class: "bc-input-native",
                    r#type: "{r#type}",
                    value: "{value}",
                    placeholder: "{placeholder}",
                    oninput: move |e| oninput.call(e)
                }
            }

            if let Some(err) = error {
                div { class: "bc-input-error-row",
                    div { class: "bc-input-error-dot" }
                    span { class: "bc-input-error-text", "{err}" }
                }
            }
        }
    }
}
