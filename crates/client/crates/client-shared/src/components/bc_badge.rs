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
    let base_class =
        "inline-flex items-center gap-xs px-sm py-xs text-caption font-semibold rounded-full";
    let variant_class = match variant {
        BadgeVariant::Success => "bc-badge-success",
        BadgeVariant::Warning => "bc-badge-warning",
        BadgeVariant::Danger => "bc-badge-danger",
        BadgeVariant::Info => "bc-badge-info",
        BadgeVariant::Neutral => "bc-badge-neutral",
    };

    rsx! {
        span {
            class: "{base_class} {variant_class} {class}",
            if dot {
                span { class: "w-1.5 h-1.5 rounded-full bg-current" }
            }
            {children}
        }
    }
}
