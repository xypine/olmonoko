ALTER TABLE events ADD COLUMN rrule TEXT;
ALTER TABLE event_occurrences ADD COLUMN from_rrule BOOLEAN DEFAULT FALSE NOT NULL;
CREATE UNIQUE INDEX events_uid ON events(event_source_id, uid, coalesce(rrule, ''));
