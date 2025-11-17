use crate::AppState;
use crate::slack::{EventWrapper, Event, MessageEvent};
use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
    http::StatusCode,
};
use std::sync::Arc;
use serde_json::json;

/// Handle Slack Events API callbacks
pub async fn handle_events(
    State(state): State<Arc<AppState>>,
    Json(event_wrapper): Json<EventWrapper>,
) -> Response {
    match event_wrapper {
        // URL verification challenge (first-time setup)
        EventWrapper::UrlVerification { challenge } => {
            tracing::info!("Received URL verification challenge");
            Json(json!({ "challenge": challenge })).into_response()
        }

        // Event callback (actual events)
        EventWrapper::EventCallback { event, event_id } => {
            // Check for duplicate events
            {
                let mut recent_events = state.recent_event_ids.lock().unwrap();

                // If we've already seen this event, skip it
                if recent_events.contains(&event_id) {
                    tracing::debug!("Skipping duplicate event: {}", event_id);
                    return StatusCode::OK.into_response();
                }

                // Add to recent events (keep last 1000)
                recent_events.push_back(event_id.clone());
                if recent_events.len() > 1000 {
                    recent_events.pop_front();
                }
            }

            match event {
                Event::Message(msg_event) => {
                    handle_message_event(state, msg_event).await.into_response()
                }
            }
        }
    }
}

/// Handle a message event (DM to the bot)
async fn handle_message_event(
    state: Arc<AppState>,
    msg_event: MessageEvent,
) -> impl IntoResponse {
    // Ignore messages from bots to avoid loops
    if msg_event.is_from_bot() {
        tracing::debug!("Ignoring bot message");
        return StatusCode::OK.into_response();
    }

    // Only handle DMs
    if !msg_event.is_dm() {
        tracing::debug!("Ignoring non-DM message");
        return StatusCode::OK.into_response();
    }

    // Get user ID
    let user_id = match msg_event.user {
        Some(ref id) => id.clone(),
        None => {
            tracing::warn!("Message event missing user ID");
            return StatusCode::OK.into_response();
        }
    };

    tracing::info!(
        "Received DM from user {}: {}",
        user_id,
        msg_event.text
    );

    // Parse the command from the message
    let (command, _args) = msg_event.parse_command();

    // Get the user's real name
    let user_name = match state.slack_client.get_user_real_name(&user_id).await {
        Ok(name) => name,
        Err(e) => {
            tracing::error!("Failed to get user name: {}", e);
            user_id.clone()
        }
    };

    // Route to command handlers
    let result = match command.to_lowercase().as_str() {
        "look" | "l" => {
            super::look::handle_look_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        "character" | "char" | "c" => {
            super::character::handle_character_dm(
                state.clone(),
                user_id.clone(),
            ).await
        }
        "exits" => {
            handle_exits_dm(state.clone(), user_id.clone(), user_name).await
        }
        "move" | "go" | "m" => {
            super::r#move::handle_move_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        // Directional shortcuts - move commands
        "north" | "n" => {
            super::r#move::handle_move_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                "north",
            ).await
        }
        "south" | "s" => {
            super::r#move::handle_move_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                "south",
            ).await
        }
        "east" | "e" => {
            super::r#move::handle_move_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                "east",
            ).await
        }
        "west" | "w" => {
            super::r#move::handle_move_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                "west",
            ).await
        }
        "up" | "u" => {
            super::r#move::handle_move_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                "up",
            ).await
        }
        "down" | "d" => {
            super::r#move::handle_move_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                "down",
            ).await
        }
        "dig" => {
            super::dig::handle_dig_dm(
                state.clone(),
                user_id.clone(),
                user_name.clone(),
                _args,
            ).await
        }
        "attach" => {
            super::attach::handle_attach_dm(
                state.clone(),
                user_id.clone(),
                user_name.clone(),
                _args,
            ).await
        }
        "detach" => {
            super::attach::handle_detach_dm(
                state.clone(),
                user_id.clone(),
                user_name.clone(),
            ).await
        }
        "import-area" => {
            super::import::handle_import_area_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        "vnums" => {
            super::import::handle_vnums_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        "teleport" | "tp" => {
            super::teleport::handle_teleport_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        // Item commands
        "get" | "take" => {
            super::item::handle_get_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        "drop" => {
            super::item::handle_drop_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        "inventory" | "inv" | "i" => {
            super::item::handle_inventory_dm(
                state.clone(),
                user_id.clone(),
                user_name,
            ).await
        }
        "help" | "h" => {
            handle_help_dm(state.clone(), user_id.clone()).await
        }
        _ => {
            // Unknown command
            let help_text = format!(
                "Unknown command: `{}`. Try:\n• `look` - Look around\n• `n/s/e/w/u/d` - Move in a direction\n• `get <item>` - Pick up an item\n• `drop <item>` - Drop an item\n• `inventory` - Show what you're carrying\n• `exits` - Show available exits\n• `character` - View character\n• `help` - Show help",
                command
            );
            state.slack_client.send_dm(&user_id, &help_text).await
        }
    };

    match result {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!("Error handling message event: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn handle_exits_dm(state: Arc<AppState>, user_id: String, user_name: String) -> anyhow::Result<()> {
    use crate::db::player::PlayerRepository;
    use crate::db::room::RoomRepository;
    use crate::db::exit::ExitRepository;

    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player has a current room
    let channel_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    // Get the room
    let room = room_repo.get_by_channel_id(&channel_id).await?;
    let room_name = room.as_ref().map(|r| r.channel_name.as_str()).unwrap_or("unknown");

    // Get exits
    let exits = exit_repo.get_exits_from_room(&channel_id).await?;

    let message = if exits.is_empty() {
        format!("*Exits from #{}:*\nThere are no exits from this room.", room_name)
    } else {
        let mut msg = format!("*Exits from #{}:*\n", room_name);
        for exit in &exits {
            let target_room_name = if let Some(room) = room_repo.get_by_channel_id(&exit.to_room_id).await? {
                room.channel_name
            } else {
                exit.to_room_id.clone()
            };
            msg.push_str(&format!("• *{}* → #{}\n", exit.direction, target_room_name));
        }
        msg
    };

    state.slack_client.send_dm(&user_id, &message).await?;
    Ok(())
}

async fn handle_help_dm(state: Arc<AppState>, user_id: String) -> anyhow::Result<()> {
    use crate::db::player::PlayerRepository;

    // Check if user is a wizard
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let user_name = match state.slack_client.get_user_real_name(&user_id).await {
        Ok(name) => name,
        Err(_) => user_id.clone(),
    };
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;
    let is_wizard = player.level >= 50;

    let mut help_text = String::from("*SlackMUD Commands*\n\n");
    help_text.push_str("• `look` or `l` - Look around the current room\n");
    help_text.push_str("• `look <item>` - Examine an item in detail\n");
    help_text.push_str("• `exits` - Show available exits\n");
    help_text.push_str("• `n/s/e/w/u/d` or `north/south/east/west/up/down` - Move in a direction\n");
    help_text.push_str("• `get <item>` or `take <item>` - Pick up an item\n");
    help_text.push_str("• `drop <item>` - Drop an item\n");
    help_text.push_str("• `inventory` or `i` - Show what you're carrying\n");
    help_text.push_str("• `character` or `c` - View your character info\n");

    if is_wizard {
        help_text.push_str("\n*Wizard Commands:*\n");
        help_text.push_str("• `dig <direction> #channel` - Create an exit\n");
        help_text.push_str("• `attach #channel` - Attach current room to a Slack channel\n");
        help_text.push_str("• `detach` - Detach current room from its Slack channel\n");
        help_text.push_str("• `import-area <url>` - Import MUD area file (creates virtual rooms)\n");
        help_text.push_str("• `vnums [page]` - List all imported virtual rooms\n");
        help_text.push_str("• `teleport <vnum>` - Teleport yourself to a room\n");
        help_text.push_str("• `teleport <player> <vnum>` - Teleport another player to a room\n");
    }

    help_text.push_str("\n• `help` or `h` - Show this help message\n");
    help_text.push_str("\nYou can also use `/mud` slash commands in any channel!");

    state.slack_client.send_dm(&user_id, &help_text).await?;
    Ok(())
}
