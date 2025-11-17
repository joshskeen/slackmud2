use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Exit {
    pub id: i32,
    pub from_room_id: String,
    pub direction: String,
    pub to_room_id: String,
    pub created_at: i64,
    pub created_by: Option<String>,
}

impl Exit {
    pub fn new(from_room_id: String, direction: String, to_room_id: String, created_by: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database
            from_room_id,
            direction,
            to_room_id,
            created_at: now,
            created_by,
        }
    }
}

/// Valid directions for exits
pub const VALID_DIRECTIONS: &[&str] = &["north", "south", "east", "west", "up", "down"];

pub fn is_valid_direction(direction: &str) -> bool {
    VALID_DIRECTIONS.contains(&direction.to_lowercase().as_str())
}
