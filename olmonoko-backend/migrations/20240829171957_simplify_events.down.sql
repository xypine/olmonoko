CREATE TABLE attendance (
    id SERIAL PRIMARY KEY,

    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    local_event_id INTEGER REFERENCES local_events(id) ON DELETE CASCADE,
    remote_event_id INTEGER REFERENCES events(id) ON DELETE CASCADE,

    planned BOOLEAN DEFAULT FALSE NOT NULL,
    planned_starts_at BIGINT,
    planned_duration INTEGER,

    actual BOOLEAN DEFAULT FALSE NOT NULL,
    actual_starts_at BIGINT,
    actual_duration INTEGER,

    created_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL,
    updated_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL,
    -- either local_event_id or remote_event_id must be set
    CHECK ((local_event_id IS NOT NULL AND remote_event_id IS NULL) OR (remote_event_id IS NOT NULL AND local_event_id IS NULL)),
    -- either planned or actual must be set
    CHECK (planned IS TRUE OR actual IS TRUE),

    -- if planned_starts_at or planned_duration are set, planned must be set
    CHECK ((planned_starts_at IS NULL AND planned_duration IS NULL) OR (planned IS TRUE)),
    -- if actual_starts_at or actual_duration are set, actual must be set
    CHECK ((actual_starts_at IS NULL AND actual_duration IS NULL) OR (actual IS TRUE)),
    UNIQUE (user_id, local_event_id, remote_event_id)
);

ALTER TABLE attendance DROP COLUMN planned_starts_at;
ALTER TABLE attendance DROP COLUMN planned_duration;

ALTER TABLE attendance DROP COLUMN actual_starts_at;
ALTER TABLE attendance DROP COLUMN actual_duration;

CREATE UNIQUE INDEX attendance_event_id ON attendance(user_id, coalesce(local_event_id, -1), coalesce(remote_event_id, -1));

ALTER TABLE event_tags ADD COLUMN remote_event_id INTEGER;
ALTER TABLE bills ADD COLUMN remote_event_id INTEGER;
ALTER TABLE event_tags ALTER COLUMN local_event_id DROP NOT NULL;
ALTER TABLE bills ALTER COLUMN local_event_id DROP NOT NULL;

ALTER TABLE local_events
DROP COLUMN auto_imported,
DROP COLUMN attendance_planned,
DROP COLUMN attendance_actual;
ALTER TABLE local_events ADD COLUMN rrule TEXT;

DROP TABLE IF EXISTS remote_local_link;
