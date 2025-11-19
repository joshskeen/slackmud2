mod models;
mod db;
mod slack;
mod handlers;
mod area;
mod social;

use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    pub db_pool: PgPool,
    pub slack_client: slack::SlackClient,
    pub recent_event_ids: Mutex<VecDeque<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "slackmud=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?;
    let slack_bot_token = std::env::var("SLACK_BOT_TOKEN")
        .context("SLACK_BOT_TOKEN must be set")?;
    let host = std::env::var("HOST")
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .context("Invalid PORT")?;

    // Set up database
    tracing::info!("Connecting to database: {}", database_url);
    let db_pool = db::create_pool(&database_url).await
        .context("Failed to create database pool")?;

    // Run migrations
    tracing::info!("Running database migrations");
    db::run_migrations(&db_pool).await
        .context("Failed to run migrations")?;

    // Load and promote wizards
    tracing::info!("Loading wizards");
    load_wizards(&db_pool).await?;

    // Load default areas (like Midgaard)
    tracing::info!("Loading default areas");
    load_default_areas(&db_pool).await?;

    // Create Slack client
    let slack_client = slack::SlackClient::new(slack_bot_token);

    // Create shared application state
    let state = Arc::new(AppState {
        db_pool,
        slack_client,
        recent_event_ids: Mutex::new(VecDeque::with_capacity(1000)),
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/slack/commands", post(handlers::handle_slash_command))
        .route("/slack/events", post(handlers::handle_events))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", host, port);
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

/// Load wizards from environment variable or wizards.txt file
async fn load_wizards(pool: &sqlx::PgPool) -> Result<()> {
    use db::player::PlayerRepository;

    let mut wizard_ids = Vec::new();

    // Try environment variable first (for production)
    if let Ok(wizards_env) = std::env::var("WIZARDS") {
        tracing::info!("Loading wizards from WIZARDS environment variable");
        for id in wizards_env.split(',') {
            let id = id.trim();
            if !id.is_empty() {
                wizard_ids.push(id.to_string());
            }
        }
    }
    // Fall back to wizards.txt file (for local development)
    else if let Ok(contents) = tokio::fs::read_to_string("wizards.txt").await {
        tracing::info!("Loading wizards from wizards.txt file");
        for line in contents.lines() {
            let line = line.trim();
            // Skip comments and empty lines
            if !line.starts_with('#') && !line.is_empty() {
                wizard_ids.push(line.to_string());
            }
        }
    } else {
        tracing::warn!("No wizards configured (no WIZARDS env var or wizards.txt file)");
        return Ok(());
    }

    if wizard_ids.is_empty() {
        tracing::warn!("Wizards list is empty");
        return Ok(());
    }

    // Promote wizards to level 50
    let player_repo = PlayerRepository::new(pool.clone());
    for wizard_id in &wizard_ids {
        match promote_to_wizard(&player_repo, wizard_id).await {
            Ok(_) => tracing::info!("Promoted {} to wizard (level 50)", wizard_id),
            Err(e) => tracing::error!("Failed to promote {} to wizard: {}", wizard_id, e),
        }
    }

    tracing::info!("Loaded {} wizard(s)", wizard_ids.len());
    Ok(())
}

/// Promote a player to wizard level (50)
async fn promote_to_wizard(player_repo: &db::player::PlayerRepository, slack_user_id: &str) -> Result<()> {
    // Check if player exists
    if let Some(mut player) = player_repo.get_by_slack_id(slack_user_id).await? {
        // Update level to 50 if not already
        if player.level < 50 {
            player.level = 50;
            player_repo.update(&player).await?;
            tracing::info!("Updated {}'s level to 50", player.name);
        }
    } else {
        // Player doesn't exist yet - they'll be promoted when they first join
        tracing::info!("Wizard {} not in database yet (will be promoted on first login)", slack_user_id);
    }
    Ok(())
}

/// Load default area files (like Midgaard) on startup
async fn load_default_areas(pool: &sqlx::PgPool) -> Result<()> {
    use db::area::AreaRepository;
    use db::room::RoomRepository;
    use db::exit::ExitRepository;
    use db::object::{ObjectRepository, ObjectInstanceRepository};
    use area::parser::parse_area_file;
    use area::types::Reset;
    use models::{Room, Exit, Area, Object, ObjectInstance};

    let area_repo = AreaRepository::new(pool.clone());
    let room_repo = RoomRepository::new(pool.clone());
    let exit_repo = ExitRepository::new(pool.clone());
    let object_repo = ObjectRepository::new(pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(pool.clone());

    // Embed the midgaard.are file directly in the binary
    const MIDGAARD_CONTENT: &str = include_str!("../data/areas/midgaard.are");
    let content = MIDGAARD_CONTENT;

    // Parse the area file
    let area_file = parse_area_file(&content)
        .context("Failed to parse midgaard.are")?;

    let area_name = &area_file.header.name;

    // Check for development mode - force reimport if enabled
    let force_reimport = std::env::var("FORCE_REIMPORT_AREAS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if force_reimport {
        tracing::warn!("FORCE_REIMPORT_AREAS is enabled - deleting and reimporting area '{}'", area_name);

        // Delete existing area and all associated data
        if area_repo.exists(area_name).await? {
            // Delete object instances first
            sqlx::query(
                "DELETE FROM object_instances WHERE object_vnum IN
                 (SELECT vnum FROM objects WHERE area_name = $1)"
            )
            .bind(area_name)
            .execute(pool)
            .await?;

            // Delete object definitions
            sqlx::query("DELETE FROM objects WHERE area_name = $1")
                .bind(area_name)
                .execute(pool)
                .await?;

            // Delete area (cascades to rooms and exits)
            area_repo.delete_by_name(area_name).await?;

            tracing::info!("Deleted existing area '{}' and all associated data", area_name);
        }
    } else {
        // Normal production behavior - skip if already imported
        if area_repo.exists(area_name).await? {
            tracing::info!("Area '{}' already imported, skipping", area_name);
            return Ok(());
        }
    }

    tracing::info!("Importing area '{}' ({} rooms, {} objects, {} resets)...",
        area_name, area_file.rooms.len(), area_file.objects.len(), area_file.resets.len());

    let mut rooms_created = 0;
    let mut exits_created = 0;
    let mut objects_created = 0;
    let mut instances_spawned = 0;

    // First pass: Create all rooms
    for area_room in &area_file.rooms {
        let room_id = format!("vnum_{}", area_room.vnum);

        let room = Room {
            channel_id: room_id.clone(),
            channel_name: area_room.name.clone(),
            description: area_room.description.clone(),
            attached_channel_id: None, // Virtual room
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
                None, // System-created exit
            );

            exit_repo.create(&exit).await?;
            exits_created += 1;
        }
    }

    // Third pass: Create all objects
    for area_object in &area_file.objects {
        let object = Object::new(
            area_object.vnum,
            area_name.clone(),
            area_object.keywords.clone(),
            area_object.short_description.clone(),
            area_object.long_description.clone(),
            area_object.material.clone(),
            area_object.item_type.clone(),
            area_object.extra_flags.clone(),
            area_object.wear_flags.clone(),
            area_object.value0,
            area_object.value1,
            area_object.value2.clone(),
            area_object.value3,
            area_object.value4,
            area_object.weight,
            area_object.cost,
            area_object.level,
            area_object.condition.clone(),
        );

        object_repo.create(&object).await?;
        objects_created += 1;
    }

    // Fourth pass: Process resets and spawn object instances
    for reset in &area_file.resets {
        match reset {
            Reset::ObjectInRoom { obj_vnum, room_vnum, .. } => {
                // Spawn object in room
                let room_id = format!("vnum_{}", room_vnum);

                // Skip if room doesn't exist (outside area range)
                if *room_vnum < area_file.header.min_vnum || *room_vnum > area_file.header.max_vnum {
                    continue;
                }

                // Create object instance
                let instance = ObjectInstance::new_in_room(*obj_vnum, room_id);
                object_instance_repo.create(&instance).await?;
                instances_spawned += 1;
            }
            _ => {
                // Skip other reset types for now (mobs, give, equip, etc.)
                // We'll implement these when we have mobs
            }
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

    tracing::info!(
        "Successfully imported area '{}': {} rooms, {} exits, {} objects, {} instances spawned",
        area_name,
        rooms_created,
        exits_created,
        objects_created,
        instances_spawned
    );

    Ok(())
}
