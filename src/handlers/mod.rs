mod look;
mod character;
mod events;
mod dig;
mod r#move;
mod attach;
mod import;
mod teleport;
mod item;
mod equipment;
mod social;
mod char_creation;

pub use events::handle_events;

use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Form,
    http::StatusCode,
};
use std::sync::Arc;
use anyhow::Result;

/// Broadcast a public action to a room's channel AND to all players in that room via DM
///
/// If actor_user_id is provided, the actor receives actor_message (first-person perspective)
/// while other players receive message (third-person perspective).
pub async fn broadcast_room_action(
    state: &Arc<AppState>,
    room_channel_id: &str,
    message: &str,
    actor_user_id: Option<&str>,
    actor_message: Option<&str>,
) -> Result<()> {
    use crate::db::room::RoomRepository;

    // 1. Check if room is attached to a Slack channel
    let room_repo = RoomRepository::new(state.db_pool.clone());
    if let Some(room) = room_repo.get_by_channel_id(room_channel_id).await? {
        if let Some(attached_channel) = room.attached_channel_id {
            // Post to the attached Slack channel with a subtle bot appearance
            // Always use third-person message in the channel
            let _ = state.slack_client.post_message_with_username(
                &attached_channel,
                message,
                None,
                Some("mud".to_string()),
                Some(":game_die:".to_string()),
            ).await;
            // Ignore post errors to avoid failing the whole broadcast
        }
    }

    // 2. Send DM to all players whose current room is this room
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let players_in_room = player_repo.get_players_in_room(room_channel_id).await?;

    for player in players_in_room {
        // Determine which message to send to this player
        let player_message = if let Some(actor_id) = actor_user_id {
            if player.slack_user_id == actor_id {
                // This is the actor - send the first-person message
                actor_message.unwrap_or(message)
            } else {
                // This is another player - send the third-person message
                message
            }
        } else {
            // No actor specified - send the same message to everyone
            message
        };

        // Send the action as a DM so it appears in their SlackMUD conversation
        let _ = state.slack_client.send_dm(&player.slack_user_id, player_message).await;
        // Ignore individual DM errors to avoid failing the whole broadcast
    }

    Ok(())
}

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

    // Check if player exists and is complete
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let player = player_repo.get_by_slack_id(&command.user_id).await;

    match player {
        Ok(Some(player)) => {
            // Player exists - check if character creation is complete
            if !player.is_character_complete() {
                let error_msg = "Your character is incomplete. Please complete character creation by typing `/mud character`.";
                let _ = state.slack_client.send_dm(&command.user_id, error_msg).await;
                return StatusCode::OK.into_response();
            }
            // Character complete - proceed with command
        }
        Ok(None) => {
            // New player - start character creation
            match char_creation::start_character_creation(state.clone(), &command.user_id).await {
                Ok(_) => return StatusCode::OK.into_response(),
                Err(e) => {
                    tracing::error!("Error starting character creation: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response();
                }
            }
        }
        Err(e) => {
            tracing::error!("Error checking player: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response();
        }
    }

    let (subcommand, args) = command.parse_subcommand();

    let result = match subcommand {
        "look" | "l" => look::handle_look(state, command).await,
        "exits" => handle_exits(state, command).await,
        "character" | "char" => character::handle_character(state, command).await,
        "dig" => dig::handle_dig(state, command.clone(), args).await,
        "attach" => attach::handle_attach(state, command.clone(), args).await,
        "detach" => attach::handle_detach(state, command.clone()).await,
        "import-area" => import::handle_import_area(state, command.clone(), args).await,
        "vnums" => import::handle_vnums(state, command.clone(), args).await,
        "listitems" => import::handle_listitems(state, command.clone(), args).await,
        "teleport" | "tp" => teleport::handle_teleport(state, command.clone(), args).await,
        "move" | "go" | "m" => r#move::handle_move(state, command.clone(), args).await,
        // Directional shortcuts
        "north" | "n" => r#move::handle_move(state, command.clone(), "north").await,
        "south" | "s" => r#move::handle_move(state, command.clone(), "south").await,
        "east" | "e" => r#move::handle_move(state, command.clone(), "east").await,
        "west" | "w" => r#move::handle_move(state, command.clone(), "west").await,
        "up" | "u" => r#move::handle_move(state, command.clone(), "up").await,
        "down" | "d" => r#move::handle_move(state, command.clone(), "down").await,
        // Item commands
        "get" | "take" => item::handle_get(state, command.clone(), args).await,
        "drop" => item::handle_drop(state, command.clone(), args).await,
        "inventory" | "inv" | "i" => item::handle_inventory(state, command).await,
        "manifest" => item::handle_manifest(state, command.clone(), args).await,
        // Equipment commands
        "wear" => equipment::handle_wear(state, command.clone(), args).await,
        "wield" => equipment::handle_wield(state, command.clone(), args).await,
        "remove" | "rem" => equipment::handle_remove(state, command.clone(), args).await,
        "equipment" | "eq" => equipment::handle_equipment(state, command).await,
        "socials" => handle_socials_list(state, command).await,
        "" | "help" => handle_help(state, command).await,
        _ => {
            // Check if it's a social command
            if crate::social::get_social(subcommand).is_some() {
                social::handle_social(state, command.clone(), subcommand, args).await
            } else {
                Err(anyhow::anyhow!("Unknown command: `{}`. Type `/mud help` for available commands.", subcommand))
            }
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

async fn handle_exits(state: Arc<AppState>, command: SlashCommand) -> anyhow::Result<()> {
    use crate::db::room::RoomRepository;
    use crate::db::exit::ExitRepository;

    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let channel_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
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

    state.slack_client.send_dm(&command.user_id, &message).await?;
    Ok(())
}

async fn handle_help(state: Arc<AppState>, command: SlashCommand) -> anyhow::Result<()> {
    // Check if user is a wizard
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;
    let is_wizard = player.level >= 50;

    let mut help_text = String::from("*SlackMUD Commands*\n\n");
    help_text.push_str("• `/mud look` or `/mud l` - Look around the current room\n");
    help_text.push_str("• `/mud look <item>` - Examine an item in detail\n");
    help_text.push_str("• `/mud exits` - Show available exits\n");
    help_text.push_str("• `/mud n/s/e/w/u/d` or `/mud north/south/east/west/up/down` - Move in a direction\n");
    help_text.push_str("• `/mud get <item>` or `/mud take <item>` - Pick up an item\n");
    help_text.push_str("• `/mud drop <item>` - Drop an item\n");
    help_text.push_str("• `/mud inventory` or `/mud i` - Show what you're carrying\n");
    help_text.push_str("• `/mud wear <item>` - Wear armor or clothing\n");
    help_text.push_str("• `/mud wield <weapon>` - Wield a weapon\n");
    help_text.push_str("• `/mud remove <item>` - Remove equipped item\n");
    help_text.push_str("• `/mud equipment` or `/mud eq` - Show your equipment\n");
    help_text.push_str("• `/mud character` or `/mud char` - Customize your character (class, race, gender)\n");
    help_text.push_str("• `/mud socials` - List all available social commands\n");
    help_text.push_str("• `/mud <social> [player]` - Perform a social action (e.g., `/mud smile` or `/mud hug bob`)\n");

    if is_wizard {
        help_text.push_str("\n*Wizard Commands:*\n");
        help_text.push_str("• `/mud dig <direction> #channel` - Create an exit to another room\n");
        help_text.push_str("• `/mud attach #channel` - Attach current room to a Slack channel\n");
        help_text.push_str("• `/mud detach` - Detach current room from its Slack channel\n");
        help_text.push_str("• `/mud import-area <url>` - Import MUD area file (creates virtual rooms)\n");
        help_text.push_str("• `/mud vnums [page]` - List all imported virtual rooms\n");
        help_text.push_str("• `/mud listitems [page]` - List all unique item definitions\n");
        help_text.push_str("• `/mud manifest <vnum|name>` - Magically create an item in the room\n");
        help_text.push_str("• `/mud teleport <vnum>` - Teleport yourself to a room\n");
        help_text.push_str("• `/mud teleport <player> <vnum>` - Teleport another player to a room\n");
    }

    help_text.push_str("\n• `/mud help` - Show this help message\n");
    help_text.push_str("\nYou can also DM me directly with commands (without `/mud`)!");

    state.slack_client.send_dm(&command.user_id, &help_text).await?;
    Ok(())
}

async fn handle_socials_list(state: Arc<AppState>, command: SlashCommand) -> anyhow::Result<()> {
    let social_names = crate::social::get_all_social_names();

    let mut message = String::from("*Available Social Commands:*\n\n");
    message.push_str("Use `/mud <social>` or `/mud <social> <player>` to perform these actions:\n\n");

    // Display in columns
    let mut col = 0;
    for name in &social_names {
        message.push_str(&format!("{:<15}", name));
        col += 1;
        if col >= 4 {
            message.push('\n');
            col = 0;
        }
    }

    if col > 0 {
        message.push('\n');
    }

    message.push_str(&format!("\n_Total: {} social commands available_", social_names.len()));

    state.slack_client.send_dm(&command.user_id, &message).await?;
    Ok(())
}
