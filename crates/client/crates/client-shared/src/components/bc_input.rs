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
    let is_password = r#type == "password";
    let mut show_password = use_signal(|| false);
    let actual_type = if is_password && show_password() { "text" } else { &r#type };

    // Password fields maintain their own local state to prevent the browser
    // from clearing masked characters when the parent component re-renders.
    let mut local_value = use_signal(String::new);
    let display_value = if is_password { local_value() } else { value.clone() };

    rsx! {
        div { class: "bc-input-group {class}",
            if let Some(l) = label {
                label { class: "bc-input-label", "{l}" }
            }

            div { class: "bc-input bc-input-field {state_class}",
                input {
                    class: "bc-input-native",
                    style: if is_password { "padding-right: 40px;" } else { "" },
                    r#type: "{actual_type}",
                    value: "{display_value}",
                    placeholder: "{placeholder}",
                    oninput: move |e| {
                        if is_password {
                            local_value.set(e.value());
                        }
                        oninput.call(e);
                    }
                }
                if is_password {
                    button {
                        class: "flex-shrink-0 flex items-center justify-center w-10 h-full text-secondary hover:text-primary transition-colors cursor-pointer bg-transparent border-none",
                        r#type: "button",
                        tabindex: "-1",
                        onclick: move |_| show_password.set(!show_password()),
                        if show_password() {
                            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" }
                            }
                        } else {
                            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
                            }
                        }
                    }
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
