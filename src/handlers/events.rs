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
        EventWrapper::EventCallback { event } => {
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
                msg_event.channel.clone(),
            ).await
        }
        "character" | "char" | "c" => {
            super::character::handle_character_dm(
                state.clone(),
                user_id.clone(),
            ).await
        }
        "dig" => {
            super::dig::handle_dig_dm(
                state.clone(),
                user_id.clone(),
                user_name,
                _args,
            ).await
        }
        "help" | "h" => {
            handle_help_dm(state.clone(), user_id.clone()).await
        }
        _ => {
            // Unknown command
            let help_text = format!(
                "Unknown command: `{}`. Try:\n• `look` - Look around\n• `character` - View character\n• `dig` - (Wizards) Create exit\n• `help` - Show help",
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

async fn handle_help_dm(state: Arc<AppState>, user_id: String) -> anyhow::Result<()> {
    let help_text = r#"*SlackMUD Commands*

• `look` or `l` - Look around the current room
• `character` or `c` - View your character info
• `help` or `h` - Show this help message

You can also use `/mud` slash commands in any channel!"#;

    state.slack_client.send_dm(&user_id, help_text).await?;
    Ok(())
}
