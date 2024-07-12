CREATE TABLE event_occurrences (
    id SERIAL PRIMARY KEY,
    event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    starts_at BIGINT NOT NULL
);
