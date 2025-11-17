-- Add equipment slot tracking to object instances
-- This allows objects to be equipped in specific body locations

-- Add column if it doesn't exist (idempotent)
ALTER TABLE object_instances
ADD COLUMN IF NOT EXISTS equipped_slot TEXT;

-- equipped_slot values:
-- NULL = not equipped (in inventory or room)
-- 'light' = used as light source
-- 'finger_l', 'finger_r' = left/right finger (rings)
-- 'neck_1', 'neck_2' = neck slots (amulets, necklaces)
-- 'body' = torso armor
-- 'head' = helmet
-- 'legs' = leg armor
-- 'feet' = boots
-- 'hands' = gloves
-- 'arms' = arm armor
-- 'shield' = shield (off-hand)
-- 'about' = cloak/cape
-- 'waist' = belt
-- 'wrist_l', 'wrist_r' = left/right wrist (bracers)
-- 'wield' = primary weapon
-- 'hold' = held item (off-hand)
-- 'float' = floating nearby

-- Create index for faster equipment queries (idempotent)
CREATE INDEX IF NOT EXISTS idx_object_instances_equipped ON object_instances(location_id, location_type, equipped_slot)
WHERE location_type = 'equipped';
