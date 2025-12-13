use dioxus::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub enum BadgeVariant {
    Success,
    Warning,
    Danger,
    Info,
    #[default]
    Neutral,
}

#[component]
pub fn BCBadge(
    #[props(default)] variant: BadgeVariant,
    #[props(default)] class: String,
    #[props(default)] dot: bool,
    children: Element,
) -> Element {
    let base_class = "badge";
    let variant_class = match variant {
        BadgeVariant::Success => "badge-success",
        BadgeVariant::Warning => "badge-warning",
        BadgeVariant::Danger => "badge-danger",
        BadgeVariant::Info => "badge-info",
        BadgeVariant::Neutral => "badge-neutral",
    };

    rsx! {
        span {
            class: "{base_class} {variant_class} {class}",
            if dot {
                span { class: "badge-dot" }
            }
            {children}
        }
    }
}
