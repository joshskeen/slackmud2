use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Room {
    pub channel_id: String,
    pub channel_name: String,
    pub description: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Room {
    pub fn new(channel_id: String, channel_name: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            channel_id,
            channel_name,
            description: "A mysterious room in the Slack workspace.".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}
