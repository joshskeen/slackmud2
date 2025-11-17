use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Object {
    pub id: i32,
    pub vnum: i32,
    pub area_name: String,
    pub keywords: String,
    pub short_description: String,
    pub long_description: String,
    pub material: String,
    pub item_type: String,
    pub extra_flags: String,
    pub wear_flags: String,
    pub value0: i32,
    pub value1: i32,
    pub value2: String,
    pub value3: i32,
    pub value4: i32,
    pub weight: i32,
    pub cost: i32,
    pub level: i32,
    pub condition: String,
    pub extra_descriptions: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Object {
    pub fn new(
        vnum: i32,
        area_name: String,
        keywords: String,
        short_description: String,
        long_description: String,
        material: String,
        item_type: String,
        extra_flags: String,
        wear_flags: String,
        value0: i32,
        value1: i32,
        value2: String,
        value3: i32,
        value4: i32,
        weight: i32,
        cost: i32,
        level: i32,
        condition: String,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database
            vnum,
            area_name,
            keywords,
            short_description,
            long_description,
            material,
            item_type,
            extra_flags,
            wear_flags,
            value0,
            value1,
            value2,
            value3,
            value4,
            weight,
            cost,
            level,
            condition,
            extra_descriptions: serde_json::json!([]),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the first keyword (used for matching player commands)
    pub fn primary_keyword(&self) -> &str {
        self.keywords.split_whitespace().next().unwrap_or(&self.keywords)
    }

    /// Check if this object matches a keyword
    pub fn matches_keyword(&self, keyword: &str) -> bool {
        self.keywords
            .split_whitespace()
            .any(|k| k.eq_ignore_ascii_case(keyword))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ObjectInstance {
    pub id: i32,
    pub object_vnum: i32,
    pub location_type: String, // 'room', 'player', 'container', 'equipped'
    pub location_id: String,
    pub wear_location: Option<String>,
    pub current_condition: i32,
    pub timer: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ObjectInstance {
    pub fn new_in_room(object_vnum: i32, room_channel_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database
            object_vnum,
            location_type: "room".to_string(),
            location_id: room_channel_id,
            wear_location: None,
            current_condition: 100,
            timer: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_in_player_inventory(object_vnum: i32, player_slack_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database
            object_vnum,
            location_type: "player".to_string(),
            location_id: player_slack_id,
            wear_location: None,
            current_condition: 100,
            timer: None,
            created_at: now,
            updated_at: now,
        }
    }
}
