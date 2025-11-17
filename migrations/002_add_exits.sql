-- Add exits table for room connections

CREATE TABLE IF NOT EXISTS exits (
    id SERIAL PRIMARY KEY,
    from_room_id TEXT NOT NULL REFERENCES rooms(channel_id) ON DELETE CASCADE,
    direction TEXT NOT NULL CHECK (direction IN ('north', 'south', 'east', 'west', 'up', 'down')),
    to_room_id TEXT NOT NULL REFERENCES rooms(channel_id) ON DELETE CASCADE,
    created_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT,
    created_by TEXT NOT NULL REFERENCES players(slack_user_id),
    UNIQUE(from_room_id, direction)
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_exits_from_room ON exits(from_room_id);
CREATE INDEX IF NOT EXISTS idx_exits_to_room ON exits(to_room_id);
