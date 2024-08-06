ALTER TABLE attendance DROP COLUMN planned_starts_at;
ALTER TABLE attendance DROP COLUMN planned_duration;

ALTER TABLE attendance DROP COLUMN actual_starts_at;
ALTER TABLE attendance DROP COLUMN actual_duration;

CREATE UNIQUE INDEX attendance_event_id ON attendance(user_id, coalesce(local_event_id, -1), coalesce(remote_event_id, -1));
