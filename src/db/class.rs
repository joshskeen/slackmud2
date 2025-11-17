use crate::models::Class;
use sqlx::SqlitePool;

pub struct ClassRepository {
    pool: SqlitePool,
}

impl ClassRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Option<Class>, sqlx::Error> {
        sqlx::query_as::<_, Class>("SELECT * FROM classes WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_all(&self) -> Result<Vec<Class>, sqlx::Error> {
        sqlx::query_as::<_, Class>("SELECT * FROM classes ORDER BY name")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<Class>, sqlx::Error> {
        sqlx::query_as::<_, Class>("SELECT * FROM classes WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
    }
}
