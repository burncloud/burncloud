// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    ActionDef, ActionEvent, BCBadge, BCButton, BCCard, BCModal, BadgeVariant, ButtonVariant,
    FormMode, PageHeader, SchemaForm, SchemaTable, StatKpi,
};
use burncloud_client_shared::schema::channel_schema;
use burncloud_client_shared::services::channel_service::{Channel, ChannelService};
use burncloud_common::types::ChannelType;
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
    let mut channels = use_signal(Vec::<Channel>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);
    let mut show_add_modal = use_signal(|| false);

    // Form state via Signal<serde_json::Value>
    let mut form_data = use_signal(|| {
        serde_json::json!({
            "name": "",
            "aws_ak": "",
            "aws_sk": "",
            "aws_region": "us-east-1"
        })
    });

    let aws_schema = aws_connect_schema();

    // Load channels
    let load_channels = move || {
        spawn(async move {
            loading.set(true);
            match ChannelService::list(0, 100).await {
                Ok(list) => {
                    channels.set(list);
                }
                Err(e) => error_msg.set(Some(e)),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load_channels();
    });

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
                type_: ChannelType::Aws as i32,
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
                    load_channels();
                    form_data.set(serde_json::json!({
                        "name": "",
                        "aws_ak": "",
                        "aws_sk": "",
                        "aws_region": "us-east-1"
                    }));
                }
                Err(e) => error_msg.set(Some(e)),
            }
        });
    };

    // Channel table data
    let schema = channel_schema();
    let table_data: Vec<serde_json::Value> = channels
        .read()
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "status": c.status.to_string(),
                "name": c.name,
                "models": c.models,
                "base_url": c.base_url
            })
        })
        .collect();

    let actions = vec![ActionDef {
        action_id: "delete".to_string(),
        label: "删除".to_string(),
        color: "var(--bc-danger)".to_string(),
    }];

    let handle_action = move |event: ActionEvent| {
        if event.action_id == "delete" {
            let channel_id = event.row["id"].as_i64().unwrap_or(0);
            spawn(async move {
                let _ = ChannelService::delete(channel_id).await;
            });
        }
    };

    rsx! {
        div { class: "p-xl flex flex-col gap-xl",
            PageHeader {
                title: "算力互联",
                subtitle: Some("BurnCloud Connect".to_string()),
                actions: rsx! {
                    BCButton {
                        class: "btn-black",
                        onclick: move |_| show_add_modal.set(true),
                        "➕ 接入本地资源"
                    }
                }
            }

            // Quick Stats
            div { class: "stats-grid cols-4",
                StatKpi {
                    label: "Active Nodes",
                    value: "{channels.read().iter().filter(|c| c.type_ == 33).count()}",
                }
                StatKpi {
                    label: "Network Capacity",
                    value: "1.2 PFlops",
                }
                StatKpi {
                    label: "Pool Balance",
                    value: "$ 42.50",
                }
                StatKpi {
                    label: "Efficiency Gain",
                    value: "34.2%",
                }
            }

            // Tabs
            div { class: "flex flex-col gap-lg",
                div { class: "flex gap-xl border-b pb-sm",
                    span { class: "font-bold border-b-2 pb-sm cursor-pointer", style: "border-color: var(--bc-primary);", "本地算力" }
                    span { class: "text-secondary cursor-pointer pb-sm", "网络互联" }
                    span { class: "text-secondary cursor-pointer pb-sm", "结算账单" }
                }

                if loading() {
                    div { class: "p-xxxl text-center text-secondary", "加载资源中..." }
                } else {
                    div { class: "flex flex-col gap-xl",
                        // Supply Side: Local Assets via SchemaTable
                        div {
                            div { class: "flex justify-between items-end mb-md",
                                h2 { class: "text-subtitle font-bold m-0", "本地资源矩阵" }
                                span { class: "text-xxs text-secondary", "当前节点贡献给网络的算力资源" }
                            }

                            if channels.read().is_empty() {
                                div { class: "bc-card-outlined p-xl text-center",
                                    style: "border-style: dashed; background: var(--bc-bg-card-solid);",
                                    p { class: "text-secondary", "暂无本地资源。请接入 AWS 账号开始共享算力。" }
                                    BCButton {
                                        variant: ButtonVariant::Secondary,
                                        onclick: move |_| show_add_modal.set(true),
                                        "立即接入 AWS 账号"
                                    }
                                }
                            } else {
                                SchemaTable {
                                    schema: schema.clone(),
                                    data: table_data,
                                    actions: actions,
                                    on_action: handle_action,
                                    on_row_click: move |_| {},
                                }
                            }
                        }

                        // Demand Side: Connected External Pools (custom UI)
                        div { class: "pt-lg border-t",
                            div { class: "flex justify-between items-center mb-md",
                                div {
                                    h2 { class: "text-subtitle font-bold m-0", "互联算力池 (Sourcing)" }
                                    p { class: "text-xxs text-secondary m-0 mt-xs", "接入外部专业矿池以采购全球算力" }
                                }
                                BCButton {
                                    variant: ButtonVariant::Secondary,
                                    "🔗 接入新算力池"
                                }
                            }

                            div { class: "flex flex-col gap-lg",
                                PoolCard {
                                    name: "SkyNet Prime (官方合作伙伴)",
                                    url: "https://pool.skynet-ops.io",
                                    status: "已连接",
                                    latency: "45ms",
                                    nodes: 842,
                                    balance: "$ 12.50",
                                    is_featured: true
                                }

                                div { class: "pl-xl ml-md",
                                    style: "border-left: 2px solid var(--bc-border);",
                                    h3 { class: "text-caption font-bold uppercase text-secondary mb-md", "算力池实时可用资源" }
                                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-md",
                                        MarketplaceCard {
                                            provider: "AWS",
                                            region: "us-east-1",
                                            latency: "12ms",
                                            price: "0.002",
                                            trust_score: 99,
                                            nodes: 312
                                        }
                                        MarketplaceCard {
                                            provider: "Azure",
                                            region: "japan-east",
                                            latency: "88ms",
                                            price: "0.0018",
                                            trust_score: 95,
                                            nodes: 128
                                        }
                                        MarketplaceCard {
                                            provider: "AWS",
                                            region: "eu-central-1",
                                            latency: "115ms",
                                            price: "0.0021",
                                            trust_score: 98,
                                            nodes: 240
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Modal for adding AWS account via SchemaForm
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
}

#[component]
fn PoolCard(
    name: &'static str,
    url: &'static str,
    status: &'static str,
    latency: &'static str,
    nodes: i32,
    balance: &'static str,
    is_featured: bool,
) -> Element {
    rsx! {
        BCCard {
            div { class: "p-lg flex items-center justify-between",
                div { class: "flex items-center gap-md",
                    div { class: "w-10 h-10 rounded-full flex items-center justify-center text-xl",
                        style: "background: var(--bc-primary-light);",
                        "🌐"
                    }
                    div {
                        div { class: "flex items-center gap-sm",
                            h3 { class: "font-bold m-0 text-primary", "{name}" }
                            if is_featured {
                                BCBadge { variant: BadgeVariant::Success, "官方推荐" }
                            }
                        }
                        div { class: "text-caption text-secondary font-mono mt-xs", "{url}" }
                    }
                }

                div { class: "flex items-center gap-xl",
                    div { class: "text-right",
                        div { class: "text-xxs text-secondary uppercase", "Status" }
                        div { class: "text-caption font-medium", style: "color: var(--bc-success);", "● {status}" }
                    }
                    div { class: "text-right",
                        div { class: "text-xxs text-secondary uppercase", "Latency" }
                        div { class: "text-caption font-medium text-primary", "{latency}" }
                    }
                    div { class: "text-right",
                        div { class: "text-xxs text-secondary uppercase", "Capacity" }
                        div { class: "text-caption font-medium text-primary", "{nodes} Nodes" }
                    }
                    div { class: "text-right pl-lg border-l",
                        div { class: "text-xxs text-secondary uppercase", "My Balance" }
                        div { class: "text-lg font-bold", style: "color: var(--bc-primary);", "{balance}" }
                    }
                    BCButton { variant: ButtonVariant::Ghost, "配置" }
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
    trust_score: i32,
    nodes: i32,
) -> Element {
    rsx! {
        div { class: "bc-card-solid",
            div { class: "p-md flex flex-col gap-sm",
                div { class: "flex justify-between items-start",
                    div {
                        BCBadge { variant: BadgeVariant::Neutral, "{provider}" }
                        h3 { class: "text-caption font-bold mt-xs mb-0 text-primary", "{region}" }
                    }
                    div { class: "text-right",
                        div { class: "text-caption font-bold", style: "color: var(--bc-primary);", "${price}" }
                        div { class: "text-xxs text-secondary", "/ 1K" }
                    }
                }
                div { class: "flex justify-between items-center mt-sm",
                    span { class: "text-xs text-secondary", "{nodes} Nodes" }
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        class: "btn-xs",
                        "接入"
                    }
                }
            }
        }
    }
}
