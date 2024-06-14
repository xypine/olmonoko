DROP TABLE public_calendar_links;

CREATE TABLE public_calendar_links
(
	id TEXT PRIMARY KEY NOT NULL,
	user_id INTEGER REFERENCES users(id) ON DELETE CASCADE NOT NULL UNIQUE,
	created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL,
);
