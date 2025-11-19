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

    // First pass: Create all rooms
    for area_room in &area_file.rooms {
        let room_id = format!("vnum_{}", area_room.vnum);

        let room = Room {
            channel_id: room_id.clone(),
            channel_name: area_room.name.clone(),
            description: area_room.description.clone(),
            attached_channel_id: None, // Virtual room (not attached)
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        room_repo.create(&room).await?;
        rooms_created += 1;
    }

    // Second pass: Create all exits (now that all rooms exist)
    for area_room in &area_file.rooms {
        let room_id = format!("vnum_{}", area_room.vnum);

        for area_exit in &area_room.exits {
            let to_room_id = format!("vnum_{}", area_exit.to_room);

            // Skip exits that point to rooms outside this area (exits to other areas)
            if area_exit.to_room < area_file.header.min_vnum || area_exit.to_room > area_file.header.max_vnum {
                tracing::debug!("Skipping exit from {} to {} (outside area range)", room_id, to_room_id);
                continue;
            }

            let exit = Exit::new(
                room_id.clone(),
                area_exit.direction.as_str().to_string(),
                to_room_id,
                Some(player_slack_id.clone()),
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

        // Show attached channel if it exists
        if let Some(ref attached_channel) = room.attached_channel_id {
            message.push_str(&format!("• `{}` - {} [attached to <#{}>]\n", vnum_display, room.channel_name, attached_channel));
        } else {
            message.push_str(&format!("• `{}` - {}\n", vnum_display, room.channel_name));
        }
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

        // Show attached channel if it exists
        if let Some(ref attached_channel) = room.attached_channel_id {
            message.push_str(&format!("• `{}` - {} [attached to <#{}>]\n", vnum_display, room.channel_name, attached_channel));
        } else {
            message.push_str(&format!("• `{}` - {}\n", vnum_display, room.channel_name));
        }
    }

    if total_pages > 1 {
        message.push_str(&format!("\n_Use `vnums {}` for next page_", page + 1));
    }

    state.slack_client.send_dm(&user_id, &message).await?;
    Ok(())
}

async fn list_virtual_rooms(state: Arc<AppState>) -> Result<Vec<crate::models::Room>> {
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

pub async fn handle_listitems(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to list items.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Parse args: search query and/or page number
    // Examples: "bre", "bre 2", "ice breaker", "ice breaker 3", "2" (just page)
    let (search_query, page) = parse_search_and_page(args);
    tracing::info!("listitems: args='{}', parsed search_query={:?}, page={}", args, search_query, page);
    const PAGE_SIZE: usize = 20;

    // Fetch all objects
    let mut objects = list_all_objects(state.clone()).await?;

    // Filter by search query if provided
    if let Some(ref query) = search_query {
        let query_lower = query.to_lowercase();
        let before_count = objects.len();
        objects.retain(|obj| {
            // Search in keywords and short description
            obj.keywords.to_lowercase().contains(&query_lower)
                || obj.short_description.to_lowercase().contains(&query_lower)
        });
        tracing::info!("listitems: filtered from {} to {} items for query '{}'", before_count, objects.len(), query);
    }

    if objects.is_empty() {
        let message = if search_query.is_some() {
            format!("No items found matching '{}'.", search_query.unwrap())
        } else {
            "No items found. Use `/mud import-area <url>` to import an area file with objects.".to_string()
        };
        state.slack_client.send_dm(&command.user_id, &message).await?;
        return Ok(());
    }

    // Calculate pagination
    let total_objects = objects.len();
    let total_pages = (total_objects + PAGE_SIZE - 1) / PAGE_SIZE;
    let start_idx = (page - 1) * PAGE_SIZE;
    let end_idx = (start_idx + PAGE_SIZE).min(total_objects);

    if start_idx >= total_objects {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("Page {} not found. Total pages: {}", page, total_pages)
        ).await?;
        return Ok(());
    }

    // Build the list message
    let mut message = if let Some(ref query) = search_query {
        format!("*Items matching '{}'* (Page {} of {})\n", query, page, total_pages)
    } else {
        format!("*Items (Page {} of {})*\n", page, total_pages)
    };
    message.push_str(&format!("_Total items: {}_\n\n", total_objects));

    for (idx, object) in objects.iter().enumerate().skip(start_idx).take(PAGE_SIZE) {
        if idx >= end_idx {
            break;
        }

        // Display vnum, short description, type, and level
        message.push_str(&format!(
            "• `{}` - {} [{}{}]\n",
            object.vnum,
            object.short_description,
            object.item_type,
            if object.level > 0 {
                format!(", lvl {}", object.level)
            } else {
                String::new()
            }
        ));
    }

    if total_pages > 1 {
        let next_cmd = if let Some(ref query) = search_query {
            format!("/mud listitems {} {}", query, page + 1)
        } else {
            format!("/mud listitems {}", page + 1)
        };
        message.push_str(&format!("\n_Use `{}` for next page_", next_cmd));
    }

    state.slack_client.send_dm(&command.user_id, &message).await?;
    Ok(())
}

pub async fn handle_listitems_dm(
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
            &format!("You must be a wizard (level {}) to list items.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Parse args: search query and/or page number
    let (search_query, page) = parse_search_and_page(args);
    const PAGE_SIZE: usize = 20;

    // Fetch all objects
    let mut objects = list_all_objects(state.clone()).await?;

    // Filter by search query if provided
    if let Some(ref query) = search_query {
        let query_lower = query.to_lowercase();
        objects.retain(|obj| {
            // Search in keywords and short description
            obj.keywords.to_lowercase().contains(&query_lower)
                || obj.short_description.to_lowercase().contains(&query_lower)
        });
    }

    if objects.is_empty() {
        let message = if search_query.is_some() {
            format!("No items found matching '{}'.", search_query.unwrap())
        } else {
            "No items found. Use `import-area <url>` to import an area file with objects.".to_string()
        };
        state.slack_client.send_dm(&user_id, &message).await?;
        return Ok(());
    }

    // Calculate pagination
    let total_objects = objects.len();
    let total_pages = (total_objects + PAGE_SIZE - 1) / PAGE_SIZE;
    let start_idx = (page - 1) * PAGE_SIZE;
    let end_idx = (start_idx + PAGE_SIZE).min(total_objects);

    if start_idx >= total_objects {
        state.slack_client.send_dm(
            &user_id,
            &format!("Page {} not found. Total pages: {}", page, total_pages)
        ).await?;
        return Ok(());
    }

    // Build the list message
    let mut message = if let Some(ref query) = search_query {
        format!("*Items matching '{}'* (Page {} of {})\n", query, page, total_pages)
    } else {
        format!("*Items (Page {} of {})*\n", page, total_pages)
    };
    message.push_str(&format!("_Total items: {}_\n\n", total_objects));

    for (idx, object) in objects.iter().enumerate().skip(start_idx).take(PAGE_SIZE) {
        if idx >= end_idx {
            break;
        }

        // Display vnum, short description, type, and level
        message.push_str(&format!(
            "• `{}` - {} [{}{}]\n",
            object.vnum,
            object.short_description,
            object.item_type,
            if object.level > 0 {
                format!(", lvl {}", object.level)
            } else {
                String::new()
            }
        ));
    }

    if total_pages > 1 {
        let next_cmd = if let Some(ref query) = search_query {
            format!("listitems {} {}", query, page + 1)
        } else {
            format!("listitems {}", page + 1)
        };
        message.push_str(&format!("\n_Use `{}` for next page_", next_cmd));
    }

    state.slack_client.send_dm(&user_id, &message).await?;
    Ok(())
}

async fn list_all_objects(state: Arc<AppState>) -> Result<Vec<crate::models::Object>> {
    // Query all objects ordered by vnum
    let objects = sqlx::query_as::<_, crate::models::Object>(
        "SELECT * FROM objects ORDER BY vnum"
    )
    .fetch_all(&state.db_pool)
    .await?;

    Ok(objects)
}

/// Parse args to extract search query and page number
/// Examples:
///   "" -> (None, 1)
///   "2" -> (None, 2)
///   "bre" -> (Some("bre"), 1)
///   "bre 2" -> (Some("bre"), 2)
///   "ice breaker" -> (Some("ice breaker"), 1)
///   "ice breaker 3" -> (Some("ice breaker"), 3)
fn parse_search_and_page(args: &str) -> (Option<String>, usize) {
    let args = args.trim();

    if args.is_empty() {
        return (None, 1);
    }

    let words: Vec<&str> = args.split_whitespace().collect();

    if words.is_empty() {
        return (None, 1);
    }

    // Check if last word is a number
    if let Ok(page_num) = words.last().unwrap().parse::<usize>() {
        if words.len() == 1 {
            // Just a page number, no search query
            (None, page_num.max(1))
        } else {
            // Search query followed by page number
            let search = words[..words.len() - 1].join(" ");
            (Some(search), page_num.max(1))
        }
    } else {
        // No page number at the end, entire args is search query
        (Some(args.to_string()), 1)
    }
}
