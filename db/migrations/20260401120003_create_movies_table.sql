-- Movies: auditorium layout for a title (bookings reference movies.uuid)
CREATE TABLE movies (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    row_count INTEGER NOT NULL,
    seats_per_row INTEGER NOT NULL
);

CREATE INDEX movies_uuid_idx ON movies (uuid);
