-- Create areas table to track imported area files
CREATE TABLE IF NOT EXISTS areas (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    min_vnum INTEGER NOT NULL,
    max_vnum INTEGER NOT NULL,
    rooms_count INTEGER NOT NULL DEFAULT 0,
    exits_count INTEGER NOT NULL DEFAULT 0,
    imported_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

-- Create index on name for quick lookups
CREATE INDEX IF NOT EXISTS idx_areas_name ON areas(name);
