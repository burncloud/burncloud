use dioxus::prelude::*;

#[component]
pub fn StatKpi(
    label: String,
    value: String,
    large: Option<bool>,
    color: Option<String>,
    delta: Option<Element>,
    chart: Option<Element>,
) -> Element {
    let large_class = if large.unwrap_or(false) { "lg" } else { "" };
    let color_class = color.map(|c| format!("text-{c}")).unwrap_or_default();
    rsx! {
        div { class: "stat-card",
            span { class: "stat-eyebrow", "{label}" }
            div { class: "stat-value {large_class} {color_class}",
                "{value}"
            }
            div { class: "stat-footer-row",
                if let Some(delta) = delta {
                    {delta}
                }
                if let Some(chart) = chart {
                    div { class: "stat-chart-wrap",
                        {chart}
                    }
                }
            }
        }
    }
}
