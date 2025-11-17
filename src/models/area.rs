use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Area {
    pub id: i32,
    pub name: String,
    pub filename: String,
    pub min_vnum: i32,
    pub max_vnum: i32,
    pub rooms_count: i32,
    pub exits_count: i32,
    pub imported_at: i64,
    pub updated_at: i64,
}

impl Area {
    pub fn new(
        name: String,
        filename: String,
        min_vnum: i32,
        max_vnum: i32,
        rooms_count: i32,
        exits_count: i32,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database
            name,
            filename,
            min_vnum,
            max_vnum,
            rooms_count,
            exits_count,
            imported_at: now,
            updated_at: now,
        }
    }
}
