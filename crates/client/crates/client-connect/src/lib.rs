// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    BCButton, BCModal, ButtonVariant,
    FormMode, PageHeader, SchemaForm, StatKpi, StatusPill, ColumnDef, PageTable,
    EmptyState, SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::channel_service::{Channel, ChannelService};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

/// AWS connection form schema
fn aws_connect_schema() -> serde_json::Value {
    serde_json::json!({
        "entity_type": "aws_connect",
        "label": "接入本地资源",
        "fields": [
            {
                "key": "name",
                "label": "资源别名",
                "type": "text",
                "required": true,
                "placeholder": "例如: 生产环境-AWS"
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
            {"title": "AWS 凭证", "fields": ["name", "aws_ak", "aws_region", "aws_sk"]}
        ]
    })
}

#[component]
pub fn ConnectPage() -> Element {
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

    let aws_schema = aws_connect_schema();

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
                    toast.success("资源已接入");
                }
                Err(e) => toast.error(&format!("接入失败: {}", e)),
            }
        });
    };

    let columns = vec![
        ColumnDef { key: "id".to_string(), label: "ID".to_string(), width: Some("60px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "状态".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "name".to_string(), label: "名称".to_string(), width: None },
        ColumnDef { key: "models".to_string(), label: "模型".to_string(), width: None },
        ColumnDef { key: "base_url".to_string(), label: "Base URL".to_string(), width: None },
    ];

    rsx! {
        PageHeader {
            title: "算力互联",
            subtitle: Some("BurnCloud Connect".to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| show_add_modal.set(true),
                    "接入本地资源"
                }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",
            // KPI strip
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "Active Nodes".to_string(),
                        value: format!("{active_nodes}"),
                    }
                    StatKpi {
                        label: "Network Capacity".to_string(),
                        value: "1.2 PFlops".to_string(),
                    }
                    StatKpi {
                        label: "Pool Balance".to_string(),
                        value: "$ 42.50".to_string(),
                    }
                    StatKpi {
                        label: "Efficiency Gain".to_string(),
                        value: "34.2%".to_string(),
                    }
                }
            }

            // Tabs
            div {
                div { class: "tabs",
                    span {
                        class: if active_tab() == "local" { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set("local".to_string()),
                        "本地算力"
                    }
                    span {
                        class: if active_tab() == "net" { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set("net".to_string()),
                        "网络互联"
                    }
                    span {
                        class: if active_tab() == "settle" { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set("settle".to_string()),
                        "结算账单"
                    }
                }

                if active_tab() == "local" {
                    div { style: "display:flex; flex-direction:column; gap:28px; margin-top:24px",
                        // Local resources
                        div {
                            div { class: "section-h lg",
                                div { class: "lead",
                                    span { class: "lead-title", "本地资源矩阵" }
                                    span { class: "lead-sub", "当前节点贡献给网络的算力资源" }
                                }
                            }

                            if loading {
                                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                            } else if ch_list.is_empty() {
                                EmptyState {
                                    icon: rsx! { span { style: "font-size:40px", "🖥️" } },
                                    title: "暂无本地资源".to_string(),
                                    description: Some("请接入 AWS 账号开始共享算力".to_string()),
                                    cta: Some(rsx! {
                                        BCButton {
                                            class: "btn-black",
                                            onclick: move |_| show_add_modal.set(true),
                                            "立即接入 AWS 账号"
                                        }
                                    }),
                                }
                            } else {
                                PageTable {
                                    columns: columns,
                                    for ch in &ch_list {
                                        tr {
                                            key: "{ch.id}",
                                            td { class: "mono", style: "font-size:12px", "#{ch.id}" }
                                            td {
                                                StatusPill {
                                                    value: if ch.status == 1 { "ok".to_string() } else { "down".to_string() }
                                                }
                                            }
                                            td { style: "font-weight:500", "{ch.name}" }
                                            td { class: "mono", style: "font-size:12px", "{ch.models}" }
                                            td { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary)", "{ch.base_url}" }
                                        }
                                    }
                                }
                            }
                        }

                        // Connected pools
                        div { style: "padding-top:24px; border-top:1px solid var(--bc-border)",
                            div { class: "section-h lg",
                                div { class: "lead",
                                    span { class: "lead-title", "互联算力池 (Sourcing)" }
                                    span { class: "lead-sub", "接入外部专业矿池以采购全球算力" }
                                }
                            }

                            // Featured pool card
                            div { class: "row-card", style: "padding:20px; margin-bottom:24px",
                                div { style: "display:flex; align-items:center; gap:16px",
                                    div { style: "width:40px; height:40px; border-radius:99px; background:var(--bc-primary-light); display:flex; align-items:center; justify-content:center; font-size:20px", "🌐" }
                                    div {
                                        div { style: "display:flex; align-items:center; gap:8px",
                                            h3 { style: "font-size:15px; font-weight:700; margin:0", "SkyNet Prime (官方合作伙伴)" }
                                            span { class: "pill success", style: "font-size:10px", "官方推荐" }
                                        }
                                        div { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary); margin-top:4px", "https://pool.skynet-ops.io" }
                                    }
                                }

                                div { style: "display:flex; align-items:center; gap:32px",
                                    div { style: "text-align:right",
                                        div { style: "font-size:10px; color:var(--bc-text-tertiary); text-transform:uppercase; letter-spacing:0.16em", "Status" }
                                        div { style: "font-size:13px; font-weight:500; margin-top:2px; color:var(--bc-success)", "● 已连接" }
                                    }
                                    div { style: "text-align:right",
                                        div { style: "font-size:10px; color:var(--bc-text-tertiary); text-transform:uppercase; letter-spacing:0.16em", "Latency" }
                                        div { style: "font-size:13px; font-weight:500; margin-top:2px", "45ms" }
                                    }
                                    div { style: "text-align:right",
                                        div { style: "font-size:10px; color:var(--bc-text-tertiary); text-transform:uppercase; letter-spacing:0.16em", "Capacity" }
                                        div { style: "font-size:13px; font-weight:500; margin-top:2px", "842 Nodes" }
                                    }
                                    div { style: "padding-left:24px; border-left:1px solid var(--bc-border)",
                                        div { style: "font-size:10px; color:var(--bc-text-tertiary); text-transform:uppercase; letter-spacing:0.16em", "My Balance" }
                                        div { style: "font-size:17px; font-weight:700; color:var(--bc-primary); margin-top:2px", "$ 12.50" }
                                    }
                                    button { class: "btn btn-ghost", "配置" }
                                }
                            }

                            // Marketplace
                            div { style: "padding-left:20px; margin-left:8px; border-left:2px solid var(--bc-border)",
                                div { class: "config-label", style: "margin-bottom:12px; color:var(--bc-text-secondary); font-weight:700", "算力池实时可用资源" }
                                div { style: "display:grid; grid-template-columns:repeat(3, 1fr); gap:12px",
                                    MarketplaceCard { provider: "AWS", region: "us-east-1", latency: "12ms", price: "0.002", trust: 99, nodes: 312 }
                                    MarketplaceCard { provider: "Azure", region: "japan-east", latency: "88ms", price: "0.0018", trust: 95, nodes: 128 }
                                    MarketplaceCard { provider: "AWS", region: "eu-central-1", latency: "115ms", price: "0.0021", trust: 98, nodes: 240 }
                                }
                            }
                        }
                    }
                } else if active_tab() == "net" {
                    div { style: "margin-top:24px",
                        EmptyState {
                            icon: rsx! { span { style: "font-size:40px", "🌐" } },
                            title: "网络拓扑视图加载中…".to_string(),
                            description: None,
                            cta: None,
                        }
                    }
                } else {
                    div { style: "margin-top:24px",
                        EmptyState {
                            icon: rsx! { span { style: "font-size:40px", "📄" } },
                            title: "暂无结算单据".to_string(),
                            description: None,
                            cta: None,
                        }
                    }
                }
            }
        }

        // Add AWS modal
        BCModal {
            title: "接入本地资源 (Miner)".to_string(),
            open: show_add_modal(),
            onclose: move |_| show_add_modal.set(false),

            div { class: "flex flex-col gap-lg p-lg",
                p { class: "text-secondary text-caption",
                    "输入您的 AWS IAM 用户凭证。您的凭证将保持在本地加密存储，仅用于算力互联。 "
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
                        "取消"
                    }
                    BCButton {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| {
                            let data = form_data.read().clone();
                            handle_add_aws(data);
                        },
                        "验证并开启互联"
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
    rsx! {
        div { class: "pick-card",
            div { style: "display:flex; justify-content:space-between; align-items:flex-start",
                div {
                    span { class: "pill neutral", style: "font-size:10px", "{provider}" }
                    h3 { style: "font-size:13px; font-weight:700; margin:6px 0 0", "{region}" }
                    div { class: "mono", style: "font-size:10px; color:var(--bc-text-tertiary); margin-top:2px", "{latency} · {nodes} nodes" }
                }
                div { style: "text-align:right",
                    div { style: "font-size:14px; font-weight:700; color:var(--bc-primary)", "${price}" }
                    div { style: "font-size:10px; color:var(--bc-text-tertiary)", "/ 1K tok" }
                }
            }
            div { style: "display:flex; justify-content:space-between; align-items:center; margin-top:12px; padding-top:10px; border-top:1px solid var(--bc-border)",
                span { style: "font-size:11px; color:var(--bc-text-secondary)", "trust ", span { class: "mono", style: "font-weight:600; color:var(--bc-text-primary)", "{trust}" } }
                button { class: "btn btn-ghost", style: "min-height:24px; padding:2px 10px; font-size:12px", "接入" }
            }
        }
    }
}