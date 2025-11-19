use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use std::sync::Arc;
use anyhow::Result;

const WIZARD_LEVEL: i32 = 50;

pub async fn handle_attach(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to use the attach command.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let current_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room to attach it! Use `/mud look` in a channel first."
            ).await?;
            return Ok(());
        }
    };

    // Parse channel from args
    let channel_arg = args.trim();
    if channel_arg.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud attach #channel-name`\\nExample: `/mud attach #general`"
        ).await?;
        return Ok(());
    }

    // Parse the Slack channel ID
    let slack_channel_id = if channel_arg.starts_with('#') {
        channel_arg.trim_start_matches('#').to_string()
    } else if channel_arg.starts_with('<') {
        // Handle <#C12345|name> format
        channel_arg.trim_start_matches('<').trim_start_matches('#').split('|').next().unwrap_or(channel_arg).trim_end_matches('>').to_string()
    } else {
        channel_arg.to_string()
    };

    // Attach the room
    room_repo.attach_to_channel(&current_room_id, &slack_channel_id).await?;

    // Get room info for confirmation
    let room = room_repo.get_by_channel_id(&current_room_id).await?;
    let room_name = room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("current room");
    let room_description = room.as_ref().map(|r| r.description.as_str()).unwrap_or("");

    // Post dramatic message to the attached channel
    let dramatic_message = format!(
        "_Reality bends and twists as ancient magic takes hold..._\n\n_You feel the veil between dimensions shimmer and part. Another world merges with your own._\n\n*{}* _materializes from the ethereal mists, its essence now intertwined with this space._\n\n_{}_",
        room_name,
        room_description.lines().next().unwrap_or("A mysterious presence fills the air.")
    );

    let _ = state.slack_client.post_message_with_username(
        &slack_channel_id,
        &dramatic_message,
        None,
        Some("The Weave".to_string()),
        Some(":crystal_ball:".to_string()),
    ).await;

    state.slack_client.send_dm(
        &command.user_id,
        &format!("✨ Room '{}' is now attached to <#{}>. Public actions in this room will be visible in that channel.", room_name, slack_channel_id)
    ).await?;

    Ok(())
}

pub async fn handle_detach(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to use the detach command.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let current_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room to detach it!"
            ).await?;
            return Ok(());
        }
    };

    // Get room info before detaching (to know which channel to post to)
    let room = room_repo.get_by_channel_id(&current_room_id).await?;
    let room_name = room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("current room");
    let attached_channel = room.as_ref().and_then(|r| r.attached_channel_id.clone());

    // Post dramatic departure message to the attached channel (if it exists)
    if let Some(channel_id) = attached_channel {
        let departure_message = format!(
            "_The mystical connection wavers and fades..._\n\n_You feel the presence of another world withdraw. *{}* dissolves back into the ethereal mists, leaving only a faint echo of its existence._",
            room_name
        );

        let _ = state.slack_client.post_message_with_username(
            &channel_id,
            &departure_message,
            None,
            Some("The Weave".to_string()),
            Some(":crystal_ball:".to_string()),
        ).await;
    }

    // Detach the room
    room_repo.detach_from_channel(&current_room_id).await?;

    state.slack_client.send_dm(
        &command.user_id,
        &format!("✨ Room '{}' has been detached. It is now a virtual room with no Slack channel visibility.", room_name)
    ).await?;

    Ok(())
}

pub async fn handle_attach_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &user_id,
            &format!("You must be a wizard (level {}) to use the attach command.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let current_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room to attach it!"
            ).await?;
            return Ok(());
        }
    };

    // Parse channel from args
    let channel_arg = args.trim();
    if channel_arg.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `attach #channel-name`\nExample: `attach #general`"
        ).await?;
        return Ok(());
    }

    // Parse the Slack channel ID
    let slack_channel_id = if channel_arg.starts_with('#') {
        channel_arg.trim_start_matches('#').to_string()
    } else if channel_arg.starts_with('<') {
        channel_arg.trim_start_matches('<').trim_start_matches('#').split('|').next().unwrap_or(channel_arg).trim_end_matches('>').to_string()
    } else {
        channel_arg.to_string()
    };

    // Attach the room
    room_repo.attach_to_channel(&current_room_id, &slack_channel_id).await?;

    // Get room info for confirmation
    let room = room_repo.get_by_channel_id(&current_room_id).await?;
    let room_name = room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("current room");
    let room_description = room.as_ref().map(|r| r.description.as_str()).unwrap_or("");

    // Post dramatic message to the attached channel
    let dramatic_message = format!(
        "_Reality bends and twists as ancient magic takes hold..._\n\n_You feel the veil between dimensions shimmer and part. Another world merges with your own._\n\n*{}* _materializes from the ethereal mists, its essence now intertwined with this space._\n\n_{}_",
        room_name,
        room_description.lines().next().unwrap_or("A mysterious presence fills the air.")
    );

    let _ = state.slack_client.post_message_with_username(
        &slack_channel_id,
        &dramatic_message,
        None,
        Some("The Weave".to_string()),
        Some(":crystal_ball:".to_string()),
    ).await;

    state.slack_client.send_dm(
        &user_id,
        &format!("✨ Room '{}' is now attached to <#{}>. Public actions in this room will be visible in that channel.", room_name, slack_channel_id)
    ).await?;

    Ok(())
}

pub async fn handle_detach_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &user_id,
            &format!("You must be a wizard (level {}) to use the detach command.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let current_room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room to detach it!"
            ).await?;
            return Ok(());
        }
    };

    // Get room info before detaching (to know which channel to post to)
    let room = room_repo.get_by_channel_id(&current_room_id).await?;
    let room_name = room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("current room");
    let attached_channel = room.as_ref().and_then(|r| r.attached_channel_id.clone());

    // Post dramatic departure message to the attached channel (if it exists)
    if let Some(channel_id) = attached_channel {
        let departure_message = format!(
            "_The mystical connection wavers and fades..._\n\n_You feel the presence of another world withdraw. *{}* dissolves back into the ethereal mists, leaving only a faint echo of its existence._",
            room_name
        );

        let _ = state.slack_client.post_message_with_username(
            &channel_id,
            &departure_message,
            None,
            Some("The Weave".to_string()),
            Some(":crystal_ball:".to_string()),
        ).await;
    }

    // Detach the room
    room_repo.detach_from_channel(&current_room_id).await?;

    state.slack_client.send_dm(
        &user_id,
        &format!("✨ Room '{}' has been detached. It is now a virtual room with no Slack channel visibility.", room_name)
    ).await?;

    Ok(())
}
