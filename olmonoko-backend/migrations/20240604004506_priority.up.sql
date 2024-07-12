CREATE TABLE ics_source_priorities (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ics_source_id INTEGER NOT NULL REFERENCES ics_sources(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL,
    PRIMARY KEY (user_id, ics_source_id)
);

ALTER TABLE local_events ADD COLUMN priority INTEGER;
