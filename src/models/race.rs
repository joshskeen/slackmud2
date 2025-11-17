use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Race {
    pub id: i32,
    pub name: String,
    pub description: String,
}
