CREATE TABLE attendance (
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
    PRIMARY KEY (user_id, local_event_id, remote_event_id)
);
CREATE UNIQUE INDEX attendance_user_id_event_id ON attendance(
    user_id,
    COALESCE(local_event_id::TEXT, ''),
    COALESCE(remote_event_id::TEXT, '')
);
