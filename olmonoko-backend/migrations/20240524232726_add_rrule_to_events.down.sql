ALTER TABLE events DROP COLUMN rrule;
ALTER TABLE event_occurrences DROP COLUMN from_rrule;
DROP INDEX IF EXISTS events_uid;
