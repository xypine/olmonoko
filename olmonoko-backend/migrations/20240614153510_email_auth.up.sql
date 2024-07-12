-- create a new table for unverified users
CREATE TABLE unverified_users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    admin BOOLEAN DEFAULT FALSE NOT NULL,
    secret VARCHAR(255) NOT NULL UNIQUE,
    created_at BIGINT DEFAULT EXTRACT(EPOCH FROM NOW())*1000 NOT NULL
);
