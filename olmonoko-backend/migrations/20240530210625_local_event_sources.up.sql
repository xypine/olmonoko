CREATE TABLE local_events (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL,
    updated_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL,
    -- Event data
    -- local events have no occurrences, instead starts_at is directly stored in the local_events table
    starts_at BIGINT NOT NULL, -- in seconds since epoch
    duration INTEGER, -- in seconds
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    uid TEXT NOT NULL
);
