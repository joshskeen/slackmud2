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

    /// Get armor class (AC) bonus from armor
    /// Returns total AC (sum of pierce, bash, slash, magic)
    pub fn get_armor_class(&self) -> i32 {
        if self.item_type.to_lowercase() != "armor" {
            return 0;
        }
        // Sum all AC values for total armor class
        self.value0 + self.value1 + self.value3 // value0=pierce, value1=bash, value3=magic
    }

    /// Get weapon damage dice (e.g., "2d4" for 2 dice of 4 sides)
    pub fn get_weapon_damage(&self) -> Option<String> {
        if self.item_type.to_lowercase() != "weapon" {
            return None;
        }
        let num_dice = self.value1;
        let dice_size = self.value2.parse::<i32>().unwrap_or(0);
        if num_dice > 0 && dice_size > 0 {
            Some(format!("{}d{}", num_dice, dice_size))
        } else {
            None
        }
    }

    /// Get average weapon damage
    pub fn get_avg_weapon_damage(&self) -> f32 {
        if self.item_type.to_lowercase() != "weapon" {
            return 0.0;
        }
        let num_dice = self.value1 as f32;
        let dice_size = self.value2.parse::<f32>().unwrap_or(0.0);
        if num_dice > 0.0 && dice_size > 0.0 {
            num_dice * (dice_size + 1.0) / 2.0
        } else {
            0.0
        }
    }

    /// Get damage type (for weapons)
    pub fn get_damage_type(&self) -> Option<String> {
        if self.item_type.to_lowercase() != "weapon" {
            return None;
        }
        // value3 is damage type in ROM
        let damage_type = match self.value3 {
            0 => "hit",
            1 => "slice",
            2 => "stab",
            3 => "slash",
            4 => "whip",
            5 => "claw",
            6 => "blast",
            7 => "pound",
            8 => "crush",
            9 => "grep",
            10 => "bite",
            11 => "pierce",
            12 => "suction",
            _ => "hit",
        };
        Some(damage_type.to_string())
    }

    /// Check if player meets level requirement for this item
    pub fn can_use(&self, player_level: i32) -> bool {
        player_level >= self.level
    }

    /// Get a formatted stat summary for this item
    pub fn get_stat_summary(&self) -> String {
        let mut stats = Vec::new();

        match self.item_type.to_lowercase().as_str() {
            "armor" => {
                let ac = self.get_armor_class();
                if ac > 0 {
                    stats.push(format!("AC: {}", ac));
                }
            }
            "weapon" => {
                if let Some(damage) = self.get_weapon_damage() {
                    stats.push(format!("Damage: {}", damage));
                    let avg = self.get_avg_weapon_damage();
                    stats.push(format!("Average: {:.1}", avg));
                }
                if let Some(dmg_type) = self.get_damage_type() {
                    stats.push(format!("Type: {}", dmg_type));
                }
            }
            _ => {}
        }

        if self.level > 0 {
            stats.push(format!("Level: {}", self.level));
        }

        if stats.is_empty() {
            String::new()
        } else {
            stats.join(", ")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ObjectInstance {
    pub id: i32,
    pub object_vnum: i32,
    pub location_type: String, // 'room', 'player', 'container', 'equipped'
    pub location_id: String,
    pub wear_location: Option<String>,
    pub equipped_slot: Option<String>, // 'body', 'wield', 'finger_l', etc.
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
            equipped_slot: None,
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
            equipped_slot: None,
            current_condition: 100,
            timer: None,
            created_at: now,
            updated_at: now,
        }
    }
}
