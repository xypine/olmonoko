CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    email VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    admin BOOLEAN DEFAULT FALSE NOT NULL,
    created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL
);
