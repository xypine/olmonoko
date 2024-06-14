CREATE TABLE ics_source_priorities (
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    ics_source_id INTEGER REFERENCES ics_sources(id) ON DELETE CASCADE NOT NULL,
    priority INTEGER NOT NULL,
    PRIMARY KEY (user_id, ics_source_id)
);

ALTER TABLE local_events ADD COLUMN priority INTEGER;
