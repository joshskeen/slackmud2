use crate::models::Exit;
use sqlx::PgPool;

pub struct ExitRepository {
    pool: PgPool,
}

impl ExitRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, exit: &Exit) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO exits (from_room_id, direction, to_room_id, created_at, created_by)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&exit.from_room_id)
        .bind(&exit.direction)
        .bind(&exit.to_room_id)
        .bind(exit.created_at)
        .bind(&exit.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_exits_from_room(&self, room_id: &str) -> Result<Vec<Exit>, sqlx::Error> {
        sqlx::query_as::<_, Exit>(
            "SELECT * FROM exits WHERE from_room_id = $1 ORDER BY direction"
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_exit_in_direction(&self, room_id: &str, direction: &str) -> Result<Option<Exit>, sqlx::Error> {
        sqlx::query_as::<_, Exit>(
            "SELECT * FROM exits WHERE from_room_id = $1 AND direction = $2"
        )
        .bind(room_id)
        .bind(direction)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete_exit(&self, room_id: &str, direction: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM exits WHERE from_room_id = $1 AND direction = $2")
            .bind(room_id)
            .bind(direction)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
