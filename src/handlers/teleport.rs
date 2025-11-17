use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use std::sync::Arc;
use anyhow::Result;

const WIZARD_LEVEL: i32 = 50;

pub async fn handle_teleport(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to use teleport.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    let args = args.trim();
    if args.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage:\n• `/mud teleport <vnum>` - Teleport yourself to a room\n• `/mud teleport <player_name> <vnum>` - Teleport another player to a room\n\nExample: `/mud teleport 3001`"
        ).await?;
        return Ok(());
    }

    // Parse arguments
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() == 1 {
        // Self teleport: teleport <vnum>
        let vnum = parts[0];
        teleport_player(
            state,
            &command.user_id,
            &player.slack_user_id,
            &player.name,
            vnum,
        ).await
    } else if parts.len() == 2 {
        // Teleport other player: teleport <player_name> <vnum>
        let target_name = parts[0];
        let vnum = parts[1];

        // Find the target player by name (case-insensitive)
        let all_players = sqlx::query_as::<_, crate::models::Player>(
            "SELECT * FROM players WHERE LOWER(name) = LOWER($1)"
        )
        .bind(target_name)
        .fetch_optional(&state.db_pool)
        .await?;

        if let Some(target_player) = all_players {
            teleport_player(
                state.clone(),
                &command.user_id,
                &target_player.slack_user_id,
                &target_player.name,
                vnum,
            ).await?;

            // Notify the wizard
            state.slack_client.send_dm(
                &command.user_id,
                &format!("✅ Teleported *{}* to room `{}`", target_player.name, vnum)
            ).await?;

            Ok(())
        } else {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("❌ Player '{}' not found.", target_name)
            ).await?;
            Ok(())
        }
    } else {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage:\n• `/mud teleport <vnum>` - Teleport yourself to a room\n• `/mud teleport <player_name> <vnum>` - Teleport another player to a room"
        ).await?;
        Ok(())
    }
}

pub async fn handle_teleport_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &user_id,
            &format!("You must be a wizard (level {}) to use teleport.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    let args = args.trim();
    if args.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage:\n• `teleport <vnum>` - Teleport yourself to a room\n• `teleport <player_name> <vnum>` - Teleport another player to a room\n\nExample: `teleport 3001`"
        ).await?;
        return Ok(());
    }

    // Parse arguments
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() == 1 {
        // Self teleport: teleport <vnum>
        let vnum = parts[0];
        teleport_player(
            state,
            &user_id,
            &player.slack_user_id,
            &player.name,
            vnum,
        ).await
    } else if parts.len() == 2 {
        // Teleport other player: teleport <player_name> <vnum>
        let target_name = parts[0];
        let vnum = parts[1];

        // Find the target player by name (case-insensitive)
        let all_players = sqlx::query_as::<_, crate::models::Player>(
            "SELECT * FROM players WHERE LOWER(name) = LOWER($1)"
        )
        .bind(target_name)
        .fetch_optional(&state.db_pool)
        .await?;

        if let Some(target_player) = all_players {
            teleport_player(
                state.clone(),
                &user_id,
                &target_player.slack_user_id,
                &target_player.name,
                vnum,
            ).await?;

            // Notify the wizard
            state.slack_client.send_dm(
                &user_id,
                &format!("✅ Teleported *{}* to room `{}`", target_player.name, vnum)
            ).await?;

            Ok(())
        } else {
            state.slack_client.send_dm(
                &user_id,
                &format!("❌ Player '{}' not found.", target_name)
            ).await?;
            Ok(())
        }
    } else {
        state.slack_client.send_dm(
            &user_id,
            "Usage:\n• `teleport <vnum>` - Teleport yourself to a room\n• `teleport <player_name> <vnum>` - Teleport another player to a room"
        ).await?;
        Ok(())
    }
}

/// Core teleport logic - teleports a player to a vnum
async fn teleport_player(
    state: Arc<AppState>,
    requesting_user_id: &str,
    target_slack_id: &str,
    target_name: &str,
    vnum: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Construct room_id from vnum
    let room_id = if vnum.starts_with("vnum_") {
        vnum.to_string()
    } else {
        format!("vnum_{}", vnum)
    };

    // Check if room exists
    if room_repo.get_by_channel_id(&room_id).await?.is_none() {
        state.slack_client.send_dm(
            requesting_user_id,
            &format!("❌ Room with vnum `{}` does not exist.", vnum)
        ).await?;
        return Ok(());
    }

    // Update player's location
    player_repo.update_current_channel(&target_slack_id, &room_id).await?;

    // Get the room details
    let room = room_repo.get_by_channel_id(&room_id).await?.unwrap();

    // Notify the teleported player
    let message = format!(
        "✨ *You have been teleported!*\n\n*{}*\n{}",
        room.channel_name,
        room.description
    );
    state.slack_client.send_dm(target_slack_id, &message).await?;

    // Broadcast to the room (if requesting user is teleporting themselves)
    if requesting_user_id == target_slack_id {
        let broadcast_msg = format!("✨ *{}* appears in a flash of light!", target_name);
        crate::handlers::broadcast_room_action(&state, &room_id, &broadcast_msg).await?;
    } else {
        // If wizard is teleporting someone else, broadcast to the destination room
        let broadcast_msg = format!("✨ *{}* appears in a flash of light!", target_name);
        crate::handlers::broadcast_room_action(&state, &room_id, &broadcast_msg).await?;
    }

    Ok(())
}
