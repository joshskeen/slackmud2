use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use crate::db::exit::ExitRepository;
use crate::models::{Exit, exit::is_valid_direction};
use std::sync::Arc;
use anyhow::Result;

const WIZARD_LEVEL: i32 = 50;

pub async fn handle_dig(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard (level 50+)
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to use the dig command. Your level: {}", WIZARD_LEVEL, player.level)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let from_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room to dig. Use `/mud look` in a channel first!"
            ).await?;
            return Ok(());
        }
    };

    // Parse args: "direction #channel"
    // Example: "north #some-channel"
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() != 2 {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud dig <direction> <target>`\nExamples:\n• `/mud dig north 3014` - link to virtual room\n• `/mud dig north #tavern` - link to Slack channel\nValid directions: north, south, east, west, up, down"
        ).await?;
        return Ok(());
    }

    let direction = parts[0].to_lowercase();
    let target_channel = parts[1];

    // Validate direction
    if !is_valid_direction(&direction) {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("Invalid direction: `{}`. Valid directions: north, south, east, west, up, down", direction)
        ).await?;
        return Ok(());
    }

    // Parse target: can be vnum, number, or #channel-name
    let to_room_id = if target_channel.starts_with("vnum_") {
        // User provided vnum_3014 format - link to virtual room
        target_channel.to_string()
    } else if target_channel.chars().all(|c| c.is_numeric()) {
        // User provided just a number like 3014 - treat as vnum
        format!("vnum_{}", target_channel)
    } else if target_channel.starts_with('#') {
        // User provided #channel-name - link to Slack channel
        target_channel.trim_start_matches('#').to_string()
    } else if target_channel.starts_with('C') || target_channel.starts_with('<') {
        // Direct channel ID or <#C12345|name> format
        target_channel.trim_start_matches('<').trim_end_matches('>').split('|').next().unwrap_or(target_channel).to_string()
    } else {
        state.slack_client.send_dm(
            &command.user_id,
            "Please specify the target as:\n• A vnum: `3014` or `vnum_3014`\n• A Slack channel: `#channel-name`"
        ).await?;
        return Ok(());
    };

    // Check if exit already exists
    if let Some(existing_exit) = exit_repo.get_exit_in_direction(&from_room_id, &direction).await? {
        let existing_room = room_repo.get_by_channel_id(&existing_exit.to_room_id).await?;
        let existing_room_name = existing_room.map(|r| r.channel_name).unwrap_or_else(|| existing_exit.to_room_id.clone());

        state.slack_client.send_dm(
            &command.user_id,
            &format!("An exit to the {} already exists, leading to #{}. Delete it first if you want to change it.", direction, existing_room_name)
        ).await?;
        return Ok(());
    }

    // Get or create the target room
    let to_room = if to_room_id.starts_with("vnum_") {
        // For virtual rooms, verify they exist (don't create)
        match room_repo.get_by_channel_id(&to_room_id).await? {
            Some(room) => room,
            None => {
                let vnum_display = to_room_id.strip_prefix("vnum_").unwrap_or(&to_room_id);
                state.slack_client.send_dm(
                    &command.user_id,
                    &format!("Virtual room `{}` does not exist. Use `/mud vnums` to see available rooms.", vnum_display)
                ).await?;
                return Ok(());
            }
        }
    } else {
        // For regular channels, create if needed
        room_repo.get_or_create(
            to_room_id.clone(),
            target_channel.trim_start_matches('#').to_string(),
        ).await?
    };

    // Create the exit
    let exit = Exit::new(from_room_id.clone(), direction.clone(), to_room.channel_id.clone(), Some(player.slack_user_id.clone()));
    exit_repo.create(&exit).await?;

    // Get current room info
    let from_room = room_repo.get_by_channel_id(&from_room_id).await?;
    let from_room_name = from_room.map(|r| r.channel_name).unwrap_or_else(|| from_room_id.clone());

    // Send success message
    state.slack_client.send_dm(
        &command.user_id,
        &format!("✨ You dig an exit to the *{}* from #{}, leading to #{}!", direction, from_room_name, to_room.channel_name)
    ).await?;

    // Post public action (broadcasts to channel and players in room via DM)
    let third_person_text = format!("_{} utters some strange words. An exit to the {} flashes into existence!_", player.name, direction);
    let first_person_text = format!("_You utter some strange words. An exit to the {} flashes into existence!_", direction);
    super::broadcast_room_action(
        &state,
        &from_room_id,
        &third_person_text,
        Some(&command.user_id),
        Some(&first_person_text),
    ).await?;

    Ok(())
}

pub async fn handle_dig_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &user_id,
            &format!("You must be a wizard (level {}) to use the dig command. Your level: {}", WIZARD_LEVEL, player.level)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let from_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room to dig. Use `/mud look` in a channel first!"
            ).await?;
            return Ok(());
        }
    };

    // Parse args (same logic as slash command version)
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() != 2 {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `dig <direction> <target>`\nExamples:\n• `dig north 3014` - link to virtual room\n• `dig north #tavern` - link to Slack channel\nValid directions: north, south, east, west, up, down"
        ).await?;
        return Ok(());
    }

    let direction = parts[0].to_lowercase();
    let target_channel = parts[1];

    // Validate direction
    if !is_valid_direction(&direction) {
        state.slack_client.send_dm(
            &user_id,
            &format!("Invalid direction: `{}`. Valid directions: north, south, east, west, up, down", direction)
        ).await?;
        return Ok(());
    }

    // Parse target: can be vnum, number, or #channel-name
    let to_room_id = if target_channel.starts_with("vnum_") {
        // User provided vnum_3014 format - link to virtual room
        target_channel.to_string()
    } else if target_channel.chars().all(|c| c.is_numeric()) {
        // User provided just a number like 3014 - treat as vnum
        format!("vnum_{}", target_channel)
    } else if target_channel.starts_with('#') {
        // User provided #channel-name - link to Slack channel
        target_channel.trim_start_matches('#').to_string()
    } else if target_channel.starts_with('C') || target_channel.starts_with('<') {
        // Direct channel ID or <#C12345|name> format
        target_channel.trim_start_matches('<').trim_end_matches('>').split('|').next().unwrap_or(target_channel).to_string()
    } else {
        state.slack_client.send_dm(
            &user_id,
            "Please specify the target as:\n• A vnum: `3014` or `vnum_3014`\n• A Slack channel: `#channel-name`"
        ).await?;
        return Ok(());
    };

    // Check if exit already exists
    if let Some(existing_exit) = exit_repo.get_exit_in_direction(&from_room_id, &direction).await? {
        let existing_room = room_repo.get_by_channel_id(&existing_exit.to_room_id).await?;
        let existing_room_name = existing_room.map(|r| r.channel_name).unwrap_or_else(|| existing_exit.to_room_id.clone());

        state.slack_client.send_dm(
            &user_id,
            &format!("An exit to the {} already exists, leading to #{}.", direction, existing_room_name)
        ).await?;
        return Ok(());
    }

    // Get or create the target room
    let to_room = if to_room_id.starts_with("vnum_") {
        // For virtual rooms, verify they exist (don't create)
        match room_repo.get_by_channel_id(&to_room_id).await? {
            Some(room) => room,
            None => {
                let vnum_display = to_room_id.strip_prefix("vnum_").unwrap_or(&to_room_id);
                state.slack_client.send_dm(
                    &user_id,
                    &format!("Virtual room `{}` does not exist. Use `vnums` to see available rooms.", vnum_display)
                ).await?;
                return Ok(());
            }
        }
    } else {
        // For regular channels, create if needed
        room_repo.get_or_create(
            to_room_id.clone(),
            target_channel.trim_start_matches('#').to_string(),
        ).await?
    };

    // Create the exit
    let exit = Exit::new(from_room_id.clone(), direction.clone(), to_room.channel_id.clone(), Some(player.slack_user_id.clone()));
    exit_repo.create(&exit).await?;

    // Get current room info
    let from_room = room_repo.get_by_channel_id(&from_room_id).await?;
    let from_room_name = from_room.map(|r| r.channel_name).unwrap_or_else(|| from_room_id.clone());

    // Send success message
    state.slack_client.send_dm(
        &user_id,
        &format!("✨ You dig an exit to the *{}* from #{}, leading to #{}!", direction, from_room_name, to_room.channel_name)
    ).await?;

    // Post public action (broadcasts to channel and players in room via DM)
    let third_person_text = format!("_{} utters some strange words. An exit to the {} flashes into existence!_", player.name, direction);
    let first_person_text = format!("_You utter some strange words. An exit to the {} flashes into existence!_", direction);
    super::broadcast_room_action(
        &state,
        &from_room_id,
        &third_person_text,
        Some(&user_id),
        Some(&first_person_text),
    ).await?;

    Ok(())
}
