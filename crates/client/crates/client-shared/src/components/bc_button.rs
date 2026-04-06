use dioxus::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Ghost,
    Gradient,
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub enum ButtonSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[component]
pub fn BCButton(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default)] loading: bool,
    #[props(default)] disabled: bool,
    #[props(default)] class: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    children: Element,
    #[props(default)] r#type: Option<String>,
) -> Element {
    let base_class = match variant {
        ButtonVariant::Gradient => "bc-btn-gradient",
        _ => "btn",
    };

    let variant_class = match variant {
        ButtonVariant::Primary => "btn-primary",
        ButtonVariant::Secondary => "btn-secondary",
        ButtonVariant::Danger => "btn-danger",
        ButtonVariant::Ghost => "btn-ghost",
        ButtonVariant::Gradient => "",
    };

    let size_class = match size {
        ButtonSize::Small => "text-[12px] min-h-[28px] px-sm py-xs",
        ButtonSize::Medium => "",
        ButtonSize::Large => "text-[16px] min-h-[40px] px-lg py-md",
    };

    let btn_type = r#type.unwrap_or("button".to_string());
    let loading_class = if loading || disabled {
        "opacity-75 cursor-not-allowed"
    } else {
        ""
    };

    rsx! {
        button {
            class: "{base_class} {variant_class} {size_class} {class} {loading_class}",
            r#type: "{btn_type}",
            onclick: move |e| if !loading && !disabled { onclick.call(e) },
            disabled: "{loading || disabled}",
            if loading {
                span { class: "loading loading-spinner loading-xs me-2", role: "status" }
                " "
            }
            {children}
        }
    }
}
