ALTER TABLE public_calendar_links RENAME TO old_calendar_links;

CREATE TABLE public_calendar_links (
    id TEXT PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL,
    min_priority INTEGER,
    max_priority INTEGER
);

INSERT INTO public_calendar_links SELECT * FROM old_calendar_links;

DROP TABLE old_calendar_links;
