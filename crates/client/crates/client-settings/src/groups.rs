// UI settings — HTTP response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{FormMode, SchemaForm};
use burncloud_client_shared::schema::group_schema;
use dioxus::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Channel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub match_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupMember {
    pub group_id: String,
    pub upstream_id: String,
    pub weight: i32,
}

#[component]
pub fn GroupManager() -> Element {
    let mut groups = use_signal::<Vec<Group>>(Vec::new);
    let mut all_channels = use_signal::<Vec<Channel>>(Vec::new);
    let mut loading = use_signal(|| true);
    let _error_msg = use_signal(String::new);

    // Editor State
    let mut selected_group_id = use_signal::<Option<String>>(|| None);
    let mut selected_group_members = use_signal::<Vec<GroupMember>>(Vec::new);

    // Form State (New Group) - SchemaForm uses Signal<serde_json::Value>
    let form_data = use_signal(|| {
        serde_json::json!({
            "name": "",
            "strategy": "round_robin",
            "match_path": "/v1/chat/completions"
        })
    });

    let schema = group_schema();

    // Load Data
    let fetch_data = move || async move {
        loading.set(true);
        let client = Client::new();

        if let Ok(resp) = client
            .get("http://127.0.0.1:3000/console/api/groups")
            .send()
            .await
        {
            if let Ok(data) = resp.json::<Vec<Group>>().await {
                groups.set(data);
            }
        }

        if let Ok(resp) = client
            .get("http://127.0.0.1:3000/console/api/channels")
            .send()
            .await
        {
            if let Ok(data) = resp.json::<Vec<Channel>>().await {
                all_channels.set(data);
            }
        }

        loading.set(false);
    };

    use_effect(move || {
        spawn(async move {
            fetch_data().await;
        });
    });

    // Create group via SchemaForm submission
    let handle_create = {
        let mut form_data = form_data;
        move |value: serde_json::Value| {
            spawn(async move {
                let client = Client::new();
                let new_group = Group {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: value["name"].as_str().unwrap_or("").to_string(),
                    strategy: value["strategy"]
                        .as_str()
                        .unwrap_or("round_robin")
                        .to_string(),
                    match_path: value["match_path"].as_str().unwrap_or("").to_string(),
                };

                if let Ok(resp) = client
                    .post("http://127.0.0.1:3000/console/api/groups")
                    .json(&new_group)
                    .send()
                    .await
                {
                    if resp.status().is_success() {
                        fetch_data().await;
                        form_data.set(serde_json::json!({
                            "name": "",
                            "strategy": "round_robin",
                            "match_path": "/v1/chat/completions"
                        }));
                    }
                }
            });
        }
    };

    let delete_group = move |id: String| async move {
        let client = Client::new();
        if let Ok(resp) = client
            .delete(format!("http://127.0.0.1:3000/console/api/groups/{}", id))
            .send()
            .await
        {
            if resp.status().is_success() {
                fetch_data().await;
                if selected_group_id() == Some(id) {
                    selected_group_id.set(None);
                }
            }
        }
    };

    let select_group = move |id: String| async move {
        selected_group_id.set(Some(id.clone()));
        let client = Client::new();
        if let Ok(resp) = client
            .get(format!(
                "http://127.0.0.1:3000/console/api/groups/{}/members",
                id
            ))
            .send()
            .await
        {
            if let Ok(members) = resp.json::<Vec<GroupMember>>().await {
                selected_group_members.set(members);
            }
        }
    };

    let mut add_member = move |upstream_id: String| {
        if let Some(gid) = selected_group_id() {
            let mut members = selected_group_members();
            if !members.iter().any(|m| m.upstream_id == upstream_id) {
                members.push(GroupMember {
                    group_id: gid,
                    upstream_id,
                    weight: 1,
                });
                selected_group_members.set(members);
            }
        }
    };

    let mut remove_member = move |upstream_id: String| {
        let mut members = selected_group_members();
        members.retain(|m| m.upstream_id != upstream_id);
        selected_group_members.set(members);
    };

    let save_members = move |_| async move {
        if let Some(gid) = selected_group_id() {
            let client = Client::new();
            let body: Vec<serde_json::Value> = selected_group_members()
                .iter()
                .map(|m| serde_json::json!({ "upstream_id": m.upstream_id, "weight": m.weight }))
                .collect();

            if let Ok(resp) = client
                .put(format!(
                    "http://127.0.0.1:3000/console/api/groups/{}/members",
                    gid
                ))
                .json(&body)
                .send()
                .await
            {
                if resp.status().is_success() {
                    // success toast?
                }
            }
        }
    };

    rsx! {
        div { class: "flex flex-col gap-lg",
            // Create
            div { class: "bc-card-solid",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "新建分组" }
                    SchemaForm {
                        schema: schema.clone(),
                        data: form_data,
                        mode: FormMode::Create,
                        show_actions: false,
                        class: "flex flex-row gap-md items-end",
                        on_submit: handle_create,
                    }
                }
            }

            div { class: "grid gap-lg", style: "grid-template-columns: 1fr 2fr;",
                // Group List
                div { class: "bc-card-solid h-full",
                    div { class: "p-lg",
                        h3 { class: "text-subtitle font-semibold mb-md", "分组列表" }
                        div { class: "flex flex-col gap-sm",
                            {groups().iter().map(|group| {
                                let gid1 = group.id.clone();
                                let gid2 = group.id.clone();
                                rsx! {
                                    div {
                                        class: "p-sm cursor-pointer",
                                        style: if selected_group_id() == Some(group.id.clone()) {{
                                            "background: var(--bc-primary-light); border-radius: var(--bc-radius-md); border: 1px solid var(--bc-primary);"
                                        }} else {{
                                            "background: var(--bc-bg-hover); border-radius: var(--bc-radius-md); border: 1px solid transparent;"
                                        }},
                                        onclick: move |_| select_group(gid1.clone()),
                                        div { class: "flex justify-between items-center",
                                            span { class: "font-medium", "{group.name}" }
                                            button { class: "btn btn-subtle",
                                                style: "color: var(--bc-danger); min-height: auto; padding: var(--bc-space-1);",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    let id = gid2.clone();
                                                    spawn(async move { delete_group(id).await; });
                                                },
                                                "🗑️"
                                            }
                                        }
                                        div { class: "text-caption text-secondary", "{group.match_path}" }
                                    }
                                }
                            })}
                        }
                    }
                }

                // Member Editor
                div { class: "bc-card-solid",
                    div { class: "p-lg",
                        if let Some(gid) = selected_group_id() {
                            h3 { class: "text-subtitle font-semibold mb-md", "编辑成员: {gid}" }

                            // Current Members
                            div { class: "mb-lg",
                                h4 { class: "text-caption font-bold text-secondary mb-sm", "当前成员" }
                                div { class: "flex flex-col gap-sm",
                                    {selected_group_members().iter().map(|member| {
                                        let uid1 = member.upstream_id.clone();
                                        let uid2 = member.upstream_id.clone();
                                        rsx! {
                                            div { class: "flex items-center justify-between p-sm",
                                                style: "background: var(--bc-bg-card-solid); border: 1px solid var(--bc-border); border-radius: var(--bc-radius-md);",
                                                span {
                                                    if let Some(ch) = all_channels().iter().find(|c| c.id == member.upstream_id) {
                                                        "{ch.name}"
                                                    } else {
                                                        "{member.upstream_id}"
                                                    }
                                                }
                                                div { class: "flex gap-sm items-center",
                                                    span { class: "text-caption", "权重:" }
                                                    input { class: "input",
                                                        style: "padding: 0 var(--bc-space-1); width: 64px; text-align: center; min-height: auto;",
                                                        r#type: "number",
                                                        value: "{member.weight}",
                                                        oninput: move |e| {
                                                            let w = e.value().parse().unwrap_or(1);
                                                            let id = uid1.clone();
                                                            let mut members = selected_group_members();
                                                            if let Some(m) = members.iter_mut().find(|m| m.upstream_id == id) {
                                                                m.weight = w;
                                                            }
                                                            selected_group_members.set(members);
                                                        }
                                                    }
                                                    button { class: "btn btn-subtle",
                                                        style: "color: var(--bc-danger); min-height: auto; padding: var(--bc-space-1);",
                                                        onclick: move |_| remove_member(uid2.clone()),
                                                        "✕"
                                                    }
                                                }
                                            }
                                        }
                                    })}
                                    if selected_group_members().is_empty() {
                                        div { class: "text-secondary text-center p-md",
                                            style: "font-style: italic;",
                                            "暂无成员"
                                        }
                                    }
                                }
                            }

                            // Add Member
                            div { class: "mb-lg",
                                h4 { class: "text-caption font-bold text-secondary mb-sm", "添加成员" }
                                div { class: "flex gap-sm", style: "flex-wrap: wrap;",
                                    for channel in all_channels() {
                                        if !selected_group_members().iter().any(|m| m.upstream_id == channel.id) {
                                            button { class: "btn btn-secondary",
                                                style: "padding: var(--bc-space-1) var(--bc-space-3); font-size: var(--bc-font-sm);",
                                                onclick: move |_| add_member(channel.id.clone()),
                                                "+ {channel.name}"
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "flex justify-end",
                                button { class: "btn btn-primary", onclick: save_members, "保存更改" }
                            }
                        } else {
                            div { class: "flex items-center justify-center h-full text-secondary", "请选择左侧分组进行编辑" }
                        }
                    }
                }
            }
        }
    }
}
