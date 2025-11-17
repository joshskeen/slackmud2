use crate::models::Race;
use sqlx::PgPool;

pub struct RaceRepository {
    pool: PgPool,
}

impl RaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Option<Race>, sqlx::Error> {
        sqlx::query_as::<_, Race>("SELECT * FROM races WHERE id = $1")
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
        sqlx::query_as::<_, Race>("SELECT * FROM races WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
    }
}
