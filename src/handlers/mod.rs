mod look;
mod character;

use crate::AppState;
use crate::slack::SlashCommand;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Form,
    http::StatusCode,
};
use std::sync::Arc;

/// Main handler for all /mud slash commands
pub async fn handle_slash_command(
    State(state): State<Arc<AppState>>,
    Form(command): Form<SlashCommand>,
) -> Response {
    tracing::info!(
        "Received command: {} from user {} in channel {}",
        command.command,
        command.user_id,
        command.channel_id
    );

    let (subcommand, args) = command.parse_subcommand();

    let result = match subcommand {
        "look" | "l" => look::handle_look(state, command).await,
        "character" | "char" => character::handle_character(state, command).await,
        "" | "help" => handle_help(state, command).await,
        _ => {
            Err(anyhow::anyhow!("Unknown command: `{}`. Type `/mud help` for available commands.", subcommand))
        }
    };

    match result {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!("Error handling command: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn handle_help(state: Arc<AppState>, command: SlashCommand) -> anyhow::Result<()> {
    let help_text = r#"*SlackMUD Commands*

• `/mud look` or `/mud l` - Look around the current room
• `/mud character` or `/mud char` - Customize your character (class, race, gender)
• `/mud help` - Show this help message

More commands coming soon!"#;

    state.slack_client.send_dm(&command.user_id, help_text).await?;
    Ok(())
}
