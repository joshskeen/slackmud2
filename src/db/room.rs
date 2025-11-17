use crate::models::Room;
use sqlx::PgPool;

pub struct RoomRepository {
    pool: PgPool,
}

impl RoomRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_channel_id(&self, channel_id: &str) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as::<_, Room>("SELECT * FROM rooms WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create(&self, room: &Room) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO rooms (channel_id, channel_name, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&room.channel_id)
        .bind(&room.channel_name)
        .bind(&room.description)
        .bind(room.created_at)
        .bind(room.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_description(&self, channel_id: &str, description: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE rooms SET description = $1, updated_at = $2 WHERE channel_id = $3")
            .bind(description)
            .bind(now)
            .bind(channel_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_or_create(&self, channel_id: String, channel_name: String) -> Result<Room, sqlx::Error> {
        if let Some(room) = self.get_by_channel_id(&channel_id).await? {
            Ok(room)
        } else {
            let room = Room::new(channel_id, channel_name);
            self.create(&room).await?;
            Ok(room)
        }
    }
}
