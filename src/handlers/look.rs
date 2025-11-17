use crate::AppState;
use crate::slack::{SlashCommand, Block};
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use crate::db::exit::ExitRepository;
use crate::models::Player;
use std::sync::Arc;
use anyhow::Result;

/// Handle look command from slash command
pub async fn handle_look(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get or create the player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let channel_id = match player.current_channel_id {
        Some(id) => id,
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
        &player.slack_user_id,
    ).await?;

    // Post public action to the player's current room (broadcasts to channel and players in room via DM)
    let public_text = format!("_{} looks around the room carefully._", player.name);
    super::broadcast_room_action(&state, &room.channel_id, &public_text).await?;

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

    // Post public action to the player's current room (broadcasts to channel and players in room via DM)
    let public_text = format!("_{} looks around the room carefully._", player.name);
    super::broadcast_room_action(&state, &channel_id, &public_text).await?;

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
    current_player_id: &str,
) -> Result<()> {
    let exit_repo = ExitRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    let mut blocks = vec![
        Block::section(&format!("*You look around #{}*", room_name)),
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
