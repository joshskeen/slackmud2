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
            "Usage: `/mud dig <direction> #channel`\nExample: `/mud dig north #tavern`\nValid directions: north, south, east, west, up, down"
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

    // Parse channel ID from #channel-name or C12345 format
    let to_room_id = if target_channel.starts_with('#') {
        // User provided #channel-name, we need to create/get the room
        // For now, we'll use the channel name as ID (Slack will validate)
        target_channel.trim_start_matches('#').to_string()
    } else if target_channel.starts_with('C') || target_channel.starts_with('<') {
        // Direct channel ID or <#C12345|name> format
        target_channel.trim_start_matches('<').trim_end_matches('>').split('|').next().unwrap_or(target_channel).to_string()
    } else {
        state.slack_client.send_dm(
            &command.user_id,
            "Please specify the target channel as #channel-name or channel ID"
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

    // Create or get the target room
    let to_room = room_repo.get_or_create(
        to_room_id.clone(),
        target_channel.trim_start_matches('#').to_string(),
    ).await?;

    // Create the exit
    let exit = Exit::new(from_room_id.clone(), direction.clone(), to_room.channel_id.clone(), player.slack_user_id.clone());
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
    let public_text = format!("_{} utters some strange words. An exit to the {} flashes into existence!_", player.name, direction);
    super::broadcast_room_action(&state, &from_room_id, &public_text).await?;

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
            "Usage: `dig <direction> #channel`\nExample: `dig north #tavern`\nValid directions: north, south, east, west, up, down"
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

    // Parse channel
    let to_room_id = if target_channel.starts_with('#') {
        target_channel.trim_start_matches('#').to_string()
    } else if target_channel.starts_with('C') || target_channel.starts_with('<') {
        target_channel.trim_start_matches('<').trim_end_matches('>').split('|').next().unwrap_or(target_channel).to_string()
    } else {
        state.slack_client.send_dm(
            &user_id,
            "Please specify the target channel as #channel-name"
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

    // Create or get the target room
    let to_room = room_repo.get_or_create(
        to_room_id.clone(),
        target_channel.trim_start_matches('#').to_string(),
    ).await?;

    // Create the exit
    let exit = Exit::new(from_room_id.clone(), direction.clone(), to_room.channel_id.clone(), player.slack_user_id.clone());
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
    let public_text = format!("_{} utters some strange words. An exit to the {} flashes into existence!_", player.name, direction);
    super::broadcast_room_action(&state, &from_room_id, &public_text).await?;

    Ok(())
}
