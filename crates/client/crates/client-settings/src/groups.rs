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

    // Form State (New Group)
    let mut form_name = use_signal(String::new);
    let mut form_strategy = use_signal(|| "round_robin".to_string());
    let mut form_match_path = use_signal(|| "/v1/chat/completions".to_string());

    // Load Data
    let fetch_data = move || async move {
        loading.set(true);
        let client = Client::new();

        // Fetch Groups
        if let Ok(resp) = client
            .get("http://127.0.0.1:3000/console/api/groups")
            .send()
            .await
        {
            if let Ok(data) = resp.json::<Vec<Group>>().await {
                groups.set(data);
            }
        }

        // Fetch Channels (for selection)
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

    // Actions
    let create_group = move |_| async move {
        let client = Client::new();
        let new_group = Group {
            id: uuid::Uuid::new_v4().to_string(),
            name: form_name(),
            strategy: form_strategy(),
            match_path: form_match_path(),
        };

        if let Ok(resp) = client
            .post("http://127.0.0.1:3000/console/api/groups")
            .json(&new_group)
            .send()
            .await
        {
            if resp.status().is_success() {
                fetch_data().await;
                form_name.set(String::new());
            }
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
                    div { class: "flex gap-md items-end",
                        div { class: "flex flex-col gap-sm flex-1",
                            label { class: "text-caption text-secondary", "名称" }
                            input { class: "input", value: "{form_name}", oninput: move |e| form_name.set(e.value()) }
                        }
                        div { class: "flex flex-col gap-sm flex-1",
                            label { class: "text-caption text-secondary", "策略" }
                            select { class: "input", value: "{form_strategy}", onchange: move |e| form_strategy.set(e.value()),
                                option { value: "round_robin", "轮询 (Round Robin)" }
                                option { value: "weighted", "权重 (Weighted)" } // Future support
                            }
                        }
                        div { class: "flex flex-col gap-sm flex-1",
                            label { class: "text-caption text-secondary", "匹配路径" }
                            input { class: "input", value: "{form_match_path}", oninput: move |e| form_match_path.set(e.value()) }
                        }
                        button { class: "btn btn-primary", onclick: create_group, "创建" }
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
                                                    // Find channel name
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
