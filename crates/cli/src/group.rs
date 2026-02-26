use anyhow::Result;
use burncloud_database::Database;
use burncloud_database_router::{DbGroup, DbGroupMember, RouterDatabase};
use clap::ArgMatches;
use serde::Serialize;
use std::io::{self, Write};
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

/// Handle group show command
pub async fn cmd_group_show(db: &Database, matches: &ArgMatches) -> Result<()> {
    let id = matches.get_one::<String>("id").unwrap();

    // Fetch the group
    let group = RouterDatabase::get_group_by_id(db, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Group not found: {}", id))?;

    // Fetch group members
    let members = RouterDatabase::get_group_members_by_group(db, id).await?;

    // Output group details
    println!("Group Details:");
    println!("  ID:          {}", group.id);
    println!("  Name:        {}", group.name);
    println!("  Strategy:    {}", group.strategy);
    println!("  Match Path:  {}", group.match_path);
    println!();

    // Output members
    if members.is_empty() {
        println!("Members: (none)");
    } else {
        println!("Members:");
        println!(
            "{:<5} {:<40} {:<10}",
            "#", "Upstream ID", "Weight"
        );
        println!("{}", "-".repeat(55));
        for (i, member) in members.iter().enumerate() {
            println!(
                "{:<5} {:<40} {:<10}",
                i + 1,
                member.upstream_id,
                member.weight
            );
        }
    }

    Ok(())
}

/// Handle group delete command
pub async fn cmd_group_delete(db: &Database, matches: &ArgMatches) -> Result<()> {
    let id = matches.get_one::<String>("id").unwrap();
    let skip_confirm = matches.get_flag("yes");

    // Check if group exists
    let group = RouterDatabase::get_group_by_id(db, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Group not found: {}", id))?;

    // Confirm deletion
    if !skip_confirm {
        print!("Delete group '{}' (ID: {})? [y/N] ", group.name, id);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Operation cancelled");
            return Ok(());
        }
    }

    // Delete group (and its members)
    RouterDatabase::delete_group(db, id).await?;
    println!("Group '{}' (ID: {}) deleted", group.name, id);

    Ok(())
}

/// Handle group members command
pub async fn cmd_group_members(db: &Database, matches: &ArgMatches) -> Result<()> {
    let id = matches.get_one::<String>("id").unwrap();
    let set_str = matches.get_one::<String>("set").cloned();

    // Check if group exists
    let group = RouterDatabase::get_group_by_id(db, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Group not found: {}", id))?;

    match set_str {
        Some(members_str) => {
            // Set mode: update group members
            let member_specs: Vec<&str> = members_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if member_specs.is_empty() {
                // Clear all members
                RouterDatabase::set_group_members(db, id, vec![]).await?;
                println!("Cleared all members from group '{}'", group.name);
            } else {
                // Parse member specs (format: upstream_id:weight or upstream_id)
                let group_members: Vec<DbGroupMember> = member_specs
                    .iter()
                    .map(|spec| {
                        let parts: Vec<&str> = spec.split(':').collect();
                        let upstream_id = parts[0].to_string();
                        let weight = if parts.len() > 1 {
                            parts[1].parse::<i32>().unwrap_or(1)
                        } else {
                            1
                        };
                        DbGroupMember {
                            group_id: id.to_string(),
                            upstream_id,
                            weight,
                        }
                    })
                    .collect();

                let count = group_members.len();
                RouterDatabase::set_group_members(db, id, group_members).await?;
                println!(
                    "Set {} member(s) for group '{}'",
                    count, group.name
                );
            }
        }
        None => {
            // Query mode: display current members
            let members = RouterDatabase::get_group_members_by_group(db, id).await?;

            if members.is_empty() {
                println!("Group '{}' has no members", group.name);
            } else {
                println!("Members of group '{}':", group.name);
                println!(
                    "{:<5} {:<40} {:<10}",
                    "#", "Upstream ID", "Weight"
                );
                println!("{}", "-".repeat(55));
                for (i, member) in members.iter().enumerate() {
                    println!(
                        "{:<5} {:<40} {:<10}",
                        i + 1,
                        member.upstream_id,
                        member.weight
                    );
                }
            }
        }
    }

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
        Some(("show", sub_m)) => {
            cmd_group_show(db, sub_m).await?;
        }
        Some(("delete", sub_m)) => {
            cmd_group_delete(db, sub_m).await?;
        }
        Some(("members", sub_m)) => {
            cmd_group_members(db, sub_m).await?;
        }
        _ => {
            println!("Usage: burncloud group <create|list|show|delete|members>");
            println!("Run 'burncloud group --help' for more information.");
        }
    }

    Ok(())
}
