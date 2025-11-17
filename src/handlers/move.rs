use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use crate::db::exit::ExitRepository;
use crate::models::exit::is_valid_direction;
use std::sync::Arc;
use anyhow::Result;

pub async fn handle_move(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let current_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    // Parse direction from args
    let direction = args.trim().to_lowercase();

    if direction.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud move <direction>`\nExample: `/mud move north`\nValid directions: north, south, east, west, up, down"
        ).await?;
        return Ok(());
    }

    // Validate direction
    if !is_valid_direction(&direction) {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("Invalid direction: `{}`. Valid directions: north, south, east, west, up, down", direction)
        ).await?;
        return Ok(());
    }

    // Check if exit exists in that direction
    let exit = match exit_repo.get_exit_in_direction(&current_room_id, &direction).await? {
        Some(exit) => exit,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("There is no exit to the {} from here.", direction)
            ).await?;
            return Ok(());
        }
    };

    // Get current and destination room info
    let current_room = room_repo.get_by_channel_id(&current_room_id).await?;
    let current_room_name = current_room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("unknown");

    let destination_room = room_repo.get_by_channel_id(&exit.to_room_id).await?;
    let destination_room_name = destination_room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("unknown");

    // Post departure message in current room (broadcasts to channel and players in room via DM)
    let departure_third_person = format!("_{} heads {}._", player.name, direction);
    let departure_first_person = format!("_You head {}._", direction);
    super::broadcast_room_action(
        &state,
        &current_room_id,
        &departure_third_person,
        Some(&command.user_id),
        Some(&departure_first_person),
    ).await?;

    // Update player's current room
    player_repo.update_current_channel(&player.slack_user_id, &exit.to_room_id).await?;

    // Post arrival message in new room (broadcasts to channel and players in room via DM)
    let arrival_third_person = format!("_{} arrives._", player.name);
    let arrival_first_person = "_You arrive._";
    super::broadcast_room_action(
        &state,
        &exit.to_room_id,
        &arrival_third_person,
        Some(&command.user_id),
        Some(arrival_first_person),
    ).await?;

    // Send DM confirmation
    state.slack_client.send_dm(
        &command.user_id,
        &format!("You travel {} from #{} to #{}.", direction, current_room_name, destination_room_name)
    ).await?;

    // Automatically show the new room description
    super::look::handle_look(state, command).await?;

    Ok(())
}

pub async fn handle_move_dm(
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

    // Check if player has a current room
    let current_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    // Parse direction from args
    let direction = args.trim().to_lowercase();

    if direction.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `move <direction>`\nExample: `move north`\nValid directions: north, south, east, west, up, down"
        ).await?;
        return Ok(());
    }

    // Validate direction
    if !is_valid_direction(&direction) {
        state.slack_client.send_dm(
            &user_id,
            &format!("Invalid direction: `{}`. Valid directions: north, south, east, west, up, down", direction)
        ).await?;
        return Ok(());
    }

    // Check if exit exists in that direction
    let exit = match exit_repo.get_exit_in_direction(&current_room_id, &direction).await? {
        Some(exit) => exit,
        None => {
            state.slack_client.send_dm(
                &user_id,
                &format!("There is no exit to the {} from here.", direction)
            ).await?;
            return Ok(());
        }
    };

    // Get current and destination room info
    let current_room = room_repo.get_by_channel_id(&current_room_id).await?;
    let current_room_name = current_room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("unknown");

    let destination_room = room_repo.get_by_channel_id(&exit.to_room_id).await?;
    let destination_room_name = destination_room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("unknown");

    // Post departure message in current room (broadcasts to channel and players in room via DM)
    let departure_third_person = format!("_{} heads {}._", player.name, direction);
    let departure_first_person = format!("_You head {}._", direction);
    super::broadcast_room_action(
        &state,
        &current_room_id,
        &departure_third_person,
        Some(&user_id),
        Some(&departure_first_person),
    ).await?;

    // Update player's current room
    player_repo.update_current_channel(&player.slack_user_id, &exit.to_room_id).await?;

    // Post arrival message in new room (broadcasts to channel and players in room via DM)
    let arrival_third_person = format!("_{} arrives._", player.name);
    let arrival_first_person = "_You arrive._";
    super::broadcast_room_action(
        &state,
        &exit.to_room_id,
        &arrival_third_person,
        Some(&user_id),
        Some(arrival_first_person),
    ).await?;

    // Send DM confirmation
    state.slack_client.send_dm(
        &user_id,
        &format!("You travel {} from #{} to #{}.", direction, current_room_name, destination_room_name)
    ).await?;

    // Automatically show the new room description
    let dm_channel = String::new(); // Not used in handle_look_dm
    super::look::handle_look_dm(state, user_id, player.name, dm_channel).await?;

    Ok(())
}
