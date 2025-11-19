use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::social;
use std::sync::Arc;
use anyhow::Result;

/// Handle a social command (e.g., smile, laugh, kiss, etc.)
pub async fn handle_social(
    state: Arc<AppState>,
    command: SlashCommand,
    social_name: &str,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get the social definition
    let social_cmd = match social::get_social(social_name) {
        Some(s) => s,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("Unknown social command: {}", social_name)
            ).await?;
            return Ok(());
        }
    };

    // Get the actor (player executing the command)
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let actor = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let room_id = match &actor.current_channel_id {
        Some(id) => id.clone(),
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    let target_name = args.trim();

    if target_name.is_empty() {
        // No target - solo social
        let actor_msg = social_cmd.messages.get_actor_message(&actor, None);
        let room_msg = social_cmd.messages.get_room_message(&actor, None);

        // Send message to actor
        if !actor_msg.is_empty() {
            state.slack_client.send_dm(&command.user_id, &actor_msg).await?;
        }

        // Broadcast to room
        if !room_msg.is_empty() {
            super::broadcast_room_action(
                &state,
                &room_id,
                &room_msg,
                Some(&command.user_id),
                Some(&actor_msg),
            ).await?;
        }
    } else {
        // Has target - find the target player
        let target = find_player_in_room(&state, &room_id, target_name).await?;

        match target {
            Some(target_player) => {
                let actor_msg = social_cmd.messages.get_actor_message(&actor, Some(&target_player));
                let target_msg = social_cmd.messages.get_target_message(&actor, &target_player);
                let room_msg = social_cmd.messages.get_room_message(&actor, Some(&target_player));

                // Send message to actor
                if !actor_msg.is_empty() {
                    state.slack_client.send_dm(&command.user_id, &actor_msg).await?;
                }

                // Send message to target (if not targeting self)
                if target_player.slack_user_id != actor.slack_user_id && !target_msg.is_empty() {
                    state.slack_client.send_dm(&target_player.slack_user_id, &target_msg).await?;
                }

                // Broadcast to room
                if !room_msg.is_empty() {
                    super::broadcast_room_action(
                        &state,
                        &room_id,
                        &room_msg,
                        Some(&command.user_id),
                        Some(&actor_msg),
                    ).await?;
                }
            }
            None => {
                // Target not found
                let not_found_msg = social_cmd.messages.char_not_found.clone();
                if !not_found_msg.is_empty() {
                    state.slack_client.send_dm(&command.user_id, &not_found_msg).await?;
                } else {
                    state.slack_client.send_dm(
                        &command.user_id,
                        &format!("You don't see '{}' here.", target_name)
                    ).await?;
                }
            }
        }
    }

    Ok(())
}

/// Find a player in the same room by name (case-insensitive)
async fn find_player_in_room(
    state: &Arc<AppState>,
    room_id: &str,
    target_name: &str,
) -> Result<Option<crate::models::Player>> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let players = player_repo.get_players_in_room(room_id).await?;

    let target_lower = target_name.to_lowercase();
    for player in players {
        if player.name.to_lowercase() == target_lower {
            return Ok(Some(player));
        }
    }

    Ok(None)
}
