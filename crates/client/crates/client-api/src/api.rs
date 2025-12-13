use burncloud_client_shared::{ChannelDto, API_CLIENT};
use dioxus::prelude::*;

#[component]
pub fn ApiManagement() -> Element {
    // Resource to fetch channels
    let mut channels =
        use_resource(move || async move { API_CLIENT.list_channels().await.unwrap_or_default() });

    let mut show_create_modal = use_signal(|| false);

    // Delete handler
    let delete_channel = move |id: String| async move {
        if API_CLIENT.delete_channel(&id).await.is_ok() {
            channels.restart();
        }
    };

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0", "渠道管理 (Channels)" }
                    p { class: "text-secondary m-0 mt-sm", "管理上游模型渠道和API密钥" }
                }
                div { class: "flex gap-sm",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| { channels.restart(); },
                        "刷新"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| { show_create_modal.set(true); },
                        "新建渠道"
                    }
                }
            }
        }

        div { class: "page-content",
            div { class: "card",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "已配置渠道" }

                    if let Some(list) = channels.read().as_ref() {
                        div { class: "flex flex-col gap-md",
                            for channel in list {
                                ChannelRow {
                                    key: "{channel.id}",
                                    channel: channel.clone(),
                                    on_delete: move |id| {
                                        spawn(async move {
                                            delete_channel(id).await;
                                        });
                                    }
                                }
                            }
                            if list.is_empty() {
                                div { class: "text-center text-secondary p-lg", "暂无渠道，请添加。" }
                            }
                        }
                    } else {
                        div { class: "text-center p-lg", "加载中..." }
                    }
                }
            }
        }

        if show_create_modal() {
            CreateChannelModal {
                on_close: move |_| show_create_modal.set(false),
                on_create: move |_| {
                    show_create_modal.set(false);
                    channels.restart();
                }
            }
        }
    }
}

#[component]
fn ChannelRow(channel: ChannelDto, on_delete: EventHandler<String>) -> Element {
    rsx! {
        div { class: "flex justify-between items-center p-md border-b",
            div {
                div { class: "font-medium", "{channel.name}" }
                div { class: "text-caption text-secondary", "{channel.base_url} ({channel.auth_type})" }
            }
            div { class: "flex items-center gap-sm",
                span { class: "status-indicator status-running",
                    span { class: "status-dot" }
                    "Active"
                }
                button {
                    class: "btn btn-sm btn-danger",
                    onclick: move |_| on_delete.call(channel.id.clone()),
                    "删除"
                }
            }
        }
    }
}

#[component]

fn CreateChannelModal(on_close: EventHandler<()>, on_create: EventHandler<()>) -> Element {
    let mut name = use_signal(String::new);

    let mut base_url = use_signal(|| String::from("https://api.openai.com"));

    let mut api_key = use_signal(String::new);

    let mut match_path = use_signal(|| String::from("/v1/chat/completions"));

    let mut auth_type = use_signal(|| String::from("Bearer"));

    let id = use_signal(String::new);

    let handle_submit = move |_| {
        let new_channel = ChannelDto {
            id: if id().is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                id()
            },

            name: name(),

            base_url: base_url(),

            api_key: api_key(),

            match_path: match_path(),

            auth_type: auth_type(),

            priority: 0,
        };

        spawn(async move {
            if API_CLIENT.create_channel(new_channel).await.is_ok() {
                on_create.call(());
            } else {
                // Handle error (todo: toast)

                println!("Failed to create channel");
            }
        });
    };

    rsx! {







            // HOTFIX: Inline styles to ensure modal renders correctly







            style { "



    



                    .modal-overlay-fixed {{



    



                        position: fixed;



    



                        top: 0; left: 0; right: 0; bottom: 0;



    



                        background-color: rgba(0, 0, 0, 0.5);



    



                        display: flex;



    



                        align-items: center;



    



                        justify-content: center;



    



                        z-index: 9999;



    



                        backdrop-filter: blur(4px);



    



                    }}



    



                    .modal-box {{



    



                        background: white;



    



                        border-radius: 8px;



    



                        box-shadow: 0 4px 20px rgba(0,0,0,0.2);



    



                        width: 400px;



    



                        padding: 24px;



    



                        display: flex;



    



                        flex-direction: column;



    



                        gap: 16px;



    



                    }}



    



                    .form-input {{



    



                        width: 100%;



    



                        padding: 8px 12px;



    



                        border: 1px solid #ccc;



    



                        border-radius: 4px;



    



                        font-size: 14px;



    



                    }}



    



                    .form-label {{



    



                        display: block;



    



                        margin-bottom: 4px;



    



                        font-weight: bold;



    



                        font-size: 14px;



    



                    }}



    



                " }







        div { class: "modal-overlay-fixed",



            div { class: "modal-box",



                div { class: "modal-header",



                    h2 { class: "text-subtitle font-bold m-0", "新建渠道" }



                }







                div { class: "flex flex-col gap-sm",



                    div {



                        label { class: "form-label", "名称 (Name)" }



                        input { class: "form-input", value: "{name}", oninput: move |e| name.set(e.value()) }



                    }



                    div {



                        label { class: "form-label", "Base URL" }



                        input { class: "form-input", value: "{base_url}", oninput: move |e| base_url.set(e.value()) }



                    }



                    div {



                        label { class: "form-label", "API Key" }



                        input { class: "form-input", type: "password", value: "{api_key}", oninput: move |e| api_key.set(e.value()) }



                    }



                    div {



                        label { class: "form-label", "匹配路径 (Match Path)" }



                        input { class: "form-input", value: "{match_path}", oninput: move |e| match_path.set(e.value()) }



                    }



                    div {



                        label { class: "form-label", "鉴权类型 (Auth Type)" }



                        select { class: "form-input", value: "{auth_type}", onchange: move |e| auth_type.set(e.value()),



                            option { value: "Bearer", "Bearer Token (OpenAI)" }



                            option { value: "XApiKey", "X-Api-Key (Claude)" }



                            option { value: "GoogleAI", "Google AI" }



                            option { value: "Azure", "Azure OpenAI" }



                            option { value: "AwsSigV4", "AWS Bedrock" }



                        }



                    }



                }







                div { class: "modal-footer",



                    button { class: "btn btn-secondary", onclick: move |_| on_close.call(()), "取消" }



                    button { class: "btn btn-primary", onclick: handle_submit, "创建" }



                }



            }



        }



    }
}
