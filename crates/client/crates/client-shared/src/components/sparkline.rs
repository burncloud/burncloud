use dioxus::prelude::*;

#[component]
pub fn Sparkline(data: Vec<f64>, tone: Option<String>) -> Element {
    if data.is_empty() {
        return rsx! { div { class: "spark" } };
    }

    let tone_class = tone.as_deref().unwrap_or("");

    if data.len() == 1 {
        let v = data[0];
        if v.is_nan() || v.is_infinite() || v == 0.0 {
            return rsx! { div { class: "spark" } };
        }
        return rsx! {
            div { class: "spark",
                div { class: "bar {tone_class}", style: "width:6px; height:100%; border-radius:50%; opacity:0.85;" }
            }
        };
    }

    let filtered: Vec<f64> = data.iter()
        .map(|v| if v.is_nan() || v.is_infinite() { 0.0 } else { *v })
        .collect();

    let max = filtered.iter().cloned().fold(0.0_f64, f64::max);
    if max == 0.0 {
        return rsx! { div { class: "spark" } };
    }

    rsx! {
        div { class: "spark",
            for val in filtered {
                {
                    let pct = (val / max * 100.0).clamp(2.0, 100.0);
                    rsx! {
                        div { class: "bar {tone_class}", style: "height: {pct}%;" }
                    }
                }
            }
        }
    }
}
