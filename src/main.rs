mod models;
mod db;
mod slack;
mod handlers;

use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    pub db_pool: SqlitePool,
    pub slack_client: slack::SlackClient,
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
        .unwrap_or_else(|_| "sqlite://slackmud.db".to_string());
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

    // Create Slack client
    let slack_client = slack::SlackClient::new(slack_bot_token);

    // Create shared application state
    let state = Arc::new(AppState {
        db_pool,
        slack_client,
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/slack/commands", post(handlers::handle_slash_command))
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
