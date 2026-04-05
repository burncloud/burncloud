use dioxus::prelude::*;

#[component]
pub fn BCInput(
    #[props(default)] value: String,
    #[props(default)] label: Option<String>,
    #[props(default = "text".to_string())] r#type: String,
    #[props(default)] placeholder: String,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
    /// Optional icon element to display inside the input
    #[props(default)]
    icon: Option<Element>,
) -> Element {
    let has_error = error.is_some();
    let has_icon = icon.is_some();

    let ring_class = if has_error {
        "shadow-[0_0_0_2px_var(--bc-danger-light)] border-[var(--bc-danger)]"
    } else {
        ""
    };

    let icon_padding = if has_icon { "pl-0" } else { "pl-md" };

    rsx! {
        div { class: "bc-input-group mb-md",
            if let Some(l) = label {
                label { class: "block text-caption font-medium text-secondary mb-sm uppercase tracking-wider",
                    "{l}"
                }
            }

            div { class: "relative flex items-center w-full h-12 bc-input {ring_class}",

                if let Some(ic) = icon {
                    div { class: "pl-md pr-sm text-tertiary flex-shrink-0 flex items-center justify-center",
                        {ic}
                    }
                }

                input {
                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none pr-md text-body text-primary placeholder-text-tertiary {icon_padding}",
                    r#type: "{r#type}",
                    value: "{value}",
                    placeholder: "{placeholder}",
                    oninput: move |e| oninput.call(e)
                }
            }

            if let Some(err) = error {
                div { class: "mt-xs flex items-center gap-xs",
                    div { class: "w-1.5 h-1.5 rounded-full bg-[var(--bc-danger)]" }
                    span { class: "text-caption font-medium text-[var(--bc-danger)]", "{err}" }
                }
            }
        }
    }
}
