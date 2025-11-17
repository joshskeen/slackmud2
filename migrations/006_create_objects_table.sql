-- Create objects table (stores object definitions/templates from .are files)
CREATE TABLE IF NOT EXISTS objects (
    id SERIAL PRIMARY KEY,
    vnum INTEGER NOT NULL UNIQUE,
    area_name TEXT NOT NULL,
    keywords TEXT NOT NULL,
    short_description TEXT NOT NULL,
    long_description TEXT NOT NULL,
    material TEXT NOT NULL,
    item_type TEXT NOT NULL,
    extra_flags TEXT NOT NULL DEFAULT '',
    wear_flags TEXT NOT NULL DEFAULT '',
    value0 INTEGER NOT NULL DEFAULT 0,
    value1 INTEGER NOT NULL DEFAULT 0,
    value2 TEXT NOT NULL DEFAULT '',
    value3 INTEGER NOT NULL DEFAULT 0,
    value4 INTEGER NOT NULL DEFAULT 0,
    weight INTEGER NOT NULL DEFAULT 0,
    cost INTEGER NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 0,
    condition TEXT NOT NULL DEFAULT 'P',
    extra_descriptions JSONB DEFAULT '[]',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

-- Create index on vnum for fast lookups
CREATE INDEX IF NOT EXISTS idx_objects_vnum ON objects(vnum);

-- Create index on area_name for area-based queries
CREATE INDEX IF NOT EXISTS idx_objects_area_name ON objects(area_name);
