-- delete any existing duplicate email entries
DELETE FROM users WHERE id NOT IN (
    SELECT MIN(id) FROM users GROUP BY email
);
-- make emails unique
CREATE UNIQUE INDEX users_unique_email ON users (email);

-- create a new table for unverified users
CREATE TABLE unverified_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    admin BOOLEAN DEFAULT FALSE NOT NULL,
    secret VARCHAR(255) NOT NULL UNIQUE,
    created_at INTEGER DEFAULT (strftime('%s', 'now')) NOT NULL
);
