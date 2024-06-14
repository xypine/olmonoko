CREATE TABLE event_tags (
    local_event_id INTEGER REFERENCES local_events(id) ON DELETE CASCADE,
    remote_event_id INTEGER REFERENCES events(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,

    -- either local_event_id or remote_event_id must be set
    CHECK ((local_event_id IS NOT NULL and remote_event_id IS NULL) OR (remote_event_id IS NOT NULL and local_event_id IS NULL)),
    PRIMARY KEY (local_event_id, remote_event_id, tag)
);
