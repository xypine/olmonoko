CREATE TABLE local_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,
    updated_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,
    -- Event data
    -- local events have no occurrences, instead starts_at is directly stored in the local_events table
    starts_at INTEGER NOT NULL, -- in milliseconds
    duration INTEGER, -- in milliseconds
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    uid TEXT NOT NULL
);
