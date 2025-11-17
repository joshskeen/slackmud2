use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use crate::db::exit::ExitRepository;
use crate::db::area::AreaRepository;
use crate::area::parser::parse_area_file;
use crate::models::{Room, Exit, Area};
use std::sync::Arc;
use anyhow::Result;

const WIZARD_LEVEL: i32 = 50;

pub async fn handle_import_area(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to import area files.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Parse args for URL and --force flag
    let args = args.trim();
    let force = args.contains("--force");
    let url = args.replace("--force", "").trim().to_string();

    if url.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud import-area <url> [--force]`\nExample: `/mud import-area https://raw.githubusercontent.com/avinson/rom24-quickmud/main/area/midgaard.are`\n\nUse `--force` to re-import an area that was already imported."
        ).await?;
        return Ok(());
    }

    // Send initial message
    state.slack_client.send_dm(
        &command.user_id,
        &format!("🔄 Fetching area file from: {}", url)
    ).await?;

    // Fetch the area file
    let content = match fetch_area_file(&url).await {
        Ok(c) => c,
        Err(e) => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("❌ Failed to fetch area file: {}", e)
            ).await?;
            return Ok(());
        }
    };

    import_area_from_content(
        state,
        &command.user_id,
        player.slack_user_id.clone(),
        &content,
        force,
    ).await
}

pub async fn handle_import_area_dm(
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
            &format!("You must be a wizard (level {}) to import area files.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Parse args for URL and --force flag
    let args = args.trim();
    let force = args.contains("--force");
    let url = args.replace("--force", "").trim().to_string();

    if url.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `import-area <url> [--force]`\nExample: `import-area https://raw.githubusercontent.com/avinson/rom24-quickmud/main/area/midgaard.are`\n\nUse `--force` to re-import an area that was already imported."
        ).await?;
        return Ok(());
    }

    // Send initial message
    state.slack_client.send_dm(
        &user_id,
        &format!("🔄 Fetching area file from: {}", url)
    ).await?;

    // Fetch the area file
    let content = match fetch_area_file(&url).await {
        Ok(c) => c,
        Err(e) => {
            state.slack_client.send_dm(
                &user_id,
                &format!("❌ Failed to fetch area file: {}", e)
            ).await?;
            return Ok(());
        }
    };

    import_area_from_content(
        state,
        &user_id,
        player.slack_user_id.clone(),
        &content,
        force,
    ).await
}

/// Shared function to import an area from file content
async fn import_area_from_content(
    state: Arc<AppState>,
    user_id: &str,
    player_slack_id: String,
    content: &str,
    force: bool,
) -> Result<()> {
    let area_repo = AreaRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

    // Parse the area file
    state.slack_client.send_dm(
        user_id,
        "🔄 Parsing area file..."
    ).await?;

    let area_file = match parse_area_file(content) {
        Ok(a) => a,
        Err(e) => {
            state.slack_client.send_dm(
                user_id,
                &format!("❌ Failed to parse area file: {}", e)
            ).await?;
            return Ok(());
        }
    };

    // Report what was parsed
    state.slack_client.send_dm(
        user_id,
        &format!("✅ Parsed area: *{}*\n📖 Rooms found: {}\n📊 Vnum range: {}-{}",
            area_file.header.name,
            area_file.rooms.len(),
            area_file.header.min_vnum,
            area_file.header.max_vnum
        )
    ).await?;

    // Check if area already exists
    if area_repo.exists(&area_file.header.name).await? {
        if !force {
            state.slack_client.send_dm(
                user_id,
                &format!("⚠️  Area *{}* has already been imported.\n\nUse `--force` flag to re-import:\n`import-area <url> --force`",
                    area_file.header.name
                )
            ).await?;
            return Ok(());
        } else {
            // Delete existing area
            state.slack_client.send_dm(
                user_id,
                &format!("🗑️  Deleting existing area *{}*...", area_file.header.name)
            ).await?;
            area_repo.delete_by_name(&area_file.header.name).await?;
        }
    }

    // Import rooms
    state.slack_client.send_dm(
        user_id,
        "🔄 Importing rooms to database..."
    ).await?;

    let mut rooms_created = 0;
    let mut exits_created = 0;

    for area_room in &area_file.rooms {
        // Convert AreaRoom to our Room model
        let room_id = format!("vnum_{}", area_room.vnum);

        let room = Room {
            channel_id: room_id.clone(),
            channel_name: area_room.name.clone(),
            description: area_room.description.clone(),
            attached_channel_id: None, // Virtual room (not attached)
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        // Create room
        room_repo.create(&room).await?;
        rooms_created += 1;

        // Create exits
        for area_exit in &area_room.exits {
            let to_room_id = format!("vnum_{}", area_exit.to_room);

            let exit = Exit::new(
                room_id.clone(),
                area_exit.direction.as_str().to_string(),
                to_room_id,
                player_slack_id.clone(),
            );

            exit_repo.create(&exit).await?;
            exits_created += 1;
        }
    }

    // Record the area in the database
    let area = Area::new(
        area_file.header.name.clone(),
        area_file.header.filename.clone(),
        area_file.header.min_vnum,
        area_file.header.max_vnum,
        rooms_created,
        exits_created,
    );
    area_repo.create(&area).await?;

    // Report success
    state.slack_client.send_dm(
        user_id,
        &format!("✨ *Import complete!*\n\n📦 Area: *{}*\n🏠 Rooms created: {}\n🚪 Exits created: {}\n\n💡 These are virtual rooms (not attached to Slack channels). Use `attach #channel` to make a room visible in a channel.",
            area_file.header.name,
            rooms_created,
            exits_created
        )
    ).await?;

    Ok(())
}

pub async fn handle_vnums(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to list vnums.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Parse page number from args (default to 1)
    let page: usize = args.trim().parse().unwrap_or(1).max(1);
    const PAGE_SIZE: usize = 20;

    // Fetch all virtual rooms (those starting with vnum_)
    let rooms = list_virtual_rooms(state.clone()).await?;

    if rooms.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "No virtual rooms found. Use `/mud import-area <url>` to import an area file."
        ).await?;
        return Ok(());
    }

    // Calculate pagination
    let total_rooms = rooms.len();
    let total_pages = (total_rooms + PAGE_SIZE - 1) / PAGE_SIZE;
    let start_idx = (page - 1) * PAGE_SIZE;
    let end_idx = (start_idx + PAGE_SIZE).min(total_rooms);

    if start_idx >= total_rooms {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("Page {} not found. Total pages: {}", page, total_pages)
        ).await?;
        return Ok(());
    }

    // Build the list message
    let mut message = format!("*Virtual Rooms (Page {} of {})*\n", page, total_pages);
    message.push_str(&format!("_Total rooms: {}_\n\n", total_rooms));

    for (idx, room) in rooms.iter().enumerate().skip(start_idx).take(PAGE_SIZE) {
        if idx >= end_idx {
            break;
        }

        // Extract vnum from channel_id (format: vnum_3001)
        let vnum_display = room.channel_id.strip_prefix("vnum_").unwrap_or(&room.channel_id);
        message.push_str(&format!("• `{}` - {}\n", vnum_display, room.channel_name));
    }

    if total_pages > 1 {
        message.push_str(&format!("\n_Use `/mud vnums {}` for next page_", page + 1));
    }

    state.slack_client.send_dm(&command.user_id, &message).await?;
    Ok(())
}

pub async fn handle_vnums_dm(
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
            &format!("You must be a wizard (level {}) to list vnums.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Parse page number from args
    let page: usize = args.trim().parse().unwrap_or(1).max(1);
    const PAGE_SIZE: usize = 20;

    // Fetch all virtual rooms
    let rooms = list_virtual_rooms(state.clone()).await?;

    if rooms.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "No virtual rooms found. Use `import-area <url>` to import an area file."
        ).await?;
        return Ok(());
    }

    // Calculate pagination
    let total_rooms = rooms.len();
    let total_pages = (total_rooms + PAGE_SIZE - 1) / PAGE_SIZE;
    let start_idx = (page - 1) * PAGE_SIZE;
    let end_idx = (start_idx + PAGE_SIZE).min(total_rooms);

    if start_idx >= total_rooms {
        state.slack_client.send_dm(
            &user_id,
            &format!("Page {} not found. Total pages: {}", page, total_pages)
        ).await?;
        return Ok(());
    }

    // Build the list message
    let mut message = format!("*Virtual Rooms (Page {} of {})*\n", page, total_pages);
    message.push_str(&format!("_Total rooms: {}_\n\n", total_rooms));

    for (idx, room) in rooms.iter().enumerate().skip(start_idx).take(PAGE_SIZE) {
        if idx >= end_idx {
            break;
        }

        let vnum_display = room.channel_id.strip_prefix("vnum_").unwrap_or(&room.channel_id);
        message.push_str(&format!("• `{}` - {}\n", vnum_display, room.channel_name));
    }

    if total_pages > 1 {
        message.push_str(&format!("\n_Use `vnums {}` for next page_", page + 1));
    }

    state.slack_client.send_dm(&user_id, &message).await?;
    Ok(())
}

async fn list_virtual_rooms(state: Arc<AppState>) -> Result<Vec<crate::models::Room>> {
    use sqlx::Row;

    // Query all rooms where channel_id starts with "vnum_"
    let rooms = sqlx::query_as::<_, crate::models::Room>(
        "SELECT * FROM rooms WHERE channel_id LIKE 'vnum_%' ORDER BY channel_id"
    )
    .fetch_all(&state.db_pool)
    .await?;

    Ok(rooms)
}

async fn fetch_area_file(url: &str) -> Result<String> {
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    let content = response.text().await?;
    Ok(content)
}
