mod models;
mod db;
mod slack;
mod handlers;

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
