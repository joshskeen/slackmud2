-- Create object_instances table (stores actual spawned objects in the game)
CREATE TABLE IF NOT EXISTS object_instances (
    id SERIAL PRIMARY KEY,
    object_vnum INTEGER NOT NULL,
    location_type TEXT NOT NULL, -- 'room', 'player', 'container', 'equipped'
    location_id TEXT NOT NULL, -- room channel_id, player slack_user_id, or container instance id
    wear_location TEXT, -- if equipped: 'wielded', 'worn_head', 'worn_body', etc.
    current_condition INTEGER NOT NULL DEFAULT 100, -- current durability/condition
    timer INTEGER, -- for timed objects (food decay, etc.)
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY (object_vnum) REFERENCES objects(vnum) ON DELETE CASCADE
);

-- Create index on object_vnum for lookups by object type
CREATE INDEX IF NOT EXISTS idx_object_instances_vnum ON object_instances(object_vnum);

-- Create index on location for finding objects in a room/player/container
CREATE INDEX IF NOT EXISTS idx_object_instances_location ON object_instances(location_type, location_id);

-- Create index on equipped items for a player
CREATE INDEX IF NOT EXISTS idx_object_instances_equipped ON object_instances(location_type, location_id, wear_location)
WHERE location_type = 'equipped';
