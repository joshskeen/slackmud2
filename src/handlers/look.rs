use crate::AppState;
use crate::slack::{SlashCommand, Block};
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use crate::db::exit::ExitRepository;
use crate::db::object::{ObjectRepository, ObjectInstanceRepository};
use crate::models::Player;
use std::sync::Arc;
use anyhow::Result;

/// Handle look command from slash command
pub async fn handle_look(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    // Parse command to check for argument (player name or object name)
    let (_, args) = command.parse_subcommand();
    let args = args.trim();

    // If there's an argument, try looking at a player first, then object
    if !args.is_empty() {
        // Try to look at a player
        if let Ok(_) = handle_look_at_player(state.clone(), &command.user_id, args).await {
            return Ok(());
        }

        // Fall back to looking at an object
        return handle_look_at_object(
            state,
            &command.user_id,
            args,
        ).await;
    }

    // Otherwise, look at the room
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get or create the player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let channel_id = match &player.current_channel_id {
        Some(id) => id.clone(),
        None => {
            // First time player - set their room to where they used the command
            player_repo.update_current_channel(&player.slack_user_id, &command.channel_id).await?;

            // Create the room if it doesn't exist
            room_repo.get_or_create(
                command.channel_id.clone(),
                command.channel_name.clone(),
            ).await?;

            state.slack_client.send_dm(
                &command.user_id,
                &format!("Welcome to SlackMUD! You have entered #{}.", command.channel_name)
            ).await?;

            command.channel_id
        }
    };

    // Get the room (player's current room)
    let room = match room_repo.get_by_channel_id(&channel_id).await? {
        Some(room) => room,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "Your current location is unknown. This shouldn't happen!"
            ).await?;
            return Ok(());
        }
    };

    // Get all players in this room
    let players_in_room = player_repo.get_players_in_room(&room.channel_id).await?;

    // Send room description to user
    send_room_description(
        state.clone(),
        &command.user_id,
        &room.channel_name,
        &room.description,
        &room.channel_id,
        &players_in_room,
        &player,
    ).await?;

    // Post public action to the player's current room (broadcasts to channel and players in room via DM)
    let third_person_text = format!("_{} looks around the room carefully._", player.name);
    let first_person_text = "_You look around the room carefully._";
    super::broadcast_room_action(
        &state,
        &room.channel_id,
        &third_person_text,
        Some(&command.user_id),
        Some(first_person_text),
    ).await?;

    Ok(())
}

/// Handle look command from DM (uses player's current room)
pub async fn handle_look_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let args = args.trim();

    // If there's an argument, try looking at a player first, then object
    if !args.is_empty() {
        // Try to look at a player
        if let Ok(_) = handle_look_at_player(state.clone(), &user_id, args).await {
            return Ok(());
        }

        // Fall back to looking at an object
        return handle_look_at_object(
            state,
            &user_id,
            args,
        ).await;
    }

    // Otherwise, look at the room
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get or create the player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player has a current room
    let channel_id = match &player.current_channel_id {
        Some(id) => id.clone(),
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You haven't entered any room yet! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    // Get the room
    let room = match room_repo.get_by_channel_id(&channel_id).await? {
        Some(room) => room,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "Your current location is unknown. Try using `/mud look` in a channel!"
            ).await?;
            return Ok(());
        }
    };

    // Get all players in this room
    let players_in_room = player_repo.get_players_in_room(&room.channel_id).await?;

    // Send room description to user
    send_room_description(
        state.clone(),
        &user_id,
        &room.channel_name,
        &room.description,
        &room.channel_id,
        &players_in_room,
        &player,
    ).await?;

    // Post public action to the player's current room (broadcasts to channel and players in room via DM)
    let third_person_text = format!("_{} looks around the room carefully._", player.name);
    let first_person_text = "_You look around the room carefully._";
    super::broadcast_room_action(
        &state,
        &channel_id,
        &third_person_text,
        Some(&user_id),
        Some(first_person_text),
    ).await?;

    Ok(())
}

/// Helper function to send room description with player list and exits
async fn send_room_description(
    state: Arc<AppState>,
    user_id: &str,
    room_name: &str,
    room_description: &str,
    room_channel_id: &str,
    players_in_room: &[Player],
    current_player: &Player,
) -> Result<()> {
    let exit_repo = ExitRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());

    // Get full room details to check for attached channel
    let room = room_repo.get_by_channel_id(room_channel_id).await?;

    // Build room title - show vnum and attached channel for wizards
    let room_title = if current_player.level >= 50 {
        // Extract vnum from channel_id (format: vnum_3014)
        if let Some(vnum) = room_channel_id.strip_prefix("vnum_") {
            // Check if room has an attached channel
            if let Some(ref room_data) = room {
                if let Some(ref attached_channel) = room_data.attached_channel_id {
                    format!("*You look around #{} [`{}` | attached to <#{}>]*", room_name, vnum, attached_channel)
                } else {
                    format!("*You look around #{} [`{}`]*", room_name, vnum)
                }
            } else {
                format!("*You look around #{} [`{}`]*", room_name, vnum)
            }
        } else {
            // Non-vnum room - check for attached channel
            if let Some(ref room_data) = room {
                if let Some(ref attached_channel) = room_data.attached_channel_id {
                    format!("*You look around #{} [non-vnum | attached to <#{}>]*", room_name, attached_channel)
                } else {
                    format!("*You look around #{} [non-vnum room]*", room_name)
                }
            } else {
                format!("*You look around #{} [non-vnum room]*", room_name)
            }
        }
    } else {
        format!("*You look around #{}*", room_name)
    };

    let mut blocks = vec![
        Block::section(&room_title),
        Block::section(room_description),
    ];

    // Add exits section
    let exits = exit_repo.get_exits_from_room(room_channel_id).await?;
    if !exits.is_empty() {
        let mut exits_text = String::from("*Exits:*\n");
        for exit in &exits {
            // Get target room name
            let target_room_name = if let Some(room) = room_repo.get_by_channel_id(&exit.to_room_id).await? {
                room.channel_name
            } else {
                exit.to_room_id.clone()
            };
            exits_text.push_str(&format!("• *{}* → #{}\n", exit.direction, target_room_name));
        }
        blocks.push(Block::section(&exits_text));
    }

    // Add players in room section
    if !players_in_room.is_empty() {
        let mut players_text = String::from("*Players here:*\n");
        for player in players_in_room {
            if player.slack_user_id == current_player.slack_user_id {
                players_text.push_str(&format!("• {} (you)\n", player.name));
            } else {
                players_text.push_str(&format!("• {}\n", player.name));
            }
        }
        blocks.push(Block::section(&players_text));
    } else {
        blocks.push(Block::section("*Players here:*\n_You are alone._"));
    }

    // Add objects in room section
    let object_instances = object_instance_repo.get_in_room(room_channel_id).await?;
    if !object_instances.is_empty() {
        let mut objects_text = String::from("*Items here:*\n");
        for instance in &object_instances {
            // Get the object definition
            if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
                objects_text.push_str(&format!("• {}\n", object.long_description));
            }
        }
        blocks.push(Block::section(&objects_text));
    }

    let dm_text = format!("You look around #{}", room_name);
    state.slack_client.send_dm_with_blocks(user_id, &dm_text, blocks).await?;

    Ok(())
}

/// Handle looking at a specific object
async fn handle_look_at_object(
    state: Arc<AppState>,
    user_id: &str,
    object_name: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(user_id).await?;
    let player = player_repo.get_or_create(user_id.to_string(), real_name).await?;

    // Check if player has a current room
    let room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    // Search for object in player's inventory first
    let inventory_instances = object_instance_repo.get_in_player_inventory(&player.slack_user_id).await?;
    for instance in &inventory_instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(object_name) {
                // Found in inventory
                send_object_description(&state, user_id, &object, "inventory").await?;
                return Ok(());
            }
        }
    }

    // Search for object in current room
    let room_instances = object_instance_repo.get_in_room(&room_id).await?;
    for instance in &room_instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(object_name) {
                // Found in room
                send_object_description(&state, user_id, &object, "room").await?;
                return Ok(());
            }
        }
    }

    // Not found
    state.slack_client.send_dm(
        user_id,
        &format!("You don't see '{}' here.", object_name)
    ).await?;

    Ok(())
}

/// Send detailed description of an object to the player
async fn send_object_description(
    state: &Arc<AppState>,
    user_id: &str,
    object: &crate::models::Object,
    location: &str,
) -> Result<()> {
    let location_text = match location {
        "inventory" => "You are carrying:",
        "room" => "You examine:",
        _ => "You see:",
    };

    let mut description = format!("*{}*\n", location_text);
    description.push_str(&format!("*{}*\n\n", object.short_description));
    description.push_str(&format!("{}\n\n", object.long_description));
    description.push_str(&format!("*Item Type:* {}\n", object.item_type));
    description.push_str(&format!("*Material:* {}\n", object.material));
    description.push_str(&format!("*Weight:* {} lbs\n", object.weight));

    if object.level > 0 {
        description.push_str(&format!("*Level:* {}\n", object.level));
    }

    if object.cost > 0 {
        description.push_str(&format!("*Value:* {} gold\n", object.cost));
    }

    // Show extra flags if present
    if !object.extra_flags.is_empty() && object.extra_flags != "0" {
        description.push_str(&format!("*Flags:* {}\n", object.extra_flags));
    }

    // Show wear locations if it can be worn
    if !object.wear_flags.is_empty() && object.wear_flags != "0" {
        description.push_str(&format!("*Can be worn:* {}\n", object.wear_flags));
    }

    state.slack_client.send_dm(user_id, &description).await?;

    Ok(())
}

/// Handle looking at another player in the same room
async fn handle_look_at_player(
    state: Arc<AppState>,
    viewer_id: &str,
    target_name: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get viewer
    let viewer_real_name = state.slack_client.get_user_real_name(viewer_id).await?;
    let viewer = player_repo.get_or_create(viewer_id.to_string(), viewer_real_name).await?;

    // Check if viewer has a current room
    let viewer_room = match viewer.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                viewer_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    // Find target player by partial name match in same room
    let players_in_room = player_repo.get_players_in_room(&viewer_room).await?;
    let target = players_in_room.iter()
        .find(|p| p.name.to_lowercase().contains(&target_name.to_lowercase()))
        .ok_or_else(|| anyhow::anyhow!("You don't see {} here.", target_name))?;

    // Get target's class, race, and gender names
    use crate::db::class::ClassRepository;
    use crate::db::race::RaceRepository;

    let class_repo = ClassRepository::new(state.db_pool.clone());
    let race_repo = RaceRepository::new(state.db_pool.clone());

    let class_name = if let Some(class_id) = target.class_id {
        class_repo.get_by_id(class_id).await?
            .map(|c| c.name)
            .unwrap_or_else(|| "Unknown".to_string())
    } else {
        "Unknown".to_string()
    };

    let race_name = if let Some(race_id) = target.race_id {
        race_repo.get_by_id(race_id).await?
            .map(|r| r.name)
            .unwrap_or_else(|| "Unknown".to_string())
    } else {
        "Unknown".to_string()
    };

    let gender_name = target.gender.as_deref().unwrap_or("neutral");

    // Build player description
    let mut description = String::new();

    // Header with name, race, class, level
    description.push_str(&format!(
        "*{}* is a {} {} {}, level {}.\n\n",
        target.name,
        gender_name,
        race_name.to_lowercase(),
        class_name.to_lowercase(),
        target.level
    ));

    // Health status (could be enhanced with actual health tracking)
    description.push_str(&format!("{} is in excellent condition.\n\n", target.name));

    // Get all equipped items
    let equipped_instances = object_instance_repo.get_equipped(&target.slack_user_id).await?;

    if !equipped_instances.is_empty() {
        use crate::models::EquipmentSlot;

        description.push_str(&format!("*{} is using:*\n", target.name));

        // Display in slot order
        for slot in EquipmentSlot::all_slots_in_order() {
            let slot_str = slot.to_db_string();

            // Find item in this slot
            if let Some(instance) = equipped_instances.iter().find(|i| {
                i.equipped_slot.as_ref().map(|s| s.as_str()) == Some(slot_str)
            }) {
                if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
                    description.push_str(&format!(
                        "{:<20} {}\n",
                        slot.display_label(),
                        object.short_description
                    ));
                }
            }
        }
        description.push_str("\n");
    } else {
        description.push_str(&format!("{} isn't wearing any equipment.\n\n", target.name));
    }

    // Show inventory (items carried but not equipped)
    let inventory_instances = object_instance_repo
        .get_in_player_inventory(&target.slack_user_id).await?;

    if !inventory_instances.is_empty() {
        description.push_str(&format!("*{} is carrying:*\n", target.name));
        for instance in &inventory_instances {
            if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
                description.push_str(&format!("• {}\n", object.short_description));
            }
        }
    }

    // Send to viewer
    state.slack_client.send_dm(viewer_id, &description).await?;

    Ok(())
}
