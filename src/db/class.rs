use crate::models::Class;
use sqlx::PgPool;

pub struct ClassRepository {
    pool: PgPool,
}

impl ClassRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Option<Class>, sqlx::Error> {
        sqlx::query_as::<_, Class>("SELECT * FROM classes WHERE id = $1")
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
        sqlx::query_as::<_, Class>("SELECT * FROM classes WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
    }
}
