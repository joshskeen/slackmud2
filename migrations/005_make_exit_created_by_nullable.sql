-- Make created_by nullable for system-created exits
ALTER TABLE exits
ALTER COLUMN created_by DROP NOT NULL;
