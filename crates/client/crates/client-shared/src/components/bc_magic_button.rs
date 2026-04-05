use dioxus::prelude::*;

/// **DEPRECATED**: Use `BCButton` with `variant: ButtonVariant::Gradient` instead.
///
/// This component is kept for backward compatibility only.
/// New code should use:
/// ```rust,ignore
/// BCButton { variant: ButtonVariant::Gradient, onclick: |_| {}, "Label" }
/// ```
#[component]
pub fn BCMagicButton(
    #[props(default)] class: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    #[props(default)] loading: bool,
    #[props(default)] disabled: bool,
    children: Element,
) -> Element {
    let is_disabled = loading || disabled;
    let state_class = if is_disabled {
        "cursor-not-allowed opacity-75"
    } else {
        ""
    };

    rsx! {
        button {
            class: "bc-btn-gradient {state_class} {class}",
            onclick: move |e| if !is_disabled { onclick.call(e) },
            disabled: is_disabled,
            if loading {
                span { class: "animate-spin mr-2 h-5 w-5 border-2 border-white border-t-transparent rounded-full" }
            }
            {children}
        }
    }
}
