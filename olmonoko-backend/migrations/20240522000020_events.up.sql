CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    event_source_id INTEGER NOT NULL REFERENCES ics_sources(id) ON DELETE CASCADE,
    -- Event data
    dt_stamp BIGINT,
    duration INTEGER, -- in seconds
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    uid TEXT NOT NULL
);
