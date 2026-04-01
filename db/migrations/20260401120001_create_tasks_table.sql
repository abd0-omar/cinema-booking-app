-- Create tasks table with id (internal speed) and uuid (external APIs)
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL
);

CREATE INDEX tasks_uuid_idx ON tasks (uuid);
