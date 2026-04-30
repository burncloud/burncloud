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
    let color_style = color.map(|c| format!("color:{}", c)).unwrap_or_default();
    rsx! {
        div { class: "stat-card",
            span { class: "stat-eyebrow", "{label}" }
            div { class: "stat-value {large_class}", style: "{color_style}",
                "{value}"
            }
            div { style: "display:flex; align-items:flex-end; justify-content:space-between; gap:12px; margin-top:4px",
                if let Some(delta) = delta {
                    {delta}
                }
                if let Some(chart) = chart {
                    div { style: "flex:1; max-width:140px",
                        {chart}
                    }
                }
            }
        }
    }
}
