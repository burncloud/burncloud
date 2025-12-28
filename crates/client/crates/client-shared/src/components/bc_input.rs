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

    // Input container styles
    // Adapting from Login Page style but using Theme variables
    // Container: bg-base-200 -> focus:bg-base-100/white
    let container_base = "relative flex items-center w-full h-12 bg-base-200/50 hover:bg-base-200/80 shadow-sm rounded-xl transition-all duration-200";

    let container_classes = if has_error {
        format!("{} ring-2 ring-error/50 bg-base-100", container_base)
    } else {
        format!(
            "{} focus-within:ring-2 focus-within:ring-primary/50 focus-within:bg-base-100",
            container_base
        )
    };

    let icon_padding = if has_icon { "pl-0" } else { "pl-4" };

    rsx! {
        div { class: "group relative mb-4",
            // Static Label (Outside)
            if let Some(l) = label {
                label { class: "block text-[13px] font-medium text-base-content/70 mb-2 uppercase tracking-wider ml-1",
                    "{l}"
                }
            }

            // Input Wrapper
            div { class: "{container_classes}",

                // Icon (Left side)
                if let Some(ic) = icon {
                    div { class: "pl-4 pr-2 text-base-content/50 group-focus-within:text-primary transition-colors duration-300 flex-shrink-0 flex items-center justify-center",
                        {ic}
                    }
                }

                // Input Field
                input {
                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-primary pr-4 text-[15px] text-base-content placeholder-base-content/40 {icon_padding}",
                    r#type: "{r#type}",
                    value: "{value}",
                    placeholder: "{placeholder}",
                    oninput: move |e| oninput.call(e)
                }
            }

            // Error Message
            if let Some(err) = error {
                div { class: "mt-1.5 flex items-center gap-1.5 animate-in slide-in-from-top-1 fade-in duration-200",
                    div { class: "w-1.5 h-1.5 rounded-full bg-error" }
                    span { class: "text-[12px] text-error font-medium", "{err}" }
                }
            }
        }
    }
}
