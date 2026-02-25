use anyhow::Result;
use burncloud_database::Database;
use burncloud_database_router::{DbGroup, DbGroupMember, RouterDatabase};
use clap::ArgMatches;
use uuid::Uuid;

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
        _ => {
            println!("Usage: burncloud group <create>");
            println!("Run 'burncloud group --help' for more information.");
        }
    }

    Ok(())
}
