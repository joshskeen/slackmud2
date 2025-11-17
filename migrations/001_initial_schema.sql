-- Initial schema for SlackMUD

-- Classes table
CREATE TABLE IF NOT EXISTS classes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL
);

-- Races table
CREATE TABLE IF NOT EXISTS races (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL
);

-- Players table
CREATE TABLE IF NOT EXISTS players (
    slack_user_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 1,
    experience_points INTEGER NOT NULL DEFAULT 0,
    class_id INTEGER,
    race_id INTEGER,
    gender TEXT,
    current_channel_id TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (class_id) REFERENCES classes(id),
    FOREIGN KEY (race_id) REFERENCES races(id)
);

-- Rooms/Channels table
CREATE TABLE IF NOT EXISTS rooms (
    channel_id TEXT PRIMARY KEY,
    channel_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT 'A mysterious room in the Slack workspace.',
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Insert default classes
INSERT INTO classes (name, description) VALUES
    ('Warrior', 'A fierce fighter skilled in melee combat and defense.'),
    ('Mage', 'A wielder of arcane magic, casting powerful spells.'),
    ('Rogue', 'A stealthy character adept at sneaking and quick strikes.'),
    ('Cleric', 'A holy warrior who can heal allies and smite foes.');

-- Insert default races
INSERT INTO races (name, description) VALUES
    ('Human', 'Versatile and adaptable, humans excel in all paths.'),
    ('Elf', 'Graceful and long-lived, with affinity for magic.'),
    ('Dwarf', 'Sturdy and resilient, masters of crafting and combat.'),
    ('Halfling', 'Small and nimble, with a knack for avoiding danger.');
