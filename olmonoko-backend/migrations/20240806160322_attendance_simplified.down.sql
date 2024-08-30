ALTER TABLE IF EXISTS attendance ADD COLUMN planned_starts_at BIGINT;
ALTER TABLE IF EXISTS attendance ADD COLUMN planned_duration INTEGER;

ALTER TABLE IF EXISTS attendance ADD COLUMN actual_starts_at BIGINT;
ALTER TABLE IF EXISTS attendance ADD COLUMN actual_duration INTEGER;
DROP INDEX IF EXISTS attendance_event_id;
