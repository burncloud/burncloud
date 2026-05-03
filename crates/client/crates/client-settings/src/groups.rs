// UI settings — HTTP response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{FormMode, SchemaForm};
use burncloud_client_shared::i18n::t;
use burncloud_client_shared::schema::group_schema;
use burncloud_client_shared::{GroupDto, GroupMemberDto, API_CLIENT};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Channel {
    pub id: String,
    pub name: String,
}

#[component]
pub fn GroupManager() -> Element {
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;
    let mut groups = use_signal::<Vec<GroupDto>>(Vec::new);
    let mut all_channels = use_signal::<Vec<Channel>>(Vec::new);
    let mut loading = use_signal(|| true);
    let _error_msg = use_signal(String::new);

    // Editor State
    let mut selected_group_id = use_signal::<Option<String>>(|| None);
    let mut selected_group_members = use_signal::<Vec<GroupMemberDto>>(Vec::new);

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

        if let Ok(data) = API_CLIENT.list_groups().await {
            groups.set(data);
        }

        if let Ok(channels) = API_CLIENT.list_channels().await {
            all_channels.set(
                channels
                    .into_iter()
                    .map(|c| Channel {
                        id: c.id,
                        name: c.name,
                    })
                    .collect(),
            );
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
                let new_group = GroupDto {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: value["name"].as_str().unwrap_or("").to_string(),
                    strategy: value["strategy"]
                        .as_str()
                        .unwrap_or("round_robin")
                        .to_string(),
                    match_path: value["match_path"].as_str().unwrap_or("").to_string(),
                };

                if API_CLIENT.create_group(&new_group).await.is_ok() {
                    fetch_data().await;
                    form_data.set(serde_json::json!({
                        "name": "",
                        "strategy": "round_robin",
                        "match_path": "/v1/chat/completions"
                    }));
                }
            });
        }
    };

    let delete_group = move |id: String| async move {
        if API_CLIENT.delete_group(&id).await.is_ok() {
            fetch_data().await;
            if selected_group_id() == Some(id) {
                selected_group_id.set(None);
            }
        }
    };

    let select_group = move |id: String| async move {
        selected_group_id.set(Some(id.clone()));
        if let Ok(members) = API_CLIENT.get_group_members(&id).await {
            selected_group_members.set(members);
        }
    };

    let mut add_member = move |upstream_id: String| {
        if selected_group_id().is_some() {
            let mut members = selected_group_members();
            if !members.iter().any(|m| m.upstream_id == upstream_id) {
                members.push(GroupMemberDto {
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
            let members: Vec<GroupMemberDto> = selected_group_members()
                .iter()
                .map(|m| GroupMemberDto {
                    upstream_id: m.upstream_id.clone(),
                    weight: m.weight,
                })
                .collect();

            if API_CLIENT.update_group_members(&gid, &members).await.is_ok() {
                // success toast?
            }
        }
    };

    rsx! {
        div { class: "flex flex-col gap-lg",
            // Create
            div { class: "card flat p-xl",
                div { class: "section-h lg mb-md",
                    div { class: "lead",
                        span { class: "lead-title", {t(*lang.read(), "settings.groups.new_group")} }
                    }
                }
                SchemaForm {
                    schema: schema.clone(),
                    data: form_data,
                    mode: FormMode::Create,
                    show_actions: false,
                    class: "flex flex-row gap-md items-end",
                    on_submit: handle_create,
                }
            }

            div { class: "grid gap-lg groups-layout",
                // Group List
                div { class: "card flat h-full p-xl",
                    div { class: "section-h lg mb-md",
                        div { class: "lead",
                            span { class: "lead-title", {t(*lang.read(), "settings.groups.group_list")} }
                        }
                    }
                        div { class: "flex flex-col gap-sm",
                            {groups().iter().map(|group| {
                                let gid1 = group.id.clone();
                                let gid2 = group.id.clone();
                                rsx! {
                                    div {
                                        class: if selected_group_id() == Some(group.id.clone()) {{ "p-sm cursor-pointer group-item-selected" }} else {{ "p-sm cursor-pointer group-item-default" }},
                                        onclick: move |_| select_group(gid1.clone()),
                                        div { class: "flex justify-between items-center",
                                            span { class: "font-medium", "{group.name}" }
                                            button { class: "btn btn-subtle btn-danger-sm",
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

                // Member Editor
                div { class: "card flat p-xl",
                    if let Some(gid) = selected_group_id() {
                        div { class: "section-h lg mb-lg",
                            div { class: "lead",
                                span { class: "lead-title", {t(*lang.read(), "settings.groups.edit_members")} }
                                span { class: "lead-sub mono", "{gid}" }
                            }
                        }

                        // Current Members
                        div { class: "mb-lg",
                            h4 { class: "text-caption font-bold text-secondary mb-sm", {t(*lang.read(), "settings.groups.current_members")} }
                            div { class: "flex flex-col gap-sm",
                                {selected_group_members().iter().map(|member| {
                                    let uid1 = member.upstream_id.clone();
                                    let uid2 = member.upstream_id.clone();
                                    rsx! {
                                        div { class: "flex items-center justify-between p-sm member-row",
                                            span {
                                                if let Some(ch) = all_channels().iter().find(|c| c.id == member.upstream_id) {
                                                    "{ch.name}"
                                                } else {
                                                    "{member.upstream_id}"
                                                }
                                            }
                                            div { class: "flex gap-sm items-center",
                                                span { class: "text-caption", {t(*lang.read(), "settings.groups.weight")} }
                                                input { class: "input weight-input",
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
                                                button { class: "btn btn-subtle btn-danger-sm",
                                                    onclick: move |_| remove_member(uid2.clone()),
                                                    "✕"
                                                }
                                            }
                                        }
                                    }
                                })}
                                if selected_group_members().is_empty() {
                                    div { class: "text-secondary text-center p-md italic",
                                        {t(*lang.read(), "settings.groups.no_members")}
                                    }
                                }
                            }
                        }

                        // Add Member
                        div { class: "mb-lg",
                            h4 { class: "text-caption font-bold text-secondary mb-sm", {t(*lang.read(), "settings.groups.add_member")} }
                            div { class: "flex gap-sm add-member-wrap",
                                for channel in all_channels() {
                                    if !selected_group_members().iter().any(|m| m.upstream_id == channel.id) {
                                        button { class: "btn btn-secondary add-member-btn",
                                            onclick: move |_| add_member(channel.id.clone()),
                                            "+ {channel.name}"
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "flex justify-end",
                            button { class: "btn btn-primary", onclick: save_members, {t(*lang.read(), "settings.groups.save_changes")} }
                        }
                    } else {
                        div { class: "flex items-center justify-center h-full text-secondary", {t(*lang.read(), "settings.groups.select_group_hint")} }
                    }
                }
            }
        }
    }
}