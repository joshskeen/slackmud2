use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use std::sync::Arc;
use anyhow::Result;

/// Handle say command - broadcast to current room
pub async fn handle_say(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let room_id = match &player.current_channel_id {
        Some(id) => id.clone(),
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room first!"
            ).await?;
            return Ok(());
        }
    };

    let message = args.trim();
    if message.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Say what?"
        ).await?;
        return Ok(());
    }

    // Format messages with quotes around what was said
    let first_person = format!("You say '{}'", message);
    let third_person = format!("_{} says '{}'_", player.name, message);

    // Broadcast to room
    super::broadcast_room_action(
        &state,
        &room_id,
        &third_person,
        Some(&command.user_id),
        Some(&first_person),
    ).await?;

    Ok(())
}

/// Handle say command from DM
pub async fn handle_say_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player has a current room
    let room_id = match &player.current_channel_id {
        Some(id) => id.clone(),
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room first!"
            ).await?;
            return Ok(());
        }
    };

    let message = args.trim();
    if message.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Say what?"
        ).await?;
        return Ok(());
    }

    // Format messages with quotes
    let first_person = format!("You say '{}'", message);
    let third_person = format!("_{} says '{}'_", player.name, message);

    // Broadcast to room
    super::broadcast_room_action(
        &state,
        &room_id,
        &third_person,
        Some(&user_id),
        Some(&first_person),
    ).await?;

    Ok(())
}

/// Handle tell command - private message to another player
pub async fn handle_tell(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get sender
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let sender = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Parse args: "tell <player> <message>"
    let args = args.trim();
    if args.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Tell whom what?\nUsage: `/mud tell <player> <message>`"
        ).await?;
        return Ok(());
    }

    // Split on first whitespace to get target and message
    let (target_name, message) = if let Some(space_pos) = args.find(' ') {
        let (target, msg) = args.split_at(space_pos);
        (target.trim(), msg.trim())
    } else {
        state.slack_client.send_dm(
            &command.user_id,
            "Tell them what?\nUsage: `/mud tell <player> <message>`"
        ).await?;
        return Ok(());
    };

    if message.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Tell them what?\nUsage: `/mud tell <player> <message>`"
        ).await?;
        return Ok(());
    }

    // Find target player (anywhere in the game)
    let target = find_player_by_name(&state, target_name).await?;

    match target {
        Some(target_player) => {
            // Can't tell yourself
            if target_player.slack_user_id == sender.slack_user_id {
                state.slack_client.send_dm(
                    &command.user_id,
                    "You have a nice conversation with yourself."
                ).await?;
                return Ok(());
            }

            // Send to target
            let target_message = format!("_{} tells you '{}'_", sender.name, message);
            state.slack_client.send_dm(&target_player.slack_user_id, &target_message).await?;

            // Confirm to sender
            let sender_message = format!("You tell {} '{}'", target_player.name, message);
            state.slack_client.send_dm(&command.user_id, &sender_message).await?;
        }
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("No player named '{}' found.", target_name)
            ).await?;
        }
    }

    Ok(())
}

/// Handle tell command from DM
pub async fn handle_tell_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get sender
    let sender = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Parse args
    let args = args.trim();
    if args.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Tell whom what?\nUsage: `tell <player> <message>`"
        ).await?;
        return Ok(());
    }

    let (target_name, message) = if let Some(space_pos) = args.find(' ') {
        let (target, msg) = args.split_at(space_pos);
        (target.trim(), msg.trim())
    } else {
        state.slack_client.send_dm(
            &user_id,
            "Tell them what?\nUsage: `tell <player> <message>`"
        ).await?;
        return Ok(());
    };

    if message.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Tell them what?\nUsage: `tell <player> <message>`"
        ).await?;
        return Ok(());
    }

    // Find target player
    let target = find_player_by_name(&state, target_name).await?;

    match target {
        Some(target_player) => {
            // Can't tell yourself
            if target_player.slack_user_id == sender.slack_user_id {
                state.slack_client.send_dm(
                    &user_id,
                    "You have a nice conversation with yourself."
                ).await?;
                return Ok(());
            }

            // Send to target
            let target_message = format!("_{} tells you '{}'_", sender.name, message);
            state.slack_client.send_dm(&target_player.slack_user_id, &target_message).await?;

            // Confirm to sender
            let sender_message = format!("You tell {} '{}'", target_player.name, message);
            state.slack_client.send_dm(&user_id, &sender_message).await?;
        }
        None => {
            state.slack_client.send_dm(
                &user_id,
                &format!("No player named '{}' found.", target_name)
            ).await?;
        }
    }

    Ok(())
}

/// Handle shout command - broadcast to all players
pub async fn handle_shout(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room (need to be somewhere to shout)
    if player.current_channel_id.is_none() {
        state.slack_client.send_dm(
            &command.user_id,
            "You need to be in a room first!"
        ).await?;
        return Ok(());
    }

    let message = args.trim();
    if message.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Shout what?"
        ).await?;
        return Ok(());
    }

    // Get all players
    let all_players = player_repo.get_all_players().await?;

    // Send to all players (different messages for shouter vs others)
    let sender_message = format!("You shout '{}'", message);
    let broadcast_message = format!("_{} shouts '{}'_", player.name, message);

    for target_player in all_players {
        if target_player.slack_user_id == player.slack_user_id {
            // Send first-person message to shouter
            state.slack_client.send_dm(&target_player.slack_user_id, &sender_message).await?;
        } else {
            // Send third-person message to everyone else
            state.slack_client.send_dm(&target_player.slack_user_id, &broadcast_message).await?;
        }
    }

    Ok(())
}

/// Handle shout command from DM
pub async fn handle_shout_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player has a current room
    if player.current_channel_id.is_none() {
        state.slack_client.send_dm(
            &user_id,
            "You need to be in a room first!"
        ).await?;
        return Ok(());
    }

    let message = args.trim();
    if message.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Shout what?"
        ).await?;
        return Ok(());
    }

    // Get all players
    let all_players = player_repo.get_all_players().await?;

    // Send to all players
    let sender_message = format!("You shout '{}'", message);
    let broadcast_message = format!("_{} shouts '{}'_", player.name, message);

    for target_player in all_players {
        if target_player.slack_user_id == player.slack_user_id {
            state.slack_client.send_dm(&target_player.slack_user_id, &sender_message).await?;
        } else {
            state.slack_client.send_dm(&target_player.slack_user_id, &broadcast_message).await?;
        }
    }

    Ok(())
}

/// Find a player by name (case-insensitive, anywhere in the game)
async fn find_player_by_name(
    state: &Arc<AppState>,
    target_name: &str,
) -> Result<Option<crate::models::Player>> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let all_players = player_repo.get_all_players().await?;

    let target_lower = target_name.to_lowercase();
    for player in all_players {
        if player.name.to_lowercase() == target_lower {
            return Ok(Some(player));
        }
    }

    Ok(None)
}
