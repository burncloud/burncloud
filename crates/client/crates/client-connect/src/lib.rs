// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    BCButton, BCModal, ButtonVariant,
    FormMode, PageHeader, SchemaForm, StatusPill,
    EmptyState, SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::i18n::{t, t_fmt};
use burncloud_client_shared::services::channel_service::{Channel, ChannelService};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

/// AWS connection form schema
fn aws_connect_schema(lang: burncloud_client_shared::i18n::Language) -> serde_json::Value {
    serde_json::json!({
        "entity_type": "aws_connect",
        "label": t(lang, "connect.schema.label"),
        "fields": [
            {
                "key": "name",
                "label": t(lang, "connect.schema.name_label"),
                "type": "text",
                "required": true,
                "placeholder": t(lang, "connect.schema.name_placeholder")
            },
            {
                "key": "aws_ak",
                "label": "Access Key ID",
                "type": "text",
                "required": true,
                "placeholder": "AKIA..."
            },
            {
                "key": "aws_region",
                "label": "Region",
                "type": "text",
                "required": true,
                "default": "us-east-1",
                "placeholder": "us-east-1"
            },
            {
                "key": "aws_sk",
                "label": "Secret Access Key",
                "type": "password",
                "required": true,
                "placeholder": "wJalrX..."
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": t(lang, "connect.schema.section_title"), "fields": ["name", "aws_ak", "aws_region", "aws_sk"]}
        ]
    })
}

#[component]
pub fn ConnectPage() -> Element {
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;
    let mut active_tab = use_signal(|| "local".to_string());
    let mut show_add_modal = use_signal(|| false);
    let toast = use_toast();

    let mut form_data = use_signal(|| {
        serde_json::json!({
            "name": "",
            "aws_ak": "",
            "aws_sk": "",
            "aws_region": "us-east-1"
        })
    });

    let aws_schema = aws_connect_schema(*lang.read());

    let mut channels = use_resource(move || async move {
        ChannelService::list(0, 100).await.unwrap_or_default()
    });

    let ch_list = channels.read().clone().unwrap_or_default();
    let loading = channels.read().is_none();
    let active_nodes = ch_list.iter().filter(|c| c.status == 1).count();

    let handle_add_aws = move |value: serde_json::Value| {
        let name = value["name"].as_str().unwrap_or("").to_string();
        let ak = value["aws_ak"].as_str().unwrap_or("").to_string();
        let sk = value["aws_sk"].as_str().unwrap_or("").to_string();
        let region = value["aws_region"]
            .as_str()
            .unwrap_or("us-east-1")
            .to_string();

        if name.is_empty() || ak.is_empty() || sk.is_empty() {
            return;
        }

        spawn(async move {
            let key = format!("{}:{}:{}", ak, sk, region);
            let new_channel = Channel {
                type_: 99, // AWS Bedrock
                name,
                key,
                base_url: format!("https://bedrock-runtime.{}.amazonaws.com", region),
                models:
                    "anthropic.claude-3-sonnet-20240229-v1:0,anthropic.claude-3-haiku-20240307-v1:0"
                        .to_string(),
                status: 1,
                priority: 0,
                weight: 1,
                ..Default::default()
            };

            match ChannelService::create(&new_channel).await {
                Ok(_) => {
                    show_add_modal.set(false);
                    channels.restart();
                    form_data.set(serde_json::json!({
                        "name": "",
                        "aws_ak": "",
                        "aws_sk": "",
                        "aws_region": "us-east-1"
                    }));
                    toast.success(t(*lang.read(), "connect.success.connected"));
                }
                Err(e) => toast.error(&t_fmt(*lang.read(), "connect.error.connect_failed", &[("error", &e.to_string())])),
            }
        });
    };

    rsx! {
        PageHeader {
            title: t(*lang.read(), "connect.title"),
            subtitle: Some(t(*lang.read(), "connect.subtitle").to_string()),
            subtitle_class: Some("mono".to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-primary",
                    onclick: move |_| show_add_modal.set(true),
                    {t(*lang.read(), "connect.add_local")}
                }
            },
        }

        div { class: "page-content flex flex-col bc-gap-7",
            // KPI strip
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "Active Nodes" }
                        div { class: "stat-value", "{active_nodes}" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "Network Capacity" }
                        div { class: "stat-value bc-text-brand", "1.2 PFlops" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "Pool Balance" }
                        div { class: "stat-value", "$ 42.50" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "Efficiency Gain" }
                        div { class: "stat-value bc-text-success", "34.2%" }
                    }
                }
            }

            // Tabs
            div {
                div { class: "tabs",
                    button {
                        class: if active_tab() == "local" { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set("local".to_string()),
                        {t(*lang.read(), "connect.tab.local")}
                    }
                    button {
                        class: if active_tab() == "net" { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set("net".to_string()),
                        {t(*lang.read(), "connect.tab.network")}
                    }
                    button {
                        class: if active_tab() == "settle" { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set("settle".to_string()),
                        {t(*lang.read(), "connect.tab.billing")}
                    }
                }

                if active_tab() == "local" {
                    div { class: "flex flex-col bc-gap-7 mt-xxl",
                        // Local resources
                        div {
                            div { class: "section-h lg",
                                div { class: "lead",
                                    span { class: "lead-title", {t(*lang.read(), "connect.local.lead_title")} }
                                    span { class: "lead-sub", {t(*lang.read(), "connect.local.lead_sub")} }
                                }
                            }

                            if loading {
                                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                            } else if ch_list.is_empty() {
                                EmptyState {
                                    icon: rsx! { span { class: "bc-font-emoji", "🖥️" } },
                                    title: t(*lang.read(), "connect.local.empty_title").to_string(),
                                    description: Some(t(*lang.read(), "connect.local.empty_desc").to_string()),
                                    cta: Some(rsx! {
                                        BCButton {
                                            class: "btn-secondary",
                                            onclick: move |_| show_add_modal.set(true),
                                            {t(*lang.read(), "connect.local.connect_aws")}
                                        }
                                    }),
                                }
                            } else {
                                table { class: "table",
                                    thead {
                                        tr {
                                            th { "ID" }
                                            th { {t(*lang.read(), "connect.col.status")} }
                                            th { {t(*lang.read(), "connect.col.name")} }
                                            th { {t(*lang.read(), "connect.col.models")} }
                                            th { "Base URL" }
                                            th { class: "text-right", {t(*lang.read(), "connect.col.actions")} }
                                        }
                                    }
                                    tbody {
                                        for ch in &ch_list {
                                            tr {
                                                key: "{ch.id}",
                                                td { class: "mono text-caption", "#{ch.id}" }
                                                td {
                                                    StatusPill {
                                                        value: if ch.status == 1 { "ok".to_string() } else { "neutral".to_string() },
                                                        label: if ch.status == 1 { Some("Active".to_string()) } else { Some("Disabled".to_string()) },
                                                    }
                                                }
                                                td { class: "font-semibold", "{ch.name}" }
                                                td { class: "mono text-caption", "{ch.models}" }
                                                td { class: "mono text-caption text-secondary", "{ch.base_url}" }
                                                td { class: "text-right",
                                                    button { class: "btn btn-ghost bc-text-danger font-semibold", {t(*lang.read(), "connect.col.delete")} }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Connected pools
                        div { class: "mt-xxl border-t",
                            div { class: "section-h lg",
                                div { class: "lead",
                                    span { class: "lead-title", {t(*lang.read(), "connect.pool.lead_title")} }
                                    span { class: "lead-sub", {t(*lang.read(), "connect.pool.lead_sub")} }
                                }
                                button { class: "btn btn-secondary", {t(*lang.read(), "connect.pool.add")} }
                            }

                            // Featured pool card
                            div { class: "row-card p-xl mb-xxl",
                                div { class: "flex items-center gap-lg",
                                    div { class: "bc-icon-circle bc-icon-circle-brand bc-font-emoji", "🌐" }
                                    div {
                                        div { class: "flex items-center gap-sm",
                                            h3 { class: "bc-h3", {t(*lang.read(), "connect.pool.skynet_title")} }
                                            span { class: "pill success text-xxs", {t(*lang.read(), "connect.pool.official")} }
                                        }
                                        div { class: "mono text-caption text-secondary mt-xs", "https://pool.skynet-ops.io" }
                                    }
                                }

                                div { class: "flex items-center bc-gap-8",
                                    div { class: "bc-pool-metric",
                                        div { class: "bc-eyebrow", "Status" }
                                        div { class: "bc-font-13 font-medium bc-mt-2 bc-text-success", {t(*lang.read(), "connect.pool.connected")} }
                                    }
                                    div { class: "bc-pool-metric",
                                        div { class: "bc-eyebrow", "Latency" }
                                        div { class: "bc-font-13 font-medium bc-mt-2", "45ms" }
                                    }
                                    div { class: "bc-pool-metric",
                                        div { class: "bc-eyebrow", "Capacity" }
                                        div { class: "bc-font-13 font-medium bc-mt-2", "842 Nodes" }
                                    }
                                    div { class: "bc-pl-6 bc-border-l",
                                        div { class: "bc-eyebrow", "My Balance" }
                                        div { class: "bc-pool-value-brand", "$ 12.50" }
                                    }
                                    button { class: "btn btn-ghost", {t(*lang.read(), "connect.pool.configure")} }
                                }
                            }

                            // Marketplace
                            div { class: "bc-indent-left",
                                div { class: "config-label mb-md text-secondary font-bold", {t(*lang.read(), "connect.pool.available")} }
                                div { class: "bc-grid-3 gap-md",
                                    MarketplaceCard { provider: "AWS", region: "us-east-1", latency: "12ms", price: "0.002", trust: 99, nodes: 312 }
                                    MarketplaceCard { provider: "Azure", region: "japan-east", latency: "88ms", price: "0.0018", trust: 95, nodes: 128 }
                                    MarketplaceCard { provider: "AWS", region: "eu-central-1", latency: "115ms", price: "0.0021", trust: 98, nodes: 240 }
                                }
                            }
                        }
                    }
                } else if active_tab() == "net" {
                    div { class: "mt-xxl",
                        EmptyState {
                            icon: rsx! { span { class: "bc-font-emoji", "🌐" } },
                            title: t(*lang.read(), "connect.network.loading").to_string(),
                            description: None,
                            cta: None,
                        }
                    }
                } else {
                    div { class: "mt-xxl",
                        EmptyState {
                            icon: rsx! { span { class: "bc-font-emoji", "📄" } },
                            title: t(*lang.read(), "connect.billing.empty_title").to_string(),
                            description: None,
                            cta: None,
                        }
                    }
                }
            }
        }

        // Add AWS modal
        BCModal {
            title: t(*lang.read(), "connect.modal.title").to_string(),
            open: show_add_modal(),
            onclose: move |_| show_add_modal.set(false),

            div { class: "flex flex-col gap-lg p-lg",
                p { class: "text-secondary text-caption",
                    {t(*lang.read(), "connect.modal.description")}
                }

                SchemaForm {
                    schema: aws_schema.clone(),
                    data: form_data,
                    mode: FormMode::Create,
                    show_actions: false,
                    on_submit: handle_add_aws,
                }

                div { class: "flex justify-end gap-md mt-md",
                    BCButton {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| show_add_modal.set(false),
                        {t(*lang.read(), "common.cancel")}
                    }
                    BCButton {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| {
                            let data = form_data.read().clone();
                            handle_add_aws(data);
                        },
                        {t(*lang.read(), "connect.modal.submit")}
                    }
                }
            }
        }
    }
}

#[component]
fn MarketplaceCard(
    provider: &'static str,
    region: &'static str,
    latency: &'static str,
    price: &'static str,
    trust: i32,
    nodes: i32,
) -> Element {
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;

    rsx! {
        div { class: "pick-card",
            div { class: "flex justify-between items-start",
                div {
                    span { class: "pill neutral text-xxs", "{provider}" }
                    h3 { class: "bc-font-13 font-bold bc-mt-6", "{region}" }
                    div { class: "mono text-xxs text-tertiary bc-mt-2", "{latency} · {nodes} nodes" }
                }
                div { class: "text-right",
                    div { class: "bc-font-13 font-bold bc-text-brand", "${price}" }
                    div { class: "text-xxs text-tertiary", "/ 1K tok" }
                }
            }
            div { class: "bc-marketplace-footer",
                span { class: "bc-font-11 text-secondary", "trust ", span { class: "mono font-semibold text-primary", "{trust}" } }
                button { class: "btn btn-ghost bc-btn-xs", {t(*lang.read(), "connect.marketplace.connect")} }
            }
        }
    }
}