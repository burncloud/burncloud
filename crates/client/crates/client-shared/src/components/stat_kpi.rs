use dioxus::prelude::*;

#[component]
pub fn StatKpi(
    label: String,
    value: String,
    large: Option<bool>,
    delta: Option<Element>,
    chart: Option<Element>,
) -> Element {
    let large_class = if large.unwrap_or(false) { "lg" } else { "" };
    rsx! {
        div { class: "stat-card",
            span { class: "stat-eyebrow", "{label}" }
            div { class: "stat-value {large_class}",
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
