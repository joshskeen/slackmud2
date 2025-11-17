use crate::AppState;
use crate::slack::{SlashCommand, Block};
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use crate::models::Player;
use std::sync::Arc;
use anyhow::Result;

/// Handle look command from slash command (may move player to new room)
pub async fn handle_look(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get or create the player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let mut player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Using /mud look in a channel moves you to that room
    if player.current_channel_id.as_deref() != Some(&command.channel_id) {
        player_repo.update_current_channel(&player.slack_user_id, &command.channel_id).await?;
        player.current_channel_id = Some(command.channel_id.clone());
    }

    // Get or create the room
    let room = room_repo.get_or_create(
        command.channel_id.clone(),
        command.channel_name.clone(),
    ).await?;

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
        &player.slack_user_id,
    ).await?;

    // Post public action to the player's current room
    let public_text = format!("_{} looks around the room carefully._", player.name);
    state.slack_client.post_message(&room.channel_id, &public_text, None).await?;

    Ok(())
}

/// Handle look command from DM (uses player's current room)
pub async fn handle_look_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    _dm_channel: String,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get or create the player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player has a current room
    let channel_id = match player.current_channel_id {
        Some(id) => id,
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
        &player.slack_user_id,
    ).await?;

    // Post public action to the player's current room
    let public_text = format!("_{} looks around the room carefully._", player.name);
    state.slack_client.post_message(&channel_id, &public_text, None).await?;

    Ok(())
}

/// Helper function to send room description with player list
async fn send_room_description(
    state: Arc<AppState>,
    user_id: &str,
    room_name: &str,
    room_description: &str,
    room_channel_id: &str,
    players_in_room: &[Player],
    current_player_id: &str,
) -> Result<()> {
    let mut blocks = vec![
        Block::section(&format!("*You look around #{}*", room_name)),
        Block::section(room_description),
    ];

    // Add players in room section
    if !players_in_room.is_empty() {
        let mut players_text = String::from("*Players here:*\n");
        for player in players_in_room {
            if player.slack_user_id == current_player_id {
                players_text.push_str(&format!("• {} (you)\n", player.name));
            } else {
                players_text.push_str(&format!("• {}\n", player.name));
            }
        }
        blocks.push(Block::section(&players_text));
    } else {
        blocks.push(Block::section("*Players here:*\n_You are alone._"));
    }

    let dm_text = format!("You look around #{}", room_name);
    state.slack_client.send_dm_with_blocks(user_id, &dm_text, blocks).await?;

    Ok(())
}
