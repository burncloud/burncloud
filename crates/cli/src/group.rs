use anyhow::Result;
use burncloud_database::Database;
use burncloud_database_router::{DbGroup, DbGroupMember, RouterDatabase};
use clap::ArgMatches;
use serde::Serialize;
use uuid::Uuid;

/// Group list item for JSON output
#[derive(Debug, Clone, Serialize)]
pub struct GroupListItem {
    pub id: String,
    pub name: String,
    pub member_count: usize,
}

/// Handle group list command
pub async fn cmd_group_list(db: &Database, matches: &ArgMatches) -> Result<()> {
    let format = matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("table");

    // Fetch all groups
    let groups = RouterDatabase::get_all_groups(db).await?;

    if groups.is_empty() {
        println!("No groups found");
        return Ok(());
    }

    // Build list with member counts
    let mut list_items: Vec<GroupListItem> = Vec::new();
    for group in groups {
        let members = RouterDatabase::get_group_members_by_group(db, &group.id).await?;
        list_items.push(GroupListItem {
            id: group.id,
            name: group.name,
            member_count: members.len(),
        });
    }

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&list_items)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!(
                "{:<40} {:<30} {:<15}",
                "ID", "Name", "Member Count"
            );
            println!("{}", "-".repeat(85));
            for item in &list_items {
                println!(
                    "{:<40} {:<30} {:<15}",
                    item.id, item.name, item.member_count
                );
            }
        }
    }

    Ok(())
}

/// Handle group create command
pub async fn cmd_group_create(db: &Database, matches: &ArgMatches) -> Result<()> {
    let name = matches.get_one::<String>("name").unwrap();
    let members_str = matches.get_one::<String>("members").cloned();

    // Generate a new UUID for the group
    let group_id = Uuid::new_v4().to_string();

    // Create the group with default strategy and match_path
    let group = DbGroup {
        id: group_id.clone(),
        name: name.clone(),
        strategy: "round_robin".to_string(),
        match_path: "/*".to_string(),
    };

    // Create the group in database
    RouterDatabase::create_group(db, &group).await?;

    // If members are provided, add them to the group
    if let Some(members) = members_str {
        let member_ids: Vec<&str> = members.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        if !member_ids.is_empty() {
            let group_members: Vec<DbGroupMember> = member_ids
                .iter()
                .map(|upstream_id| DbGroupMember {
                    group_id: group_id.clone(),
                    upstream_id: upstream_id.to_string(),
                    weight: 1,
                })
                .collect();

            RouterDatabase::set_group_members(db, &group_id, group_members).await?;

            println!("Group '{}' created successfully with {} member(s)!", name, member_ids.len());
        } else {
            println!("Group '{}' created successfully!", name);
        }
    } else {
        println!("Group '{}' created successfully!", name);
    }

    println!("Group ID: {}", group_id);

    Ok(())
}

/// Route group commands
pub async fn handle_group_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("create", sub_m)) => {
            cmd_group_create(db, sub_m).await?;
        }
        Some(("list", sub_m)) => {
            cmd_group_list(db, sub_m).await?;
        }
        _ => {
            println!("Usage: burncloud group <create|list>");
            println!("Run 'burncloud group --help' for more information.");
        }
    }

    Ok(())
}
