use crate::models::Player;
use sqlx::SqlitePool;

pub struct PlayerRepository {
    pool: SqlitePool,
}

impl PlayerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_by_slack_id(&self, slack_user_id: &str) -> Result<Option<Player>, sqlx::Error> {
        sqlx::query_as::<_, Player>(
            "SELECT * FROM players WHERE slack_user_id = ?"
        )
        .bind(slack_user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, player: &Player) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO players (slack_user_id, name, level, experience_points, class_id, race_id, gender, current_channel_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&player.slack_user_id)
        .bind(&player.name)
        .bind(player.level)
        .bind(player.experience_points)
        .bind(player.class_id)
        .bind(player.race_id)
        .bind(&player.gender)
        .bind(&player.current_channel_id)
        .bind(player.created_at)
        .bind(player.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update(&self, player: &Player) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE players
             SET name = ?, level = ?, experience_points = ?, class_id = ?, race_id = ?,
                 gender = ?, current_channel_id = ?, updated_at = ?
             WHERE slack_user_id = ?"
        )
        .bind(&player.name)
        .bind(player.level)
        .bind(player.experience_points)
        .bind(player.class_id)
        .bind(player.race_id)
        .bind(&player.gender)
        .bind(&player.current_channel_id)
        .bind(now)
        .bind(&player.slack_user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_current_channel(&self, slack_user_id: &str, channel_id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE players SET current_channel_id = ?, updated_at = ? WHERE slack_user_id = ?"
        )
        .bind(channel_id)
        .bind(now)
        .bind(slack_user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_or_create(&self, slack_user_id: String, name: String) -> Result<Player, sqlx::Error> {
        if let Some(player) = self.get_by_slack_id(&slack_user_id).await? {
            Ok(player)
        } else {
            let player = Player::new(slack_user_id, name);
            self.create(&player).await?;
            Ok(player)
        }
    }
}
