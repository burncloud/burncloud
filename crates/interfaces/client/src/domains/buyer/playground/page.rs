use super::{
    actions::code_snippet,
    model::{CodeLanguage, RoutingTier, MODEL_CATALOG, STREAMED_RESPONSE},
    state::{PlaygroundState, STREAM_CHUNK_SIZE},
};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;
use std::time::Instant;

fn tier_description(tier: RoutingTier, copy: &crate::i18n::LocaleStrings) -> &'static str {
    match tier {
        RoutingTier::Economy => copy.playground_tier_economy_description,
        RoutingTier::Standard => copy.playground_tier_standard_description,
        RoutingTier::Performance => copy.playground_tier_performance_description,
    }
}

fn tier_label(tier: RoutingTier, copy: &crate::i18n::LocaleStrings) -> &'static str {
    match tier {
        RoutingTier::Economy => copy.playground_tier_economy,
        RoutingTier::Standard => copy.playground_tier_standard,
        RoutingTier::Performance => copy.playground_tier_performance,
    }
}

#[component]
pub fn BuyerPlayground() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(PlaygroundState::default);
    let snapshot = state.read().clone();
    let selected_model = snapshot.selected_model();
    let code = code_snippet(&snapshot);
    let title_conclusion = format!(
        "{} ({} • {})",
        copy.playground_conclusion,
        selected_model.name,
        snapshot.tier.display_name()
    );

    rsx! {
        BuyerShell {
            div { class: "playground-stack",
                section { class: "playground-header",
                    h1 { {copy.playground_title} }
                    p { {copy.playground_subtitle} }
                }
                div { class: "conclusion conclusion-healthy playground-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {title_conclusion} }
                }
                div { class: "playground-grid",
                    div { class: "playground-controls",
                        section { class: "panel playground-settings",
                            h2 { class: "playground-panel-title", Icon { name: IconName::Cpu, size: 16 } span { {format!("{} & {}", copy.playground_model_select, copy.playground_routing_tier)} } }
                            div { class: "playground-field",
                                label { r#for: "playground-model", {copy.playground_model_select} }
                                select { id: "playground-model", value: selected_model.id,
                                    onchange: move |event| state.with_mut(|value| value.set_model_id(&event.value())),
                                    for model in MODEL_CATALOG {
                                        option { value: model.id, {format!("{} (${:0.2} / ${:0.2})", model.name, model.input_price_per_million, model.output_price_per_million)} }
                                    }
                                }
                            }
                            div { class: "playground-field",
                                span { class: "playground-label", {copy.playground_routing_tier} }
                                div { class: "tier-selector", role: "group", aria_label: copy.playground_routing_tier,
                                    for tier in RoutingTier::ALL {
                                        button {
                                            r#type: "button",
                                            class: if snapshot.tier == tier { "tier-option active" } else { "tier-option" },
                                            aria_pressed: if snapshot.tier == tier { "true" } else { "false" },
                                            onclick: move |_| state.with_mut(|value| value.tier = tier),
                                            {tier_label(tier, copy)}
                                        }
                                    }
                                }
                                p { class: "tier-description", {tier_description(snapshot.tier, copy)} }
                            }
                            div { class: "playground-parameters",
                                div { class: "range-label", label { r#for: "temperature", {copy.playground_temperature} } span { {format!("{:.1}", snapshot.temperature)} } }
                                input { id: "temperature", r#type: "range", min: "0", max: "2", step: "0.1", value: snapshot.temperature.to_string(), oninput: move |event| if let Ok(value) = event.value().parse::<f32>() { state.with_mut(|current| current.temperature = value) } }
                                div { class: "range-label", label { r#for: "max-tokens", {copy.playground_max_tokens} } span { {snapshot.max_tokens.to_string()} } }
                                input { id: "max-tokens", r#type: "range", min: "256", max: "8192", step: "256", value: snapshot.max_tokens.to_string(), oninput: move |event| if let Ok(value) = event.value().parse::<u32>() { state.with_mut(|current| current.max_tokens = value) } }
                            }
                        }
                        section { class: "panel playground-code-panel",
                            div { class: "playground-code-heading",
                                h2 { class: "playground-panel-title", Icon { name: IconName::Code2, size: 14 } span { {copy.playground_code_snippet_title} } }
                                div { class: "code-tabs", role: "tablist", aria_label: copy.playground_code_snippet_title,
                                    for language in CodeLanguage::ALL {
                                        button {
                                            r#type: "button",
                                            role: "tab",
                                            aria_selected: if snapshot.code_language == language { "true" } else { "false" },
                                            class: if snapshot.code_language == language { "code-tab active" } else { "code-tab" },
                                            onclick: move |_| state.with_mut(|value| value.code_language = language),
                                            {language.label()}
                                        }
                                    }
                                }
                            }
                            pre { class: "code-block", code { {code.clone()} } }
                            button { r#type: "button", class: "copy-code-button", onclick: move |_| {
                                    let script = format!("navigator.clipboard?.writeText({:?});", code);
                                    dioxus::document::eval(&script);
                                    state.with_mut(|value| value.copied = true);
                                    let mut copied_state = state;
                                    spawn(async move {
                                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                        copied_state.with_mut(|value| value.copied = false);
                                    });
                                },
                                Icon { name: if snapshot.copied { IconName::Check } else { IconName::Copy }, size: 13 }
                                span { if snapshot.copied { {copy.playground_code_copied} } else { {copy.playground_copy_code} } }
                            }
                        }
                    }
                    div { class: "playground-workspace-column",
                        section { class: "panel playground-workspace",
                            div { class: "playground-prompt-field",
                                div { class: "prompt-label-row", label { r#for: "system-prompt", {copy.playground_system_prompt} } span { {copy.playground_optional} } }
                                textarea { id: "system-prompt", rows: 2, value: snapshot.system_prompt, oninput: move |event| state.with_mut(|value| value.system_prompt = event.value()) }
                            }
                            div { class: "playground-prompt-field",
                                label { r#for: "user-prompt", {copy.playground_user_prompt} }
                                textarea { id: "user-prompt", rows: 4, value: snapshot.user_prompt, oninput: move |event| state.with_mut(|value| value.user_prompt = event.value()) }
                            }
                            div { class: "playground-runbar",
                                div { class: "attestation-copy", Icon { name: IconName::Shield, size: 16 } span { {copy.playground_attestation_verified} } }
                                div { class: "run-actions",
                                    button { r#type: "button", class: "clear-output-button", onclick: move |_| state.with_mut(PlaygroundState::clear_output), Icon { name: IconName::RotateCcw, size: 14 } span { {copy.playground_clear} } }
                                    button { r#type: "button", class: "run-inference-button", disabled: snapshot.running, onclick: move |_| {
                                            state.with_mut(PlaygroundState::begin_inference);
                                            let stream_id = state.read().stream_id;
                                            let mut streaming_state = state;
                                            spawn(async move {
                                                let started = Instant::now();
                                                let mut length = STREAM_CHUNK_SIZE;
                                                while length < STREAMED_RESPONSE.len() {
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(35)).await;
                                                    if streaming_state.read().stream_id != stream_id {
                                                        return;
                                                    }
                                                    streaming_state.with_mut(|value| value.append_stream_chunk(length));
                                                    length += STREAM_CHUNK_SIZE;
                                                }
                                                tokio::time::sleep(tokio::time::Duration::from_millis(35)).await;
                                                if streaming_state.read().stream_id == stream_id {
                                                    let total_ms = started.elapsed().as_millis() as u32 + 280;
                                                    streaming_state.with_mut(|value| value.complete_inference(total_ms));
                                                }
                                            });
                                        },
                                        Icon { name: IconName::Play, size: 14 }
                                        span { if snapshot.running { {copy.playground_running} } else { {copy.playground_run_inference} } }
                                    }
                                }
                            }
                            div { class: "playground-output",
                                div { class: "output-content",
                                    div { class: "output-heading", span { {copy.playground_response_output} } if snapshot.running { span { class: "output-running", i {}, {copy.playground_running} } } }
                                    div { class: "output-text",
                                        if snapshot.output.is_empty() {
                                            span { class: "empty-output", {format!("{} \"{}\" {}", copy.playground_empty_output_prefix, copy.playground_run_inference, copy.playground_empty_output_suffix)} }
                                        } else {
                                            {snapshot.output}
                                        }
                                    }
                                }
                                if let Some(stats) = snapshot.stats {
                                    div { class: "inference-stats",
                                        div { class: "inference-stat", span { {copy.playground_ttft} } strong { {format!("{} ms", stats.ttft_ms)} } }
                                        div { class: "inference-stat", span { {copy.playground_total_time} } strong { {format!("{} ms", stats.total_ms)} } }
                                        div { class: "inference-stat", span { {copy.playground_tokens_generated} } strong { {format!("{} / {}", stats.prompt_tokens, stats.completion_tokens)} } }
                                        div { class: "inference-stat cost", span { {copy.playground_cost_estimate} } strong { {format!("${:.6}", stats.cost_usd)} } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
