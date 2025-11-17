use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::room::RoomRepository;
use crate::db::exit::ExitRepository;
use crate::area::parser::parse_area_file;
use crate::models::{Room, Exit};
use std::sync::Arc;
use anyhow::Result;

const WIZARD_LEVEL: i32 = 50;

pub async fn handle_import_area(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

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

    // Parse URL from args
    let url = args.trim();
    if url.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud import-area <url>`\nExample: `/mud import-area https://raw.githubusercontent.com/avinson/rom24-quickmud/main/area/midgaard.are`"
        ).await?;
        return Ok(());
    }

    // Send initial message
    state.slack_client.send_dm(
        &command.user_id,
        &format!("🔄 Fetching area file from: {}", url)
    ).await?;

    // Fetch the area file
    let content = match fetch_area_file(url).await {
        Ok(c) => c,
        Err(e) => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("❌ Failed to fetch area file: {}", e)
            ).await?;
            return Ok(());
        }
    };

    // Parse the area file
    state.slack_client.send_dm(
        &command.user_id,
        "🔄 Parsing area file..."
    ).await?;

    let area_file = match parse_area_file(&content) {
        Ok(a) => a,
        Err(e) => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("❌ Failed to parse area file: {}", e)
            ).await?;
            return Ok(());
        }
    };

    // Report what was parsed
    state.slack_client.send_dm(
        &command.user_id,
        &format!("✅ Parsed area: *{}*\n📖 Rooms found: {}\n📊 Vnum range: {}-{}",
            area_file.header.name,
            area_file.rooms.len(),
            area_file.header.min_vnum,
            area_file.header.max_vnum
        )
    ).await?;

    // Import rooms
    state.slack_client.send_dm(
        &command.user_id,
        "🔄 Importing rooms to database..."
    ).await?;

    let mut rooms_created = 0;
    let mut exits_created = 0;

    for area_room in &area_file.rooms {
        // Convert AreaRoom to our Room model
        // Use vnum as room_id (virtual room)
        let room_id = format!("vnum_{}", area_room.vnum);

        let room = Room {
            channel_id: room_id.clone(),
            channel_name: area_room.name.clone(),
            description: area_room.description.clone(),
            attached_channel_id: None, // Virtual room (not attached to Slack channel)
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
                player.slack_user_id.clone(),
            );

            exit_repo.create(&exit).await?;
            exits_created += 1;
        }
    }

    // Report success
    state.slack_client.send_dm(
        &command.user_id,
        &format!("✨ *Import complete!*\n\n📦 Area: *{}*\n🏠 Rooms created: {}\n🚪 Exits created: {}\n\n💡 These are virtual rooms (not attached to Slack channels). Use `/mud attach #channel` to make a room visible in a channel.",
            area_file.header.name,
            rooms_created,
            exits_created
        )
    ).await?;

    Ok(())
}

pub async fn handle_import_area_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let room_repo = RoomRepository::new(state.db_pool.clone());
    let exit_repo = ExitRepository::new(state.db_pool.clone());

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

    // Parse URL from args
    let url = args.trim();
    if url.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `import-area <url>`\nExample: `import-area https://raw.githubusercontent.com/avinson/rom24-quickmud/main/area/midgaard.are`"
        ).await?;
        return Ok(());
    }

    // Send initial message
    state.slack_client.send_dm(
        &user_id,
        &format!("🔄 Fetching area file from: {}", url)
    ).await?;

    // Fetch the area file
    let content = match fetch_area_file(url).await {
        Ok(c) => c,
        Err(e) => {
            state.slack_client.send_dm(
                &user_id,
                &format!("❌ Failed to fetch area file: {}", e)
            ).await?;
            return Ok(());
        }
    };

    // Parse the area file
    state.slack_client.send_dm(
        &user_id,
        "🔄 Parsing area file..."
    ).await?;

    let area_file = match parse_area_file(&content) {
        Ok(a) => a,
        Err(e) => {
            state.slack_client.send_dm(
                &user_id,
                &format!("❌ Failed to parse area file: {}", e)
            ).await?;
            return Ok(());
        }
    };

    // Report what was parsed
    state.slack_client.send_dm(
        &user_id,
        &format!("✅ Parsed area: *{}*\n📖 Rooms found: {}\n📊 Vnum range: {}-{}",
            area_file.header.name,
            area_file.rooms.len(),
            area_file.header.min_vnum,
            area_file.header.max_vnum
        )
    ).await?;

    // Import rooms
    state.slack_client.send_dm(
        &user_id,
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
                player.slack_user_id.clone(),
            );

            exit_repo.create(&exit).await?;
            exits_created += 1;
        }
    }

    // Report success
    state.slack_client.send_dm(
        &user_id,
        &format!("✨ *Import complete!*\n\n📦 Area: *{}*\n🏠 Rooms created: {}\n🚪 Exits created: {}\n\n💡 These are virtual rooms (not attached to Slack channels). Use `attach #channel` to make a room visible in a channel.",
            area_file.header.name,
            rooms_created,
            exits_created
        )
    ).await?;

    Ok(())
}

async fn fetch_area_file(url: &str) -> Result<String> {
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    let content = response.text().await?;
    Ok(content)
}
