use crate::models::{Object, ObjectInstance};
use sqlx::{PgPool, Row};

pub struct ObjectRepository {
    pool: PgPool,
}

impl ObjectRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Create a new object definition
    pub async fn create(&self, object: &Object) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO objects (
                vnum, area_name, keywords, short_description, long_description,
                material, item_type, extra_flags, wear_flags,
                value0, value1, value2, value3, value4,
                weight, cost, level, condition, extra_descriptions,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            ON CONFLICT (vnum) DO UPDATE SET
                area_name = EXCLUDED.area_name,
                keywords = EXCLUDED.keywords,
                short_description = EXCLUDED.short_description,
                long_description = EXCLUDED.long_description,
                material = EXCLUDED.material,
                item_type = EXCLUDED.item_type,
                extra_flags = EXCLUDED.extra_flags,
                wear_flags = EXCLUDED.wear_flags,
                value0 = EXCLUDED.value0,
                value1 = EXCLUDED.value1,
                value2 = EXCLUDED.value2,
                value3 = EXCLUDED.value3,
                value4 = EXCLUDED.value4,
                weight = EXCLUDED.weight,
                cost = EXCLUDED.cost,
                level = EXCLUDED.level,
                condition = EXCLUDED.condition,
                extra_descriptions = EXCLUDED.extra_descriptions,
                updated_at = EXCLUDED.updated_at"
        )
        .bind(object.vnum)
        .bind(&object.area_name)
        .bind(&object.keywords)
        .bind(&object.short_description)
        .bind(&object.long_description)
        .bind(&object.material)
        .bind(&object.item_type)
        .bind(&object.extra_flags)
        .bind(&object.wear_flags)
        .bind(object.value0)
        .bind(object.value1)
        .bind(&object.value2)
        .bind(object.value3)
        .bind(object.value4)
        .bind(object.weight)
        .bind(object.cost)
        .bind(object.level)
        .bind(&object.condition)
        .bind(&object.extra_descriptions)
        .bind(object.created_at)
        .bind(object.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get an object definition by vnum
    pub async fn get_by_vnum(&self, vnum: i32) -> Result<Option<Object>, sqlx::Error> {
        sqlx::query_as::<_, Object>("SELECT * FROM objects WHERE vnum = $1")
            .bind(vnum)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get all objects for an area
    pub async fn get_by_area(&self, area_name: &str) -> Result<Vec<Object>, sqlx::Error> {
        sqlx::query_as::<_, Object>("SELECT * FROM objects WHERE area_name = $1 ORDER BY vnum")
            .bind(area_name)
            .fetch_all(&self.pool)
            .await
    }

    /// Delete all objects for an area
    pub async fn delete_by_area(&self, area_name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM objects WHERE area_name = $1")
            .bind(area_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

pub struct ObjectInstanceRepository {
    pool: PgPool,
}

impl ObjectInstanceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new object instance
    pub async fn create(&self, instance: &ObjectInstance) -> Result<i32, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO object_instances (
                object_vnum, location_type, location_id, wear_location,
                current_condition, timer, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id"
        )
        .bind(instance.object_vnum)
        .bind(&instance.location_type)
        .bind(&instance.location_id)
        .bind(&instance.wear_location)
        .bind(instance.current_condition)
        .bind(instance.timer)
        .bind(instance.created_at)
        .bind(instance.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Get all object instances in a room
    pub async fn get_in_room(&self, room_channel_id: &str) -> Result<Vec<ObjectInstance>, sqlx::Error> {
        sqlx::query_as::<_, ObjectInstance>(
            "SELECT * FROM object_instances WHERE location_type = 'room' AND location_id = $1"
        )
        .bind(room_channel_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get all object instances in a player's inventory
    pub async fn get_in_player_inventory(&self, player_slack_id: &str) -> Result<Vec<ObjectInstance>, sqlx::Error> {
        sqlx::query_as::<_, ObjectInstance>(
            "SELECT * FROM object_instances WHERE location_type = 'player' AND location_id = $1"
        )
        .bind(player_slack_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get all equipped objects for a player
    pub async fn get_equipped(&self, player_slack_id: &str) -> Result<Vec<ObjectInstance>, sqlx::Error> {
        sqlx::query_as::<_, ObjectInstance>(
            "SELECT * FROM object_instances WHERE location_type = 'equipped' AND location_id = $1"
        )
        .bind(player_slack_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update object instance location
    pub async fn update_location(
        &self,
        instance_id: i32,
        location_type: &str,
        location_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE object_instances SET location_type = $1, location_id = $2, updated_at = $3 WHERE id = $4"
        )
        .bind(location_type)
        .bind(location_id)
        .bind(chrono::Utc::now().timestamp())
        .bind(instance_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete an object instance
    pub async fn delete(&self, instance_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM object_instances WHERE id = $1")
            .bind(instance_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete all object instances in a room
    pub async fn delete_in_room(&self, room_channel_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM object_instances WHERE location_type = 'room' AND location_id = $1")
            .bind(room_channel_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Equip an item in a specific slot
    pub async fn equip_item(
        &self,
        instance_id: i32,
        player_slack_id: &str,
        equipped_slot: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE object_instances
             SET location_type = 'equipped', location_id = $1, equipped_slot = $2, updated_at = $3
             WHERE id = $4"
        )
        .bind(player_slack_id)
        .bind(equipped_slot)
        .bind(now)
        .bind(instance_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Unequip an item (move from equipped to inventory)
    pub async fn unequip_item(
        &self,
        instance_id: i32,
        player_slack_id: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE object_instances
             SET location_type = 'player', location_id = $1, equipped_slot = NULL, updated_at = $2
             WHERE id = $3"
        )
        .bind(player_slack_id)
        .bind(now)
        .bind(instance_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get item in a specific equipment slot for a player
    pub async fn get_item_in_slot(
        &self,
        player_slack_id: &str,
        equipped_slot: &str,
    ) -> Result<Option<ObjectInstance>, sqlx::Error> {
        sqlx::query_as::<_, ObjectInstance>(
            "SELECT * FROM object_instances
             WHERE location_type = 'equipped' AND location_id = $1 AND equipped_slot = $2"
        )
        .bind(player_slack_id)
        .bind(equipped_slot)
        .fetch_optional(&self.pool)
        .await
    }
}
