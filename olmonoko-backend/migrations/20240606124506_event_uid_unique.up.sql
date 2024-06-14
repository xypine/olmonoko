CREATE UNIQUE INDEX events_unique_uid ON events (event_source_id, uid, coalesce(rrule,""));
