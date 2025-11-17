use crate::models::Area;
use sqlx::PgPool;

pub struct AreaRepository {
    pool: PgPool,
}

impl AreaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<Area>, sqlx::Error> {
        sqlx::query_as::<_, Area>("SELECT * FROM areas WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create(&self, area: &Area) -> Result<Area, sqlx::Error> {
        sqlx::query_as::<_, Area>(
            "INSERT INTO areas (name, filename, min_vnum, max_vnum, rooms_count, exits_count, imported_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING *"
        )
        .bind(&area.name)
        .bind(&area.filename)
        .bind(area.min_vnum)
        .bind(area.max_vnum)
        .bind(area.rooms_count)
        .bind(area.exits_count)
        .bind(area.imported_at)
        .bind(area.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(&self, area: &Area) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE areas
             SET filename = $2, min_vnum = $3, max_vnum = $4,
                 rooms_count = $5, exits_count = $6, updated_at = $7
             WHERE name = $1"
        )
        .bind(&area.name)
        .bind(&area.filename)
        .bind(area.min_vnum)
        .bind(area.max_vnum)
        .bind(area.rooms_count)
        .bind(area.exits_count)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete an area and all its associated rooms/exits
    pub async fn delete_by_name(&self, name: &str) -> Result<(), sqlx::Error> {
        // Get the area first to know the vnum range
        if let Some(area) = self.get_by_name(name).await? {
            // Delete all rooms in this vnum range
            sqlx::query("DELETE FROM rooms WHERE channel_id LIKE 'vnum_%' AND CAST(SUBSTRING(channel_id FROM 6) AS INTEGER) BETWEEN $1 AND $2")
                .bind(area.min_vnum)
                .bind(area.max_vnum)
                .execute(&self.pool)
                .await?;

            // Delete the area record
            sqlx::query("DELETE FROM areas WHERE name = $1")
                .bind(name)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn exists(&self, name: &str) -> Result<bool, sqlx::Error> {
        let result: Option<(bool,)> = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM areas WHERE name = $1)")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(result.map(|(exists,)| exists).unwrap_or(false))
    }
}
