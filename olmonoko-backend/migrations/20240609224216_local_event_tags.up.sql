CREATE TABLE event_tags (
    local_event_id INTEGER REFERENCES local_events(id) ON DELETE CASCADE,
    remote_event_id INTEGER REFERENCES events(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    created_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL,

    -- either local_event_id or remote_event_id must be set
    CHECK ((local_event_id IS NOT NULL AND remote_event_id IS NULL) OR (remote_event_id IS NOT NULL AND local_event_id IS NULL)),
    UNIQUE (local_event_id, remote_event_id, tag)
);
