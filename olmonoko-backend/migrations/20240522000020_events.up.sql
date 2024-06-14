CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    event_source_id INTEGER REFERENCES ics_sources(id) ON DELETE CASCADE NOT NULL,
    -- Event data
    dt_stamp INTEGER,
    duration INTEGER, -- in milliseconds
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    uid TEXT NOT NULL
);
