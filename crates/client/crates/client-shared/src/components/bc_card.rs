use dioxus::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub enum CardVariant {
    #[default]
    Solid,
    Glass,
    Outlined,
}

#[component]
pub fn BCCard(
    #[props(default)] variant: CardVariant,
    #[props(default)] header: Option<String>,
    #[props(default)] interactive: bool,
    #[props(default)] class: String,
    children: Element,
) -> Element {
    let variant_class = match variant {
        CardVariant::Solid => "bc-card-solid",
        CardVariant::Glass => "bc-card-glass",
        CardVariant::Outlined => "bc-card-outlined",
    };

    let interactive_class = if interactive { "card-interactive" } else { "" };

    rsx! {
        div { class: "{variant_class} {interactive_class} p-lg {class}",
            if let Some(h) = header {
                div { class: "flex items-center justify-between mb-md",
                    h3 { class: "text-subtitle font-semibold text-primary", "{h}" }
                }
            }
            {children}
        }
    }
}
