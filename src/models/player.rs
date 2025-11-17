use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Player {
    pub slack_user_id: String,
    pub name: String,
    pub level: i32,
    pub experience_points: i32,
    pub class_id: Option<i32>,
    pub race_id: Option<i32>,
    pub gender: Option<String>,
    pub current_channel_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Player {
    pub fn new(slack_user_id: String, name: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            slack_user_id,
            name,
            level: 1,
            experience_points: 0,
            class_id: None,
            race_id: None,
            gender: None,
            current_channel_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_character_complete(&self) -> bool {
        self.class_id.is_some() && self.race_id.is_some() && self.gender.is_some()
    }
}
