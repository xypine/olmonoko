CREATE TABLE attendance (
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE NOT NULL,

    local_event_id INTEGER REFERENCES local_events(id) ON DELETE CASCADE,
    remote_event_id INTEGER REFERENCES events(id) ON DELETE CASCADE,

    planned BOOLEAN DEFAULT FALSE NOT NULL,
    planned_starts_at INTEGER,
    planned_duration INTEGER,

    actual BOOLEAN DEFAULT FALSE NOT NULL,
    actual_starts_at INTEGER,
    actual_duration INTEGER,

    created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,
    updated_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,
    -- either local_event_id or remote_event_id must be set
    CHECK ((local_event_id IS NOT NULL and remote_event_id IS NULL) OR (remote_event_id IS NOT NULL and local_event_id IS NULL)),
    -- either planned or actual must be set
    -- otherwise, the row should be deleted
    CHECK (planned IS TRUE or actual IS TRUE),
    -- if planned_starts_at or planned_duration are set, planned must be set
    CHECK ((planned_starts_at IS NULL and planned_duration IS NULL) OR (planned IS TRUE)),
    -- if actual_starts_at or actual_duration are set, actual must be set
    CHECK ((actual_starts_at IS NULL and actual_duration IS NULL) OR (actual IS TRUE)),
    PRIMARY KEY (user_id, local_event_id, remote_event_id)
);
CREATE UNIQUE INDEX attendance_user_id_event_id ON attendance(user_id, COALESCE(local_event_id, ""), COALESCE(remote_event_id, ""));
