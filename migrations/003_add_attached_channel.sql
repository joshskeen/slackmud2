-- Add attached_channel_id to rooms table
-- This allows rooms to be virtual (not tied to a channel) or attached to a Slack channel for viewing

ALTER TABLE rooms
ADD COLUMN IF NOT EXISTS attached_channel_id TEXT;

-- For existing rooms, auto-attach them to their channel_id
-- This maintains backward compatibility
UPDATE rooms
SET attached_channel_id = channel_id
WHERE attached_channel_id IS NULL;
