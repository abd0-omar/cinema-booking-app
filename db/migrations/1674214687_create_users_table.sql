-- Create users table with id (internal speed) and uuid (external APIs)
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    token TEXT NOT NULL
);

CREATE INDEX users_uuid_idx ON users (uuid);
