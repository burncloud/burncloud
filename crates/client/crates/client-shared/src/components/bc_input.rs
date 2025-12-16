use dioxus::prelude::*;

#[component]
pub fn BCInput(
    #[props(default)] value: String,
    #[props(default)] label: Option<String>,
    #[props(default = "text".to_string())] r#type: String,
    #[props(default)] placeholder: String,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: EventHandler<FormEvent>,
    /// Optional icon element to display inside the input (inner icon pattern)
    #[props(default)]
    icon: Option<Element>,
) -> Element {
    let has_icon = icon.is_some();
    let has_value = !value.is_empty();

    // Error state styling
    let error_border = if error.is_some() {
        "border-red-500/50 focus:border-red-500 focus:ring-red-500/20"
    } else {
        "border-transparent focus:border-accent focus:ring-accent/15"
    };

    // Padding adjustment for icon
    let input_padding = if has_icon { "pl-11" } else { "pl-4" };

    // Base input classes split for readability
    let base_classes = "bc-input peer w-full h-12 pr-4 rounded-xl text-[15px]";
    let bg_classes = "bg-white/5 focus:bg-white/80";
    let text_classes = "text-primary placeholder-transparent";
    let transition_classes = "transition-all duration-200 ease-out";
    let focus_classes = "focus:outline-none focus:ring-4 focus:scale-[1.02] origin-center";
    let border_classes = format!("border {}", error_border);

    // Combine all classes
    let input_class = format!(
        "{} {} {} {} {} {} {}",
        base_classes, input_padding, bg_classes, text_classes, transition_classes, focus_classes, border_classes
    );

    // Label positioning classes
    let label_left = if has_icon { "left-11" } else { "left-4" };
    let label_value_state = if has_value {
        "top-1 translate-y-0 text-[11px] font-medium"
    } else {
        ""
    };

    rsx! {
        div { class: "bc-input-group relative mb-4",
            // Floating label + input container
            div { class: "relative group",
                // Inner Icon (if provided)
                if let Some(icon_el) = icon {
                    div {
                        class: "absolute left-4 top-1/2 -translate-y-1/2 \
                                text-secondary group-focus-within:text-accent \
                                transition-colors duration-200 z-10 pointer-events-none",
                        {icon_el}
                    }
                }

                // Input field with physics effects
                input {
                    class: "{input_class}",
                    r#type: "{r#type}",
                    value: "{value}",
                    placeholder: "{placeholder}",
                    oninput: move |e| oninput.call(e),
                }

                // Floating Label
                if let Some(l) = label {
                    label {
                        class: "absolute text-secondary transition-all duration-200 ease-out pointer-events-none \
                                {label_left} \
                                peer-placeholder-shown:top-1/2 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:text-[15px] \
                                peer-focus:top-1 peer-focus:translate-y-0 peer-focus:text-[11px] peer-focus:text-accent peer-focus:font-medium \
                                {label_value_state}",
                        "{l}"
                    }
                }
            }

            // Error message
            if let Some(err) = error {
                div { class: "mt-1.5 text-[12px] text-error font-medium pl-1",
                    "{err}"
                }
            }
        }
    }
}
