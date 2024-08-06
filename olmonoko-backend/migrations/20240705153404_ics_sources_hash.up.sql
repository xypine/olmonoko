ALTER TABLE ics_sources ADD COLUMN file_hash TEXT;
ALTER TABLE ics_sources ADD COLUMN object_hash TEXT;
ALTER TABLE ics_sources ADD COLUMN updated_at BIGINT;
