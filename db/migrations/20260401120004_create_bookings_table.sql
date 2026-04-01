-- Bookings: seat reservation for a movie show, tied to a user
CREATE TABLE bookings (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,
    movie_slug TEXT NOT NULL REFERENCES movies (slug),
    seat_uuid TEXT NOT NULL,
    user_uuid TEXT NOT NULL REFERENCES users (uuid)
);

CREATE INDEX bookings_uuid_idx ON bookings (uuid);
