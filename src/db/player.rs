use crate::models::Player;
use sqlx::PgPool;

pub struct PlayerRepository {
    pool: PgPool,
}

impl PlayerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_slack_id(&self, slack_user_id: &str) -> Result<Option<Player>, sqlx::Error> {
        sqlx::query_as::<_, Player>(
            "SELECT * FROM players WHERE slack_user_id = $1"
        )
        .bind(slack_user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, player: &Player) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO players (slack_user_id, name, level, experience_points, class_id, race_id, gender, current_channel_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
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
             SET name = $1, level = $2, experience_points = $3, class_id = $4, race_id = $5,
                 gender = $6, current_channel_id = $7, updated_at = $8
             WHERE slack_user_id = $9"
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
            "UPDATE players SET current_channel_id = $1, updated_at = $2 WHERE slack_user_id = $3"
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

    pub async fn get_players_in_room(&self, channel_id: &str) -> Result<Vec<Player>, sqlx::Error> {
        sqlx::query_as::<_, Player>(
            "SELECT * FROM players WHERE current_channel_id = $1 ORDER BY name"
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn is_name_taken(&self, name: &str) -> Result<bool, sqlx::Error> {
        let result: Option<(bool,)> = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM players WHERE LOWER(name) = LOWER($1))"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(exists,)| exists).unwrap_or(false))
    }

    pub async fn delete_all(&self) -> Result<(), sqlx::Error> {
        // Delete in order to respect foreign key constraints
        // 1. Delete player inventory and equipment (object instances owned by players)
        //    Equipment is tracked in object_instances with equipped_slot set
        sqlx::query("DELETE FROM object_instances WHERE owner_id IS NOT NULL")
            .execute(&self.pool)
            .await?;

        // 2. Delete exits created by players
        sqlx::query("DELETE FROM exits WHERE created_by IS NOT NULL")
            .execute(&self.pool)
            .await?;

        // 3. Now safe to delete all players
        sqlx::query("DELETE FROM players")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
