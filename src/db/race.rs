use crate::models::Race;
use sqlx::SqlitePool;

pub struct RaceRepository {
    pool: SqlitePool,
}

impl RaceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Option<Race>, sqlx::Error> {
        sqlx::query_as::<_, Race>("SELECT * FROM races WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_all(&self) -> Result<Vec<Race>, sqlx::Error> {
        sqlx::query_as::<_, Race>("SELECT * FROM races ORDER BY name")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<Race>, sqlx::Error> {
        sqlx::query_as::<_, Race>("SELECT * FROM races WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
    }
}
