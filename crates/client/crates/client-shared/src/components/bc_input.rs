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
        "border-transparent focus:border-[#007AFF] focus:ring-[#007AFF]/15"
    };

    // Padding for icon
    let input_padding = if has_icon { "pl-11" } else { "pl-4" };

    rsx! {
        div { class: "bc-input-group relative mb-4",
            // Floating label + input container
            div { class: "relative group",
                // Inner Icon (if provided)
                if let Some(icon_el) = icon {
                    div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#007AFF] transition-colors duration-200 z-10 pointer-events-none",
                        {icon_el}
                    }
                }

                // Input field with physics effects
                input {
                    class: "bc-input peer w-full h-12 {input_padding} pr-4 bg-white/5 border rounded-xl text-[15px] text-[#1D1D1F] placeholder-transparent transition-all duration-200 ease-out focus:outline-none focus:ring-4 focus:bg-white/80 focus:scale-[1.02] origin-center {error_border}",
                    r#type: "{r#type}",
                    value: "{value}",
                    placeholder: "{placeholder}",
                    oninput: move |e| oninput.call(e),
                }

                // Floating Label
                if let Some(l) = label {
                    label {
                        class: "absolute text-[#86868B] transition-all duration-200 ease-out pointer-events-none
                            {if has_icon { \"left-11\" } else { \"left-4\" }}
                            peer-placeholder-shown:top-1/2 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:text-[15px]
                            peer-focus:top-1 peer-focus:translate-y-0 peer-focus:text-[11px] peer-focus:text-[#007AFF] peer-focus:font-medium
                            {if has_value { \"top-1 translate-y-0 text-[11px] font-medium\" } else { \"\" }}",
                        "{l}"
                    }
                }
            }

            // Error message
            if let Some(err) = error {
                div { class: "mt-1.5 text-[12px] text-red-500 font-medium pl-1",
                    "{err}"
                }
            }
        }
    }
}
