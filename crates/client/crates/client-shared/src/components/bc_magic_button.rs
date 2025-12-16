use dioxus::prelude::*;

#[component]
pub fn BCMagicButton(
    #[props(default)] class: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    #[props(default)] loading: bool,
    #[props(default)] disabled: bool,
    children: Element,
) -> Element {
    let is_disabled = loading || disabled;
    let cursor_class = if is_disabled {
        "cursor-not-allowed opacity-75"
    } else {
        "cursor-pointer"
    };

    rsx! {
        button {
            class: "w-full inline-flex items-center justify-center px-6 py-4 text-[17px] font-semibold text-white transition-all duration-300 bg-gradient-to-r from-[#0071E3] to-[#5856D6] rounded-xl shadow-lg shadow-[#0071E3]/40 hover:shadow-xl hover:shadow-[#0071E3]/50 hover:from-[#0077ED] hover:to-[#6E6AE8] active:scale-[0.98] {cursor_class} {class}",
            onclick: move |e| if !is_disabled { onclick.call(e) },
            disabled: is_disabled,
            if loading {
                span { class: "animate-spin mr-2 h-5 w-5 border-2 border-white border-t-transparent rounded-full" }
            }
            {children}
        }
    }
}
