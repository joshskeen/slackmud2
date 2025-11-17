use crate::AppState;
use crate::slack::{SlashCommand, Block};
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use std::sync::Arc;
use anyhow::Result;

pub async fn handle_look(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get or create the player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let mut player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Update player's current channel
    if player.current_channel_id.as_deref() != Some(&command.channel_id) {
        player_repo.update_current_channel(&player.slack_user_id, &command.channel_id).await?;
        player.current_channel_id = Some(command.channel_id.clone());
    }

    // Get or create the room
    let room = room_repo.get_or_create(
        command.channel_id.clone(),
        command.channel_name.clone(),
    ).await?;

    // Send private DM with room description to the user
    let dm_text = format!(
        "*You look around #{}*\n\n{}",
        room.channel_name,
        room.description
    );

    let blocks = vec![
        Block::section(&format!("*You look around #{}*", room.channel_name)),
        Block::section(&room.description),
    ];

    state.slack_client.send_dm_with_blocks(&command.user_id, &dm_text, blocks).await?;

    // Post public message in the channel that the user looked around
    let public_text = format!("_{} looks around the room carefully._", player.name);
    state.slack_client.post_message(&command.channel_id, &public_text, None).await?;

    Ok(())
}
